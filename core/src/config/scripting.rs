//! The live runtime half of Orpheus's Lua integration: things that can't
//! be resolved once at config parse time because they depend on state
//! that only exists after playback starts (`on_song_change`,
//! `on_song_halfway`).

use crate::config::theme::UiProperty;
use crate::config::theme::hex_to_color;
use crate::config::theme::is_valid_hex_color;
use crate::local_backend::Song;
use crate::player_bridge::PlayerCommand;
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

/// Live playback state, updated every tick by `player_bridge`'s
/// `TickState::on_tick` and read syncrhonously by `player.get_state()`.
#[derive(Clone)]
pub struct PlaybackState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub position: u64,
    pub duration: u64,
    pub paused: bool,
}

pub type PlaybackStateHandle = Arc<Mutex<Option<PlaybackState>>>;

impl PlaybackState {
    fn into_lua_table(self, lua: &Lua) -> mlua::Result<mlua::Table> {
        let table = lua.create_table()?;
        table.set("title", self.title)?;
        table.set("artist", self.artist)?;
        table.set("album", self.album)?;
        table.set("position", self.position)?;
        table.set("duration", self.duration)?;
        table.set("paused", self.paused)?;
        Ok(table)
    }
}

/// A single pending or repeating timer, registered via `defer`/`every`.
pub(super) struct Timer {
    deadline: Instant,
    /// `Some(interval)` for `evert`, re-armed after firing. `None` for
    /// `defer`, removed after firing once.
    interval: Option<Duration>,
    callback: mlua::RegistryKey,
}

/// Shared, script-thread-only queue of pending timers
type TimerQueue = Rc<RefCell<Vec<Timer>>>;

/// A playback event reported from `player_bridge`'s tick loop to the
/// dedicated script runtime thread (see `main.rs`), which turns each
/// variant into the matching `ScriptRuntime` call.
pub enum ScriptEvent {
    SongChanged(CurrentSong),
}

/// Owns the `Lua` instance for the lifetime of the app, plus whatever
/// scripted hooks the user's `config.lua` registered. Constructed once
/// during `load_config`, then held by whatever part of the app tracks
/// playback state, so it can call `fire_song_change` when a track changes.
pub struct ScriptRuntime {
    lua: Lua,
    on_song_change: Option<mlua::RegistryKey>,
    timers: TimerQueue,
}

impl ScriptRuntime {
    /// Wraps an already executed `Lua` instance into a `ScriptRuntime`,
    /// capturing `on_song_change` and `on_song_halfway` if the script
    /// defined them. Either, both, or neither may be present: each is
    /// independently optional.
    pub(super) fn new(lua: Lua, timers: TimerQueue) -> Self {
        let on_song_change = lua
            .globals()
            .get::<mlua::Function>("on_song_change")
            .ok()
            .and_then(|f| lua.create_registry_value(f).ok());

        Self {
            lua,
            on_song_change,
            timers,
        }
    }

    /// Calls the user's `on_song_change(song)`, if one was registered, and
    /// updates the cached `current_song` so `player.current_song()` reflects
    /// the new track for any script code that runs afterward (including
    /// from unrelated callbacks, not just this hook).
    pub fn fire_song_change(&self, song: CurrentSong) {
        let Some(key) = &self.on_song_change else {
            return;
        };

        let Ok(func) = self.lua.registry_value::<mlua::Function>(key) else {
            return;
        };

        let Ok(table) = song.into_lua_table(&self.lua) else {
            eprintln!("Could not build Lua table for on_song_change");
            return;
        };

        if let Err(e) = func.call::<()>(table) {
            eprintln!("Error in on_song_change: {e}");
        }
    }

    /// The `Instant` of the soonest pending timer, if any. Used by the
    /// script thread's main loop to compute how long to block on
    /// `recv_timeout` before it needs to wake up and check for due timers.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.timers.borrow().iter().map(|t| t.deadline).min()
    }

    /// Fires every timer whose deadline has passed, re-arming `every`
    /// timers for their next interval. Errors from a timer callback are
    /// logged and otherwise ignored, same as the other hooks.
    pub fn fire_due_timers(&self) {
        let now = Instant::now();

        let (due, still_pending): (Vec<_>, Vec<_>) = self
            .timers
            .borrow_mut()
            .drain(..)
            .partition(|t| t.deadline <= now);
        *self.timers.borrow_mut() = still_pending;

        for timer in due {
            if let Ok(func) = self.lua.registry_value::<mlua::Function>(&timer.callback)
                && let Err(e) = func.call::<()>(())
            {
                eprintln!("Error in deferred callback: {e}");
            }

            if let Some(interval) = timer.interval {
                self.timers.borrow_mut().push(Timer {
                    deadline: now + interval,
                    interval: Some(interval),
                    callback: timer.callback,
                });
            } else {
                let _ = self.lua.remove_registry_value(timer.callback);
            }
        }
    }
}

