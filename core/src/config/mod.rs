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
pub(crate) mod keys;
/// Playlist parsing: `PlaylistDef` and the Lua-loading logic that turns a
/// `playlists` table into a `Vec<PlaylistDef>`.
pub(crate) mod playlist;
/// The live runtime half of Lua integration: `on_song_change`,
/// `on_song_halfway`, and anything else that needs a `Lua` instance kept
/// alive past config parse time. See `scripting::ScriptRuntime`.
pub(crate) mod scripting;
/// Theme parsing: `Background`, `TextColor`, `TextSize`, `Theme`, and the
/// Lua-loading logic that turns a nested `theme` table into a `Theme`.
pub(crate) mod theme;

use crate::config::keys::KeyAction;
use crate::config::keys::KeyCombo;
use crate::config::keys::default_keymaps;
use crate::config::keys::load_keymaps;
use crate::config::playlist::PlaylistDef;
use crate::config::playlist::get_playlists;
use crate::config::theme::Theme;
use crate::config::theme::load_backgrounds;
use crate::config::theme::load_text_colors;
use crate::config::theme::load_text_sizes;
use crate::utils::expand_tilde;
use mlua::Lua;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// Window geometry and state. `width` and `height` are `None` when the user
/// wants the window to use Slint's default (800x600) or when `maximized` is
/// true.
pub(crate) struct WindowState {
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
pub(crate) struct Config {
    pub(crate) music_dir: String,
    pub(crate) default_volume: f32,
    pub(crate) window_state: WindowState,
    pub(crate) keymaps: HashMap<KeyCombo, KeyAction>,
    pub(crate) playlists: Vec<PlaylistDef>,
    pub(crate) theme: Theme,
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

/// Default config written to disk on first run. Sourced directly from
/// `config_examples/01_default.lua`, so the shipped default and the
/// documented example can never drift apart.
const DEFAULT_CONFIG_FILE: &str = include_str!("../../../config_examples/01_default.lua");

/// Type annotations for `lua-language-server`, giving editors autocomplete
/// and type-checking on `config.lua`. Sourced directly from
/// `config_examples/meta/orpheus.lua`. Written to
/// `~/.config/orpheus/meta/orpheus.lua` on every run, so it always matches
/// the schema this version of Orpheus actually reads. Not meant to be
/// hand-edited, see the config README for what it documents.
const ORPHEUS_LUA_META: &str = include_str!("../../../config_examples/meta/orpheus.lua");

/// `lua_language_server` workspace config, pointing it at `ORPHEUS_LUA_META`
/// and declaring every config global so it isn't flagged as undefined.
/// Sourced directly from `config_examples/.luarc.json` Written to
/// `~/.config/orpheus/.luarc.json` only if it doesn't already exist, since
/// (unlike `orpheus.lua`) users may reasonably extend this with their own
/// settings.
const DEFAULT_LUARC_JSON: &str = include_str!("../../../config_examples/.luarc.json");

/// Entry point: locates `config.lua`, then runs it to produce a `Config`.
///
/// Loading happens in two passes. `music_dir` is extracted first
/// (`get_music_dir`), then the script is run again from scratch
/// (`load_lua`) with the `list_music_files` function registered and bound
/// to that directory. This exists because `list_music_files` needs to know
/// `music_dir` before it can be registered, but `music_dir` itself only
/// becomes known by running the user's script. See `load_lua` for the
/// second half of this.
pub(crate) fn load_config() -> (Config, String) {
    let ConfigFile::Custom(file_contents) = load_config_file() else {
        return (Config::default(), String::new());
    };

    let music_dir = get_music_dir(&file_contents);

    (load_lua(&file_contents, &music_dir), file_contents)
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

    write_lsp_support_files(&config_dir);

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

/// Writes the `lua-language-server` support files (`meta/orpheus.lua` and
/// `.luarc.json`) into `config_dir`, giving editors autocomplete and type
/// diagnostics on `config.lua`. `orpheus.lua` is rewritten every run to
/// stay in sync with this version's schema. `.luarc.json` is only written
/// if missing, since users may customize it. Failures are logged and
/// otherwise ignored: this is tooling support, not required for Orpheus
/// to function.
fn write_lsp_support_files(config_dir: &Path) {
    let meta_dir = config_dir.join("meta");
    if std::fs::create_dir_all(&meta_dir).is_err() {
        eprintln!("Could not create meta directory");
        return;
    }

    let meta_path = meta_dir.join("orpheus.lua");
    if std::fs::write(&meta_path, ORPHEUS_LUA_META).is_err() {
        eprintln!("Could not write orpheus.lua meta file");
        return;
    }

    let luarc_path = config_dir.join(".luarc.json");
    if !luarc_path.exists() && std::fs::write(&luarc_path, DEFAULT_LUARC_JSON).is_err() {
        eprintln!("Could not create .luarc.json");
    }
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
    let lua = Lua::new();
    let _ = lua.load(contents).exec();
    let globals = lua.globals();
    get_music_dir_or_default(&globals, Config::default().music_dir)
}

/// Second, real pass: runs the script on a fresh `Lua` instance with
/// `list_music_files` registered (bound to the already-resolved
/// `music_dir`), then reads every config field back out of globals.
fn load_lua(contents: &str, music_dir: &str) -> Config {
    let lua = Lua::new();

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
        .unwrap_or(defaults.default_volume)
        .clamp(0.0, 1.0);

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
        default_volume,
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
fn register_list_music_files(lua: &Lua, music_dir: PathBuf) -> mlua::Result<()> {
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
