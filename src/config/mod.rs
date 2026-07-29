//! Loads and parses `config.lua`, Orpheus's user configuration file.
//!
//! Config is a real Lua script, not static data. See the README in
//! `config_examples/` for the full user-facing explanation, including why
//! Lua was chosen and how `list_music_files` works. This file is the
//! implementation code of that: reading the file, running it in two passes
//! (see `load_config` and `load_lua`), and pulling typed values back out of
//! Lua's global table.

/// Keybinding parsing: `KeyCombo`, `KeyAction`, and the Lua-loading logic
/// that turns a `keymaps` table into a `HashMap<KeyCombo, KeyAction>`.
pub mod keys;
/// Playlist parsing: `PlaylistDef` and the Lua-loading logic that turns a
/// `playlists` table into a `Vec<PlaylistDef>`.
pub mod playlist;
/// Theme parsing: `Background`, `TextColor`, `TextSize`, `Theme`, and the
/// Lua-loading logic that turns a nested `theme` table into a `Theme`.
pub mod theme;

use crate::config::playlist::PlaylistDef;
use crate::config::playlist::get_playlists;
use crate::config::keys::KeyAction;
use crate::config::keys::KeyCombo;
use crate::config::keys::default_keymaps;
use crate::config::keys::load_keymaps;
use crate::config::theme::Theme;
use crate::config::theme::load_backgrounds;
use crate::config::theme::load_text_colors;
use crate::config::theme::load_text_sizes;
use crate::utils::expand_tilde;
use std::collections::HashMap;
use std::path::PathBuf;

/// Window geometry and state. `width` and `height` are `None` when the user
/// wants the window to use Slint's default (800x600) or when `maximized` is
/// true.
pub struct WindowState {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            maximized: true,
        }
    }
}

/// Fully resolved configuration, ready for the rest of the app to consume.
///
/// Every field has a sensible default (see `impl Default`), so a missing or
/// partially invalid `config.lua` never prevents the app from starting.
pub struct Config {
    pub music_dir: String,
    pub default_volume: f32,
    pub window_state: WindowState,
    pub keymaps: HashMap<KeyCombo, KeyAction>,
    pub playlists: Vec<PlaylistDef>,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            music_dir: String::from("~/Music"),
            default_volume: 1.0,
            window_state: WindowState::default(),
            keymaps: default_keymaps(),
            playlists: Vec::new(),
            theme: Theme::default(),
        }
    }
}

/// Result of locating (or creating) the user's config file.
enum ConfigFile {
    /// No usable config file was found or created: fall back to
    /// `Config::default()` entirely, skipping Lua altogether.
    Default,
    /// The file's raw contents, ready to be executed as Lua.
    Custom(String),
}

/// Default config written to disk on first run.
const DEFAULT_CONFIG_FILE: &str = r##"music_dir = "~/Music"
default_volume = 1.0
window_maximized = true

keymaps = {
    toggle_play = "Space",
    next_track = "N",
    prev_track = "P",
    volume_up = "UpArrow",
    volume_down = "DownArrow",
}

theme = {
    bg = {
        sidebar = "#0e101d",
        now_playing_bar = "#0a0a0f",
        library_view = "#1f1d2f",
        album_view = "#1f1d2f",
        playlists_view = "#1f1d2f",
        open_playlist_view = "#1f1d2f",
    },

    text_color = {
        sidebar = "#cdd6f4",
        now_playing_song = "#cdd6f4",
        now_playing_artist = "#cdd6f4",
        detail_view_header_title = "#cdd6f4",
        detail_view_header_subtitle = "#cdd6f4",
        library_list_title = "#cdd6f4",
        library_list_subtitle = "#cdd6f4",
        album_list_title = "#cdd6f4",
        album_list_subtitle = "#cdd6f4",
    },

    text_size = {
        sidebar = 18,
        now_playing_song = 15,
        now_playing_artist = 12,
        detail_view_header_title = 28,
        detail_view_header_subtitle = 18,
        library_list_title = 15,
        library_list_subtitle = 12,
        album_list_title = 15,
        album_list_subtitle = 12,
    },
}
"##;

/// Entry point: locates `config.lua`, then runs it to produce a `Config`.
///
/// Loading happens in two passes. `music_dir` is extracted first
/// (`get_music_dir`), then the script is run again from scratch
/// (`load_lua`) with the `list_music_files` function registered and bound
/// to that directory. This exists because `list_music_files` needs to know
/// `music_dir` before it can be registered, but `music_dir` itself only
/// becomes known by running the user's script. See `load_lua` for the
/// second half of this.
pub fn load_config() -> Config {
    let ConfigFile::Custom(file_contents) = load_config_file() else {
        return Config::default();
    };

    let music_dir = get_music_dir(&file_contents);

    load_lua(&file_contents, &music_dir)
}

