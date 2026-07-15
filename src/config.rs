use slint::Color;

#[derive(Debug)]
pub struct Config {
    pub sidebar_bg: Color,
    pub now_playing_bar_bg: Color,
    pub library_view_bg: Color,
    pub album_view_bg: Color,
    pub playlists_view_bg: Color,
    pub open_playlist_view_bg: Color,
}

impl Config {
    fn default() -> Config {
        Config {
            sidebar_bg: Color::from_rgb_u8(0x0e, 0x10, 0x1d),
            now_playing_bar_bg: Color::from_rgb_u8(0x0a, 0x0a, 0x0f),
            library_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f),
            album_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f), 
            playlists_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f), 
            open_playlist_view_bg: Color::from_rgb_u8(0x1f, 0x1d, 0x2f), 
        }
    }
}

const DEFAULT_CONFIG_FILE: &str = r##"sidebar_bg = "#0e101d"
now_playing_bar_bg = "#0a0a0f"
library_view_bg = "#1f1d2f"
album_view_bg = "#1f1d2f"
playlists_view_bg = "#1f1d2f"
open_playlist_view_bg = "#1f1d2f"
"##;

pub fn load_config() -> Config {

    
    let Some(config_dir) = dirs::config_dir() else {
        eprintln!("Could not find config path");
        return Config::default();
    };

    if std::fs::create_dir_all(&config_dir).is_err() {
        eprintln!("Could not create config directory");
        return Config::default();
    }

    let config_path = config_dir.join("config.lua");
    if !config_path.exists() {
        if std::fs::write(config_path.as_path(), DEFAULT_CONFIG_FILE).is_err() {
            eprintln!("Could not create config file");
        }
        return Config::default();
    }

    let Ok(file_contents) = std::fs::read_to_string(config_path) else {
        eprintln!("Could not read config file");
        return Config::default();
    };

    load_lua(&file_contents)
}

fn load_lua(contents: &str) -> Config {
    let lua = mlua::Lua::new();
    if let Err(e) = lua.load(contents).exec() {
        eprintln!("Error running config.lua: {e}");
        return Config::default();
    }
    let globals = lua.globals();
    let defaults = Config::default();

    let sidebar_bg = get_color_or_default(&globals, "sidebar_bg", defaults.sidebar_bg);
    let now_playing_bar_bg = get_color_or_default(&globals, "now_playing_bar_bg", defaults.now_playing_bar_bg);
    let library_view_bg = get_color_or_default(&globals, "library_view_bg", defaults.library_view_bg);
    let album_view_bg = get_color_or_default(&globals, "album_view_bg", defaults.album_view_bg);
    let playlists_view_bg = get_color_or_default(&globals, "playlists_view_bg", defaults.playlists_view_bg);
    let open_playlist_view_bg = get_color_or_default(&globals, "open_playlist_view_bg", defaults.open_playlist_view_bg);

    Config {
        sidebar_bg,
        now_playing_bar_bg,
        library_view_bg,
        album_view_bg,
        playlists_view_bg,
        open_playlist_view_bg,
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
