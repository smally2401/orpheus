//! Loads and parses `config.lua`, Orpheus's user configuration file.
//!
//! Config is a real Lua script, not static data. See the README in
//! `config_examples/` for the full user-facing explanation, including why
//! Lua was chosen and how `list_music_files` works. This file is the
//! implementation code of that: reading the file, running it in two passes
//! (see `load_config` and `load_lua`), and pulling typed values back out of
//! Lua's global table.

use crate::utils::expand_tilde;
use slint::Color;
use std::path::PathBuf;

pub struct Background {
    pub sidebar: Color,
    pub now_playing_bar: Color,
    pub library_view: Color,
    pub album_view: Color,
    pub playlists_view: Color,
    pub open_playlist_view: Color,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            sidebar: Color::from_rgb_u8(0x0e, 0x10, 0x1d),
            now_playing_bar: Color::from_rgb_u8(0x0a, 0x0a, 0x0f),
            library_view: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            album_view: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            playlists_view: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            open_playlist_view: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
        }
    }
}

pub struct TextColor {
    pub sidebar: Color,
    pub now_playing_song: Color,
    pub now_playing_artist: Color,
    pub detail_view_header_title: Color,
    pub detail_view_header_subtitle: Color,
    pub library_list_title: Color,
    pub library_list_subtitle: Color,
    pub album_list_title: Color,
    pub album_list_subtitle: Color,
}

impl Default for TextColor {
    fn default() -> Self {
        Self {
            sidebar: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            now_playing_song: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            now_playing_artist: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            detail_view_header_title: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            detail_view_header_subtitle: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            library_list_title: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            library_list_subtitle: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            album_list_title: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
            album_list_subtitle: Color::from_rgb_u8(0xcd, 0xd6, 0xf4),
        }
    }
}

pub struct TextSize {
    pub sidebar: u32,
    pub now_playing_song: u32,
    pub now_playing_artist: u32,
    pub detail_view_header_title: u32,
    pub detail_view_header_subtitle: u32,
    pub library_list_title: u32,
    pub library_list_subtitle: u32,
    pub album_list_title: u32,
    pub album_list_subtitle: u32,
}

impl Default for TextSize {
    fn default() -> Self {
        Self {
            sidebar: 18,
            now_playing_song: 15,
            now_playing_artist: 12,
            detail_view_header_title: 28,
            detail_view_header_subtitle: 18,
            library_list_title: 15,
            library_list_subtitle: 12,
            album_list_title: 15,
            album_list_subtitle: 12,
        }
    }
}

/// The UI color palette.
#[derive(Default)]
pub struct Theme {
    pub bg: Background,
    pub text_color: TextColor,
    pub text_size: TextSize,
}

/// Fully resolved configuration, ready for the rest of the app to consume.
///
/// Every field has a sensible default (see `impl Default`), so a missing or
/// partially invalid `config.lua` never prevents the app from starting.
pub struct Config {
    pub music_dir: String,
    pub playlists: Vec<PlaylistDef>,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            music_dir: String::from("~/Music"),
            playlists: Vec::new(),
            theme: Theme::default(),
        }
    }
}

/// A single playlist as declared in `config.lua`, before its song paths
/// have been resolved against `music_dir` or matched against the library.
/// See `local_backend::build_playlists` for that resolution step.
pub struct PlaylistDef {
    pub name: String,
    pub songs: Vec<String>,
    /// If true, songs are re-sorted by artist/album/track number when
    /// resolved, rather than kept in the order listed. Intended for
    /// playlists generated with `list_music_files`, where listed order
    /// is just filesystem walk order.
    pub sort: bool,
    /// Optional path to a custom cover image for this playlist, relative
    /// to `music_dir`. If set, the image is loaded and displayed as the
    /// playlist's cover art. Falls back to the bundled placeholder if the
    /// path is missing or the image fails to load.
    pub art: Option<String>,
}

/// Result of locating (or creating) the user's config file.
enum ConfigFile {
    /// No usable config file was found or created: fall back to
    /// `Config::default()` entirely, skipping Lua altogether.
    Default,
    /// The file's raw contents, ready to be executed as Lua.
    Custom(String),
}