/// A snapshot of the currently playing song, passed to `on_song_change`.
/// Deliberately a separate type from `local_backend::Song` (same
/// relationship as `PlaylistDef` to `Playlist`): only the fields a
/// script actually needs, decoupled from playback/tag-reading internals.
#[derive(Clone)]
pub struct CurrentSong {
    pub title: String,
    pub artist: String,
    pub album: String,
}

impl From<&Song> for CurrentSong {
    /// Builds a `CurrentSong` from a real `Song`, called by
    /// `LocalBackend::load_track` on every track change.
    fn from(song: &Song) -> Self {
        Self {
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album_title.clone(),
        }
    }
}

impl CurrentSong {
    /// Converts to the Lua table shape passed to `on_song_change`:
    /// `{ title, artist, album }`.
    fn into_lua_table(self, lua: &Lua) -> mlua::Result<mlua::Table> {
        let table = lua.create_table()?;
        table.set("title", self.title)?;
        table.set("artist", self.artist)?;
        table.set("album", self.album)?;
        Ok(table)
    }
}

/// Builds a fresh `ScriptRuntime` from `contents`.
///
/// Must be called on whatever thread will own the resulting
/// `ScriptRuntime` for its entire lifetime: `mlua::Lua` is not `Send`, so
/// the `Lua` instance built here can never be moved to a different thread
/// afterward. See `main.rs`'s dedicated script runtime thread, which calls
/// this immediately after spawning, rather than receiving an already built
/// `ScriptRuntime` from elsewhere.
pub fn build_runtime(
    contents: &str,
    tx: &tokio::sync::mpsc::Sender<PlayerCommand>,
    playback_state: PlaybackStateHandle,
) -> ScriptRuntime {
    let lua = unsafe { Lua::unsafe_new() };
    let timers: TimerQueue = Rc::new(RefCell::new(Vec::new()));

    if let Err(e) = register_set_property(&lua, tx.clone()) {
        eprintln!("Could not register set_property: {e}");
    }

    if let Err(e) = register_playback_commands(&lua, tx, playback_state) {
        eprintln!("Could not register playback commands: {e}");
    }

    if let Err(e) = register_timers(&lua, timers.clone()) {
        eprintln!("Could not register timers: {e}");
    }

    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error building the script runtime: {e}");
    }

    if let Ok(on_startup) = lua.globals().get::<mlua::Function>("on_startup")
        && let Err(e) = on_startup.call::<()>(())
    {
        eprintln!("Error in on_startup: {e}");
    }

    ScriptRuntime::new(lua, timers)
}

/// Registers `set_property(element, property, value) -> nil` as a Lua
/// global, letting scripts change UI colors/text sizes at runtime (e.g.
/// from `on_song_change`). `property` must be `"bg"`, `"text_color"`, or
/// `"text_size"`, `value` must be a hex color string for the first two,
/// or a number for the third. Malformed calls (unknown element, unknown
/// property, wrong value type, invalid hex string) are logged and
/// ignored rather than erroring, so a mistake here can't crash a script.
fn register_set_property(
    lua: &Lua,
    tx: tokio::sync::mpsc::Sender<PlayerCommand>,
) -> mlua::Result<()> {
    let func = lua.create_function(
        move |_, (element, property, value): (String, String, mlua::Value)| {
            let property = match property.as_str() {
                "bg" | "text_color" => {
                    let mlua::Value::String(s) = value else {
                        eprintln!("{property} expects a string, got {value:?}");
                        return Ok(());
                    };
                    let Ok(s) = s.to_str() else {
                        eprintln!("{property} value was not valid UTF-8");
                        return Ok(());
                    };
                    if !is_valid_hex_color(&s) {
                        eprintln!("{s} is not a valid hex color");
                        return Ok(());
                    }
                    let color = hex_to_color(&s);
                    if property == "bg" {
                        UiProperty::Bg(color)
                    } else {
                        UiProperty::TextColor(color)
                    }
                }

                "text_size" => {
                    let size = match value {
                        mlua::Value::Integer(i) => i as i32,
                        mlua::Value::Number(n) => n as i32,
                        _ => {
                            eprintln!("text_size expects a number, got {value:?}");
                            return Ok(());
                        }
                    };
                    UiProperty::TextSize(size)
                }

                other => {
                    eprintln!("{other} is not a valid property");
                    return Ok(());
                }
            };

            let _ = tx.try_send(PlayerCommand::SetProperty(element, property));
            Ok(())
        },
    )?;

    lua.globals().set("set_property", func)
}

