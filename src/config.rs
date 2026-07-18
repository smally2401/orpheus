use slint::Color;
use std::path::PathBuf;
use crate::utils::expand_tilde;

pub struct Config {
    pub music_dir: String,
    pub playlists: Vec<PlaylistDef>,

    pub sidebar_bg: Color,
    pub now_playing_bar_bg: Color,
    pub library_view_bg: Color,
    pub album_view_bg: Color,
    pub playlists_view_bg: Color,
    pub open_playlist_view_bg: Color,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            music_dir: String::from("~/Music"),
            playlists: Vec::new(),

            sidebar_bg: Color::from_rgb_u8(0x0e, 0x10, 0x1d),
            now_playing_bar_bg: Color::from_rgb_u8(0x0a, 0x0a, 0x0f),
            library_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            album_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            playlists_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            open_playlist_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
        }
    }
}

pub struct PlaylistDef {
    pub name: String,
    pub songs: Vec<String>,
    pub sort: bool,
}

enum ConfigFile {
    Default,
    Custom(String),
}

const DEFAULT_CONFIG_FILE: &str = r##"music_dir = "~/Music"

sidebar_bg = "#0e101d"
now_playing_bar_bg = "#0a0a0f"
library_view_bg = "#1f1d2f"
album_view_bg = "#1f1d2f"
playlists_view_bg = "#1f1d2f"
open_playlist_view_bg = "#1f1d2f""##;

pub fn load_config() -> Config {
    let ConfigFile::Custom(file_contents) = load_config_file() else {
        return Config::default();
    };

    let music_dir = get_music_dir(&file_contents);

    load_lua(&file_contents, &music_dir)
}

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

fn get_music_dir(contents: &str) -> String {
    let lua = mlua::Lua::new();
    let _ = lua.load(contents).exec();
    let globals = lua.globals();
    get_music_dir_or_default(&globals, Config::default().music_dir)
}

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

    let sidebar_bg = get_color_or_default(&globals, "sidebar_bg", defaults.sidebar_bg);
    let now_playing_bar_bg =
        get_color_or_default(&globals, "now_playing_bar_bg", defaults.now_playing_bar_bg);
    let library_view_bg =
        get_color_or_default(&globals, "library_view_bg", defaults.library_view_bg);
    let album_view_bg = get_color_or_default(&globals, "album_view_bg", defaults.album_view_bg);
    let playlists_view_bg =
        get_color_or_default(&globals, "playlists_view_bg", defaults.playlists_view_bg);
    let open_playlist_view_bg = get_color_or_default(
        &globals,
        "open_playlist_view_bg",
        defaults.open_playlist_view_bg,
    );

    Config {
        music_dir: music_dir.to_string(),
        playlists,
        sidebar_bg,
        now_playing_bar_bg,
        library_view_bg,
        album_view_bg,
        playlists_view_bg,
        open_playlist_view_bg,
    }
}

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

        let songs: Vec<String> = songs_table
            .sequence_values::<String>()
            .filter_map(std::result::Result::ok)
            .collect();

        let sort: bool = entry_table.get("sort").unwrap_or(false);

        playlists.push(PlaylistDef { name, songs, sort });
    }

    playlists
}

fn register_list_music_files(lua: &mlua::Lua, music_dir: PathBuf) -> mlua::Result<()> {
    let func = lua.create_function(move |_, relative_dir: String| {
        let target_dir = music_dir.join(&relative_dir);
        let mut songs = Vec::new();

        for entry in walkdir::WalkDir::new(&target_dir).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() 
                && let Ok(rel) = entry.path().strip_prefix(&music_dir) {
                    songs.push(rel.to_string_lossy().into_owned());
            }
        }
        Ok(songs)
    })?;

    lua.globals().set("list_music_files", func)
}

fn get_music_dir_or_default(globals: &mlua::Table, default: String) -> String {
    match globals.get::<String>("music_dir") {
        Ok(value) => value,
        _ => default,
    }
}

fn get_color_or_default(globals: &mlua::Table, key: &str, default: Color) -> Color {
    match globals.get::<String>(key) {
        Ok(value) if is_valid_hex_color(&value) => hex_to_color(&value),
        _ => default,
    }
}

fn is_valid_hex_color(str: &str) -> bool {
    let stripped = str.strip_prefix("#").unwrap_or(str);
    stripped.len() == 6 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn hex_to_color(hex: &str) -> slint::Color {
    let stripped = hex.strip_prefix("#").unwrap_or(hex);
    let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
    slint::Color::from_rgb_u8(r, g, b)
}