const DEFAULT_CONFIG_FILE: &str = r##"music_dir = "~/Music"

sidebar_bg = "#0e101d"
now_playing_bar_bg = "#0a0a0f"
library_view_bg = "#1f1d2f"
album_view_bg = "#1f1d2f"
playlists_view_bg = "#1f1d2f"
open_playlist_view_bg = "#1f1d2f"

sidebar_text_color = "#cdd6f4"
now_playing_song_text_color = "#cdd6f4"
now_playing_artist_text_color = "#cdd6f4"
detail_view_header_title_text_color = "#cdd6f4"
detail_view_header_subtitle_text_color = "#cdd6f4"
library_list_title_text_color = "#cdd6f4"
library_list_subtitle_text_color = "#cdd6f4"
album_list_title_text_color = "#cdd6f4"
album_list_subtitle_text_color = "#cdd6f4""##;

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

/// Firts pass of loading: runs the script on a throwaway `Lua` instance
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
        playlists,
        theme,
    }
}

fn load_backgrounds(globals: &mlua::Table, defaults: &Background) -> Background {
    let sidebar_bg = get_color_or_default(globals, "sidebar_bg", defaults.sidebar);
    let now_playing_bar_bg =
        get_color_or_default(globals, "now_playing_bar_bg", defaults.now_playing_bar);
    let library_view_bg = get_color_or_default(globals, "library_view_bg", defaults.library_view);
    let album_view_bg = get_color_or_default(globals, "album_view_bg", defaults.album_view);
    let playlists_view_bg =
        get_color_or_default(globals, "playlists_view_bg", defaults.playlists_view);
    let open_playlist_view_bg = get_color_or_default(
        globals,
        "open_playlist_view_bg",
        defaults.open_playlist_view,
    );

    Background {
        sidebar: sidebar_bg,
        now_playing_bar: now_playing_bar_bg,
        library_view: library_view_bg,
        album_view: album_view_bg,
        playlists_view: playlists_view_bg,
        open_playlist_view: open_playlist_view_bg,
    }
}

fn load_text_colors(globals: &mlua::Table, defaults: &TextColor) -> TextColor {
    let sidebar_text_color = get_color_or_default(globals, "sidebar_text_color", defaults.sidebar);
    let now_playing_song_text_color = get_color_or_default(
        globals,
        "now_playing_song_text_color",
        defaults.now_playing_song,
    );
    let now_playing_artist_text_color = get_color_or_default(
        globals,
        "now_playing_artist_text_color",
        defaults.now_playing_artist,
    );
    let detail_view_header_title_text_color = get_color_or_default(
        globals,
        "detail_view_header_title_text_color",
        defaults.detail_view_header_title,
    );
    let detail_view_header_subtitle_text_color = get_color_or_default(
        globals,
        "detail_view_header_subtitle_text_color",
        defaults.detail_view_header_subtitle,
    );
    let library_list_title_text_color = get_color_or_default(
        globals,
        "library_list_title_text_color",
        defaults.library_list_title,
    );
    let library_list_subtitle_text_color = get_color_or_default(
        globals,
        "library_list_subtitle_text_color",
        defaults.library_list_subtitle,
    );
    let album_list_title_text_color = get_color_or_default(
        globals,
        "album_list_title_text_color",
        defaults.album_list_title,
    );
    let album_list_subtitle_text_color = get_color_or_default(
        globals,
        "album_list_subtitle_text_color",
        defaults.album_list_subtitle,
    );

    TextColor {
        sidebar: sidebar_text_color,
        now_playing_song: now_playing_song_text_color,
        now_playing_artist: now_playing_artist_text_color,
        detail_view_header_title: detail_view_header_title_text_color,
        detail_view_header_subtitle: detail_view_header_subtitle_text_color,
        library_list_title: library_list_title_text_color,
        library_list_subtitle: library_list_subtitle_text_color,
        album_list_title: album_list_title_text_color,
        album_list_subtitle: album_list_subtitle_text_color,
    }
}

