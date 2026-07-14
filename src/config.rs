pub struct Config {
    pub sidebar_bg: String,
    pub now_playing_bar_bg: String,
    pub library_view_bg: String,
    pub album_view_bg: String,
    pub playlists_view_bg: String,
    pub open_playlist_view_bg: String,
}

impl Config {
    fn default() -> Config {
        Config {
            sidebar_bg: "#0e101d".to_string(),
            now_playing_bar_bg: "#0a0a0f".to_string(),
            library_view_bg: "#1f1d2f".to_string(),
            album_view_bg: "#1f1d2f".to_string(),
            playlists_view_bg: "#1f1d2f".to_string(),
            open_playlist_view_bg: "#1f1d2f".to_string(),
        }
    }
}

const DEFAULT_CONFIG_FILE: &str = r#######"
    sidebar_bg = "#0e101d"
    now_playing_bar_bg = "#0a0a0f"
    library_view_bg = "#1f1d2f"
    album_view_bg = "#1f1d2f"
    playlists_view_bg = "#1f1d2f"
    open_playlist_view_bg = "#1f1d2f"
"#######;

pub fn load_config() -> Config {

    
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("orpheus"),
        None => {
            eprintln!("Could not find config path");
            return Config::default();
        }
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

    let file_contents = match std::fs::read_to_string(config_path) {
        Ok(contents) => contents,
        Err(_) => {
            eprintln!("Could not read config file");
            return Config::default();
        }
    };

    load_lua(file_contents)
}

fn load_lua(contents: String) -> Config {
    let lua = mlua::Lua::new();
    lua.load(contents);
}
