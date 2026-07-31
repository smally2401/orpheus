//! The live runtime half of Orpheus's Lua integration: things that can't
//! be resolved once at config parse time because they depend on state
//! that only exists after playback starts (`current_song`, `on_song_change`).

use crate::local_backend::Song;
use mlua::Lua;
use std::sync::Arc;
use std::sync::Mutex;

/// Owns the `Lua` instance for the lifetime of the app, plus whatever
/// scripted hooks the user's `config.lua` registered. Constructed once
/// during `load_config`, then held by whatever part of the app tracks
/// playback state, so it can call `fire_song_change` when a track changes.
pub(crate) struct ScriptRuntime {
    lua: Lua,
    on_song_change: Option<mlua::RegistryKey>,
}

impl ScriptRuntime {
    /// Wraps an already executed `Lua` instance (see `load_lua` in
    /// `mod.rs`, which registers `current_song` and runs the script
    /// before calling this) into a `ScriptRuntime`, capturing
    /// `on_song_change` if the script defined one.
    pub(super) fn new(lua: Lua) -> Self {
        let on_song_change = lua
            .globals()
            .get::<mlua::Function>("on_song_change")
            .ok()
            .and_then(|f| lua.create_registry_value(f).ok());

        Self {
            lua,
            on_song_change,
        }
    }

    /// Calls the user's `on_song_change(song)`, if one was regustered.
    /// Errors from the script are logged and otherwise ignored, so a bug
    /// in someone's `config.lua` can't interrupt playback.
    pub(crate) fn fire_song_change(&self, song: CurrentSong) {
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
}

/// A snapshot of the currently playing song, as exposed to Lua via
/// `current_song()` and passed to `on_song_change`. Deliberately a
/// separate type from `local_backend::Song` (same relationship as
/// `PlaylistDef` to `Playlist`): only the fields a script actually needs,
/// decoupled from playback/tag-reading internals.
#[derive(Clone)]
pub(crate) struct CurrentSong {
    pub title: String,
    pub artist: String,
    pub album: String,
}

impl CurrentSong {
    /// Builds a `CurrentSong` from a real `Song`, called by
    /// `LocalBackend::load_track` on every track change.
    pub(crate) fn from_song(song: &Song) -> Self {
        Self {
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album_title.clone(),
        }
    }

    /// Converts to the Lua table shape `current_song()` and
    /// `on_song_change` hand to scripts: `{ title, artist, album }`.
    fn into_lua_table(self, lua: &Lua) -> mlua::Result<mlua::Table> {
        let table = lua.create_table()?;
        table.set("title", self.title)?;
        table.set("artist", self.artist)?;
        table.set("album", self.album)?;
        Ok(table)
    }
}

/// Registers `current_song() -> table?` as a Lua global, backed by
/// `current`. Returns `nil` when nothing's playing, so scripts can write
/// `if current_song() then ... end`.
///
/// Must be called before the config script is executed (same requirement
/// as `register_list_music_files` in `mod.rs`), in case the script calls
/// `current_song()` at the top level rather than only from inside a hook.
pub(super) fn register_current_song(
    lua: &Lua,
    current: Arc<Mutex<Option<CurrentSong>>>,
) -> mlua::Result<()> {
    let func = lua.create_function(move |lua, ()| {
        let guard = current.lock().unwrap();
        match &*guard {
            Some(song) => song.clone().into_lua_table(lua).map(mlua::Value::Table),
            None => Ok(mlua::Value::Nil),
        }
    })?;

    lua.globals().set("current_song", func)
}

/// Builds a fresh `ScriptRuntime` from `contents`, registering
/// `current_song` (backed by `current`) before running the script so a
/// top-level `current_song()` call works, same as `register_current_song`'s
/// requirement in `mod.rs`.
///
/// Must be called on whatever thread will own the resulting
/// `ScriptRuntime` for its entire lifetime: `mlua::Lua` is not `Send`, so
/// the `Lua` instance built here can never be moved to a different thread
/// afterward. See `main.rs`'s dedicated script runtime thread, which calls
/// this immediately after spawning, rather than receiving an already built
/// `ScriptRuntime` from elsewhere.
pub(crate) fn build_runtime(
    contents: &str,
    current: Arc<Mutex<Option<CurrentSong>>>,
) -> ScriptRuntime {
    let lua = Lua::new();

    if let Err(e) = register_current_song(&lua, current) {
        eprintln!("Could not register current_song: {e}");
    };

    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error building the script runtime: {e}");
    };

    ScriptRuntime::new(lua)
}
