//! The live runtime half of Orpheus's Lua integration: things that can't
//! be resolved once at config parse time because they depend on state
//! that only exists after playback starts (`on_song_change`,
//! `on_song_halfway`).

use crate::local_backend::Song;
use mlua::Lua;

/// A playback event reported from `player_bridge`'s tick loop to the
/// dedicated script runtime thread (see `main.rs`), which turns each
/// variant into the matching `ScriptRuntime` call.
pub(crate) enum ScriptEvent {
    SongChanged(CurrentSong),
    SongHalfway,
}

/// Owns the `Lua` instance for the lifetime of the app, plus whatever
/// scripted hooks the user's `config.lua` registered. Constructed once
/// during `load_config`, then held by whatever part of the app tracks
/// playback state, so it can call `fire_song_change` when a track changes.
pub(crate) struct ScriptRuntime {
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

    /// Calls the user's `on_song_halfway()`, if one was registered. Takes
    /// no arguments, unlike `fire_song_change`, since by the time this
    /// fires the script already knows what's playing from the
    /// `on_song_change` call it received earlier for the same track.
    /// Errors from the script are logged and otherwise ignored, so a bug
    /// in someone's `config.lua` can't interrupt playback.
    pub(crate) fn fire_song_halfway(&self) {
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
pub(crate) fn build_runtime(contents: &str) -> ScriptRuntime {
    let lua = unsafe { Lua::unsafe_new() };

    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error building the script runtime: {e}");
    }

    ScriptRuntime::new(lua)
}