/// Finds `~/.config/orpheus/config.lua`, creating it with default contents
/// on first run. Any failure along the way (no config dir, can't create
/// it, can't read it) falls back to `ConfigFile::Default` rather than
/// erroring out.
fn load_config_file() -> ConfigFile {
    let Some(config_dir) = dirs::config_dir() else {
        eprintln!("Could not find config path");
        return ConfigFile::Default;
    };

    let config_dir = config_dir.join("orpheus");
    if std::fs::create_dir_all(&config_dir).is_err() {
        eprintln!("Could not create config directory");
        return ConfigFile::Default;
    }

    let config_path = config_dir.join("config.lua");
    if !config_path.exists() {
        if std::fs::write(config_path.as_path(), DEFAULT_CONFIG_FILE).is_err() {
            eprintln!("Could not create config file");
        }
        return ConfigFile::Default;
    }

    let Ok(file_contents) = std::fs::read_to_string(config_path) else {
        eprintln!("Could not read config file");
        return ConfigFile::Default;
    };

    ConfigFile::Custom(file_contents)
}

/// First pass of loading: runs the script on a throwaway `Lua` instance
/// with no custom functions registered, purely to read back `music_dir`.
///
/// The script is expected to potentially error partway through this pass
/// (e.g. it may call `list_music_files`, which doesn't exist yet) so the
/// exec error is deliberately ignored. As long as `music_dir` was assigned
/// as a plan statement before that point, it's already sitting in globals
/// by the time the error happens.
fn get_music_dir(contents: &str) -> String {
    let lua = mlua::Lua::new();
    let _ = lua.load(contents).exec();
    let globals = lua.globals();
    get_music_dir_or_default(&globals, Config::default().music_dir)
}

/// Second, real pass: runs the script on a fresh `Lua` instance with
/// `list_music_files` registered (bound to the already-resolved
/// `music_dir`), then reads every config field back out of globals.
fn load_lua(contents: &str, music_dir: &str) -> Config {
    let lua = mlua::Lua::new();

    let expanded = expand_tilde(music_dir);
    if let Err(e) = register_list_music_files(&lua, expanded) {
        eprintln!("Could not register list_music_files: {e}");
    }

    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error running config.lua: {e}");
        return Config::default();
    }
    let globals = lua.globals();
    let defaults = Config::default();

    let default_volume = globals
        .get::<f32>("default_volume")
        .unwrap_or(defaults.default_volume);

    let window_state = load_window_state(&globals, &defaults.window_state);
    let keymaps = load_keymaps(&globals);
    let playlists = get_playlists(&globals);

    let bg = load_backgrounds(&globals, &defaults.theme.bg);
    let text_color = load_text_colors(&globals, &defaults.theme.text_color);
    let text_size = load_text_sizes(&globals, &defaults.theme.text_size);

    let theme = Theme {
        bg,
        text_color,
        text_size,
    };

    Config {
        music_dir: music_dir.to_string(),
        default_volume: default_volume.min(1.0),
        window_state,
        keymaps,
        playlists,
        theme,
    }
}

/// Extract window geometry from Lua globals. `width` and `height` are
/// optional, `maximized` defaults to true.
fn load_window_state(globals: &mlua::Table, defaults: &WindowState) -> WindowState {
    let width = globals.get::<f32>("window_width").ok();
    let height = globals.get::<f32>("window_height").ok();
    let maximized = match globals.get::<mlua::Value>("window_maximized") {
        Ok(mlua::Value::Boolean(b)) => b,
        _ => defaults.maximized,
    };

    WindowState {
        width,
        height,
        maximized,
    }
}

/// Registers `list_music_files(relative_dir) -> table of strings` as a Lua
/// global, backed by a recursive walk of `music_dir.join(relative_dir)`.
///
/// Returned paths are relative to `music_dir` again (via `strip_prefix`),
/// matching the format `PlaylistDef.songs` already expects, so scripts can
/// feed the result straight into a playlist's `songs` field.
fn register_list_music_files(lua: &mlua::Lua, music_dir: PathBuf) -> mlua::Result<()> {
    let func = lua.create_function(move |_, relative_dir: String| {
        let target_dir = music_dir.join(&relative_dir);
        let mut songs = Vec::new();

        for entry in walkdir::WalkDir::new(&target_dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file()
                && let Ok(rel) = entry.path().strip_prefix(&music_dir)
            {
                songs.push(rel.to_string_lossy().into_owned());
            }
        }
        Ok(songs)
    })?;

    lua.globals().set("list_music_files", func)
}

/// Reads `music_dir` from globals, falling back to `default` if it's
/// missing or not a string.
fn get_music_dir_or_default(globals: &mlua::Table, default: String) -> String {
    match globals.get::<String>("music_dir") {
        Ok(value) => value,
        _ => default,
    }
}