fn load_text_sizes(globals: &mlua::Table, defaults: &TextSize) -> TextSize {
    let sidebar_text_size = globals
        .get::<u32>("sidebar_text_size")
        .unwrap_or(defaults.sidebar);
    let now_playing_song_text_size = globals
        .get::<u32>("now_playing_song_text_size")
        .unwrap_or(defaults.now_playing_song);
    let now_playing_artist_text_size = globals
        .get::<u32>("now_playing_artist_text_size")
        .unwrap_or(defaults.now_playing_artist);
    let detail_view_header_title_text_size = globals
        .get::<u32>("detail_view_header_title_text_size")
        .unwrap_or(defaults.detail_view_header_title);
    let detail_view_header_subtitle_text_size = globals
        .get::<u32>("detail_view_header_subtitle_text_size")
        .unwrap_or(defaults.detail_view_header_subtitle);
    let library_list_title_text_size = globals
        .get::<u32>("library_list_title_text_size")
        .unwrap_or(defaults.library_list_title);
    let library_list_subtitle_text_size = globals
        .get::<u32>("library_list_subtitle_text_size")
        .unwrap_or(defaults.library_list_subtitle);
    let album_list_title_text_size = globals
        .get::<u32>("album_list_title_text_size")
        .unwrap_or(defaults.album_list_title);
    let album_list_subtitle_text_size = globals
        .get::<u32>("album_list_subtitle_text_size")
        .unwrap_or(defaults.album_list_subtitle);

    TextSize {
        sidebar: sidebar_text_size,
        now_playing_song: now_playing_song_text_size,
        now_playing_artist: now_playing_artist_text_size,
        detail_view_header_title: detail_view_header_title_text_size,
        detail_view_header_subtitle: detail_view_header_subtitle_text_size,
        library_list_title: library_list_title_text_size,
        library_list_subtitle: library_list_subtitle_text_size,
        album_list_title: album_list_title_text_size,
        album_list_subtitle: album_list_subtitle_text_size,
    }
}

/// Reads the `playlists` global, a Lua array of `{ name, songs, sort }`
/// tables, into `PlaylistDef`s.
///
/// Missing or malformed entries are skipped individually rather than
/// failing the whole config load. Each song string that fails to
/// convert is also dropped silently.
fn get_playlists(globals: &mlua::Table) -> Vec<PlaylistDef> {
    let playlists_table: mlua::Table = match globals.get("playlists") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut playlists = Vec::new();

    for entry in playlists_table.sequence_values::<mlua::Table>() {
        let entry_table: mlua::Table = match entry {
            Ok(t) => t,
            Err(_) => continue,
        };

        let name: String = match entry_table.get("name") {
            Ok(n) => n,
            Err(_) => continue,
        };

        let songs_table: mlua::Table = match entry_table.get("songs") {
            Ok(t) => t,
            Err(_) => continue,
        };

        let art: Option<String> = entry_table.get("art").ok();

        let songs: Vec<String> = songs_table
            .sequence_values::<String>()
            .filter_map(std::result::Result::ok)
            .collect();

        let sort: bool = entry_table.get("sort").unwrap_or(false);

        playlists.push(PlaylistDef {
            name,
            songs,
            sort,
            art,
        });
    }

    playlists
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

/// Reads a hex color string from globals, falling back to `default` if the
/// key is missing or fails hex validation (`is_valid_hex_color`).
fn get_color_or_default(globals: &mlua::Table, key: &str, default: Color) -> Color {
    match globals.get::<String>(key) {
        Ok(value) if is_valid_hex_color(&value) => hex_to_color(&value),
        _ => default,
    }
}

/// True if `str` is a 6-digit hex color, with or without a leading `#`.
fn is_valid_hex_color(str: &str) -> bool {
    let stripped = str.strip_prefix("#").unwrap_or(str);
    stripped.len() == 6 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses a `#rrbbgg` (or `rrbbgg`) string into a Slint `Color`.
/// Malformed hex digits fall back to `0` for that channel rather than
/// errorring, since `is_valid_hex_color` should already have filtered out
/// anything that would fail here.
fn hex_to_color(hex: &str) -> slint::Color {
    let stripped = hex.strip_prefix("#").unwrap_or(hex);
    let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
    slint::Color::from_rgb_u8(r, g, b)
}