/// Registers the `player` table as a Lua global, giving scripts direct
/// playback control (`player.play()`, `player.next_track()`, etc.).
fn register_playback_commands(
    lua: &Lua,
    tx: &tokio::sync::mpsc::Sender<PlayerCommand>,
    playback_state: PlaybackStateHandle,
) -> mlua::Result<()> {
    use crate::player_bridge::PlayerCommand::*;

    let player_table = lua.create_table()?;

    let tx_clone = tx.clone();
    let toggle_play_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(TogglePlay);
        Ok(())
    })?;
    player_table.set("toggle_play", toggle_play_func)?;

    let tx_clone = tx.clone();
    let play_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(Play);
        Ok(())
    })?;
    player_table.set("play", play_func)?;

    let tx_clone = tx.clone();
    let pause_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(Pause);
        Ok(())
    })?;
    player_table.set("pause", pause_func)?;

    let tx_clone = tx.clone();
    let next_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(NextTrack);
        Ok(())
    })?;
    player_table.set("next_track", next_func)?;

    let tx_clone = tx.clone();
    let prev_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(PrevTrack);
        Ok(())
    })?;
    player_table.set("prev_track", prev_func)?;

    let tx_clone = tx.clone();
    let seek_func = lua.create_function(move |_, pos: usize| {
        let _ = tx_clone.try_send(SetPosition(pos));
        Ok(())
    })?;
    player_table.set("seek", seek_func)?;

    let tx_clone = tx.clone();
    let seek_by_func = lua.create_function(move |_, secs: i64| {
        let _ = tx_clone.try_send(SeekBy(secs));
        Ok(())
    })?;
    player_table.set("seek_by", seek_by_func)?;

    let tx_clone = tx.clone();
    let vol_func = lua.create_function(move |_, vol: f32| {
        let _ = tx_clone.try_send(SetVolume(vol));
        Ok(())
    })?;
    player_table.set("set_volume", vol_func)?;

    let tx_clone = tx.clone();
    let repeat_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(ToggleRepeat);
        Ok(())
    })?;
    player_table.set("toggle_repeat", repeat_func)?;

    let tx_clone = tx.clone();
    let shuffle_func = lua.create_function(move |_, ()| {
        let _ = tx_clone.try_send(ToggleShuffle);
        Ok(())
    })?;
    player_table.set("toggle_shuffle", shuffle_func)?;

    let get_state_func =
        lua.create_function(
            move |lua, ()| match playback_state.lock().unwrap().clone() {
                Some(state) => state.into_lua_table(lua).map(mlua::Value::Table),
                None => Ok(mlua::Value::Nil),
            },
        )?;
    player_table.set("get_state", get_state_func)?;

    lua.globals().set("player", player_table)
}

/// Registers the `defer` and `every` functions.
fn register_timers(lua: &Lua, timers: TimerQueue) -> mlua::Result<()> {
    let timers_clone = timers.clone();
    let defer_func = lua.create_function(move |lua, (secs, func): (f64, mlua::Function)| {
        let key = lua.create_registry_value(func)?;
        timers_clone.borrow_mut().push(Timer {
            deadline: Instant::now() + Duration::from_secs_f64(secs),
            interval: None,
            callback: key,
        });
        Ok(())
    })?;
    lua.globals().set("defer", defer_func)?;

    let every_func = lua.create_function(move |lua, (secs, func): (f64, mlua::Function)| {
        let key = lua.create_registry_value(func)?;
        let interval = Duration::from_secs_f64(secs);
        timers.borrow_mut().push(Timer {
            deadline: Instant::now() + interval,
            interval: Some(interval),
            callback: key,
        });
        Ok(())
    })?;
    lua.globals().set("every", every_func)?;

    Ok(())
}
