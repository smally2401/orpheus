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

/// A playback event reported from `player_bridge`'s tick loop to the
/// dedicated script runtime thread (see `main.rs`), which turns each
/// variant into the matching `ScriptRuntime` call.
pub enum ScriptEvent {
    SongChanged(CurrentSong),
    SongHalfway,
}

/// Owns the `Lua` instance for the lifetime of the app, plus whatever
/// scripted hooks the user's `config.lua` registered. Constructed once
/// during `load_config`, then held by whatever part of the app tracks
/// playback state, so it can call `fire_song_change` when a track changes.
pub struct ScriptRuntime {
    lua: Lua,
    on_song_change: Option<mlua::RegistryKey>,
    on_song_halfway: Option<mlua::RegistryKey>,
}

impl ScriptRuntime {
    /// Wraps an already executed `Lua` instance into a `ScriptRuntime`,
    /// capturing `on_song_change` and `on_song_halfway` if the script
    /// defined them. Either, both, or neither may be present: each is
    /// independently optional.
    pub(super) fn new(lua: Lua) -> Self {
        let on_song_change = lua
            .globals()
            .get::<mlua::Function>("on_song_change")
            .ok()
            .and_then(|f| lua.create_registry_value(f).ok());

        let on_song_halfway = lua
            .globals()
            .get::<mlua::Function>("on_song_halfway")
            .ok()
            .and_then(|f| lua.create_registry_value(f).ok());

        Self {
            lua,
            on_song_change,
            on_song_halfway,
        }
    }

    /// Calls the user's `on_song_change(song)`, if one was registered.
    /// Errors from the script are logged and otherwise ignored, so a bug
    /// in someone's `config.lua` can't interrupt playback.
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

    /// Calls the user's `on_song_halfway()`, if one was registered. Takes
    /// no arguments, unlike `fire_song_change`, since by the time this
    /// fires the script already knows what's playing from the
    /// `on_song_change` call it received earlier for the same track.
    /// Errors from the script are logged and otherwise ignored, so a bug
    /// in someone's `config.lua` can't interrupt playback.
    pub fn fire_song_halfway(&self) {
        let Some(key) = &self.on_song_halfway else {
            return;
        };

        let Ok(func) = self.lua.registry_value::<mlua::Function>(key) else {
            return;
        };

        if let Err(e) = func.call::<()>(()) {
            eprintln!("Error in on_song_halfway: {e}");
        }
    }
}

/// A snapshot of the currently playing song, passed to `on_song_change`.
/// Deliberately a separate type from `local_backend::Song` (same
/// relationship as `PlaylistDef` to `Playlist`): only the fields a
/// script actually needs, decoupled frm playback/tag-reading internals.
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
    tx: tokio::sync::mpsc::Sender<PlayerCommand>,
) -> ScriptRuntime {
    let lua = unsafe { Lua::unsafe_new() };

    if let Err(e) = register_set_property(&lua, tx) {
        eprintln!("Could not register set_property: {e}");
    }

    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error building the script runtime: {e}");
    }

    ScriptRuntime::new(lua)
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
