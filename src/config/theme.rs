//! The `theme` block of `config.lua`: three nested sub-tables (`bg`,
//! `text_color`, `text_size`), each keyed by UI region/element name. Every
//! lookup falls back independently to `Theme::default()` for that one
//! value, so a `config.lua` that only sets `theme.bg.sidebar` still gets
//! sensible defaults for everything else.

use slint::Color;
use strum::Display;
use strum::EnumString;

use crate::AppWindow;

#[derive(EnumString, Display, Debug)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum UiElement {
    Sidebar,
    NowPlayingBar,
    NowPlayingSong,
    NowPlayingArtist,
    LibraryView,
    AlbumView,
    PlaylistsView,
    OpenPlaylistView,
    DetailViewHeaderTitle,
    DetailViewHeaderSubtitle,
    LibraryListTitle,
    LibraryListSubtitle,
    AlbumListTitle,
    AlbumListSubtitle,
}

#[derive(Debug, Clone)]
pub(crate) enum UiProperty {
    Bg(Color),
    TextColor(Color),
    TextSize(i32),
}

pub(crate) fn set_property(ui: &AppWindow, element: UiElement, property: &UiProperty) {
    match property {
        UiProperty::Bg(color) => match element {
            UiElement::Sidebar => ui.set_sidebar_bg(*color),
            UiElement::NowPlayingBar => ui.set_now_playing_bar_bg(*color),
            UiElement::LibraryView => ui.set_library_view_bg(*color),
            UiElement::AlbumView => ui.set_album_view_bg(*color),
            UiElement::PlaylistsView => ui.set_playlists_view_bg(*color),
            UiElement::OpenPlaylistView => ui.set_open_playlist_view_bg(*color),
            other => eprintln!("{other} doesn't have a bg property"),
        },
        UiProperty::TextColor(color) => match element {
            UiElement::Sidebar => ui.set_sidebar_text_color(*color),
            UiElement::NowPlayingSong => ui.set_now_playing_song_text_color(*color),
            UiElement::NowPlayingArtist => ui.set_now_playing_artist_text_color(*color),
            UiElement::DetailViewHeaderTitle => ui.set_detail_view_header_title_text_color(*color),
            UiElement::DetailViewHeaderSubtitle => {
                ui.set_detail_view_header_subtitle_text_color(*color)
            }
            UiElement::LibraryListTitle => ui.set_library_list_title_text_color(*color),
            UiElement::LibraryListSubtitle => ui.set_library_list_subtitle_text_color(*color),
            UiElement::AlbumListTitle => ui.set_album_list_title_text_color(*color),
            UiElement::AlbumListSubtitle => ui.set_album_list_subtitle_text_color(*color),
            other => eprintln!("{other} doesn't have a text_color property"),
        },
        UiProperty::TextSize(size) => match element {
            UiElement::Sidebar => ui.set_sidebar_text_size(*size),
            UiElement::NowPlayingSong => ui.set_now_playing_song_text_size(*size),
            UiElement::NowPlayingArtist => ui.set_now_playing_artist_text_size(*size),
            UiElement::DetailViewHeaderTitle => ui.set_detail_view_header_title_text_size(*size),
            UiElement::DetailViewHeaderSubtitle => {
                ui.set_detail_view_header_subtitle_text_size(*size)
            }
            UiElement::LibraryListTitle => ui.set_library_list_title_text_size(*size),
            UiElement::LibraryListSubtitle => ui.set_library_list_subtitle_text_size(*size),
            UiElement::AlbumListTitle => ui.set_album_list_title_text_size(*size),
            UiElement::AlbumListSubtitle => ui.set_album_list_subtitle_text_size(*size),
            other => eprintln!("{other} doesn't have a text_size property"),
        },
    }
}

/// Background colors for every major UI region. These are applied in
/// `main.rs` via `apply_theme`.
pub(crate) struct Background {
    pub(crate) sidebar: Color,
    pub(crate) now_playing_bar: Color,
    pub(crate) library_view: Color,
    pub(crate) album_view: Color,
    pub(crate) playlists_view: Color,
    pub(crate) open_playlist_view: Color,
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

/// Text colors for every label/title/subtitle in the UI.
pub(crate) struct TextColor {
    pub(crate) sidebar: Color,
    pub(crate) now_playing_song: Color,
    pub(crate) now_playing_artist: Color,
    pub(crate) detail_view_header_title: Color,
    pub(crate) detail_view_header_subtitle: Color,
    pub(crate) library_list_title: Color,
    pub(crate) library_list_subtitle: Color,
    pub(crate) album_list_title: Color,
    pub(crate) album_list_subtitle: Color,
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

/// Font sizes for every text element in the UI.
#[derive(Clone, Copy)]
pub(crate) struct TextSize {
    pub(crate) sidebar: i32,
    pub(crate) now_playing_song: i32,
    pub(crate) now_playing_artist: i32,
    pub(crate) detail_view_header_title: i32,
    pub(crate) detail_view_header_subtitle: i32,
    pub(crate) library_list_title: i32,
    pub(crate) library_list_subtitle: i32,
    pub(crate) album_list_title: i32,
    pub(crate) album_list_subtitle: i32,
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
pub(crate) struct Theme {
    pub(crate) bg: Background,
    pub(crate) text_color: TextColor,
    pub(crate) text_size: TextSize,
}

/// Extract all background colors from Lua globals.
pub(super) fn load_backgrounds(globals: &mlua::Table, defaults: &Background) -> Background {
    let theme = get_table(globals, "theme");
    let bg = theme.as_ref().and_then(|t| get_table(t, "bg"));

    let sidebar = get_color_or_default(bg.as_ref(), "sidebar", defaults.sidebar);
    let now_playing_bar =
        get_color_or_default(bg.as_ref(), "now_playing_bar", defaults.now_playing_bar);
    let library_view = get_color_or_default(bg.as_ref(), "library_view", defaults.library_view);
    let album_view = get_color_or_default(bg.as_ref(), "album_view", defaults.album_view);
    let playlists_view =
        get_color_or_default(bg.as_ref(), "playlists_view", defaults.playlists_view);
    let open_playlist_view = get_color_or_default(
        bg.as_ref(),
        "open_playlist_view",
        defaults.open_playlist_view,
    );

    Background {
        sidebar,
        now_playing_bar,
        library_view,
        album_view,
        playlists_view,
        open_playlist_view,
    }
}

/// Extract all text colors from Lua globals.
pub(super) fn load_text_colors(globals: &mlua::Table, defaults: &TextColor) -> TextColor {
    let theme = get_table(globals, "theme");
    let text_color = theme.as_ref().and_then(|t| get_table(t, "text_color"));

    let sidebar = get_color_or_default(text_color.as_ref(), "sidebar", defaults.sidebar);
    let now_playing_song = get_color_or_default(
        text_color.as_ref(),
        "now_playing_song",
        defaults.now_playing_song,
    );
    let now_playing_artist = get_color_or_default(
        text_color.as_ref(),
        "now_playing_artist",
        defaults.now_playing_artist,
    );
    let detail_view_header_title = get_color_or_default(
        text_color.as_ref(),
        "detail_view_header_title",
        defaults.detail_view_header_title,
    );
    let detail_view_header_subtitle = get_color_or_default(
        text_color.as_ref(),
        "detail_view_header_subtitle",
        defaults.detail_view_header_subtitle,
    );
    let library_list_title = get_color_or_default(
        text_color.as_ref(),
        "library_list_title",
        defaults.library_list_title,
    );
    let library_list_subtitle = get_color_or_default(
        text_color.as_ref(),
        "library_list_subtitle",
        defaults.library_list_subtitle,
    );
    let album_list_title = get_color_or_default(
        text_color.as_ref(),
        "album_list_title",
        defaults.album_list_title,
    );
    let album_list_subtitle = get_color_or_default(
        text_color.as_ref(),
        "album_list_subtitle",
        defaults.album_list_subtitle,
    );

    TextColor {
        sidebar,
        now_playing_song,
        now_playing_artist,
        detail_view_header_title,
        detail_view_header_subtitle,
        library_list_title,
        library_list_subtitle,
        album_list_title,
        album_list_subtitle,
    }
}

/// Extract all text sizes from Lua globals.
pub(super) fn load_text_sizes(globals: &mlua::Table, defaults: &TextSize) -> TextSize {
    let theme = get_table(globals, "theme");
    let text_size = theme.as_ref().and_then(|t| get_table(t, "text_size"));

    let sidebar = get_size_or_default(text_size.as_ref(), "sidebar", defaults.sidebar);
    let now_playing_song = get_size_or_default(
        text_size.as_ref(),
        "now_playing_song",
        defaults.now_playing_song,
    );
    let now_playing_artist = get_size_or_default(
        text_size.as_ref(),
        "now_playing_artist",
        defaults.now_playing_artist,
    );
    let detail_view_header_title = get_size_or_default(
        text_size.as_ref(),
        "detail_view_header_title",
        defaults.detail_view_header_title,
    );
    let detail_view_header_subtitle = get_size_or_default(
        text_size.as_ref(),
        "detail_view_header_subtitle",
        defaults.detail_view_header_subtitle,
    );
    let library_list_title = get_size_or_default(
        text_size.as_ref(),
        "library_list_title",
        defaults.library_list_title,
    );
    let library_list_subtitle = get_size_or_default(
        text_size.as_ref(),
        "library_list_subtitle",
        defaults.library_list_subtitle,
    );
    let album_list_title = get_size_or_default(
        text_size.as_ref(),
        "album_list_title",
        defaults.album_list_title,
    );
    let album_list_subtitle = get_size_or_default(
        text_size.as_ref(),
        "album_list_subtitle",
        defaults.album_list_subtitle,
    );

    TextSize {
        sidebar,
        now_playing_song,
        now_playing_artist,
        detail_view_header_title,
        detail_view_header_subtitle,
        library_list_title,
        library_list_subtitle,
        album_list_title,
        album_list_subtitle,
    }
}

/// Reads an i32 from a (possibly missing) nested table, falling back to
/// `default` if the table is absent or the key is missing/not an int.
fn get_size_or_default(table: Option<&mlua::Table>, key: &str, default: i32) -> i32 {
    let Some(table) = table else {
        return default;
    };

    table.get::<i32>(key).unwrap_or(default)
}

/// Reads a hex color string from globals, falling back to `default` if the
/// key is missing or fails hex validation (`is_valid_hex_color`).
fn get_color_or_default(table: Option<&mlua::Table>, key: &str, default: Color) -> Color {
    let Some(table) = table else {
        return default;
    };

    match table.get::<String>(key) {
        Ok(value) if is_valid_hex_color(&value) => hex_to_color(&value),
        _ => default,
    }
}

/// True if `str` is a 6-digit hex color, with or without a leading `#`.
pub(crate) fn is_valid_hex_color(str: &str) -> bool {
    let stripped = str.strip_prefix("#").unwrap_or(str);
    stripped.len() == 6 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses a `#rrbbgg` (or `rrbbgg`) string into a Slint `Color`.
/// Malformed hex digits fall back to `0` for that channel rather than
/// errorring, since `is_valid_hex_color` should already have filtered out
/// anything that would fail here.
pub(crate) fn hex_to_color(hex: &str) -> slint::Color {
    let stripped = hex.strip_prefix("#").unwrap_or(hex);
    let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
    slint::Color::from_rgb_u8(r, g, b)
}

/// Looks up a nested table by key, returning `None` (rather than an
/// empty table) if it's missing or not a table. Callers treat `None` the
/// same way as an absent key and fall through to their own default.
fn get_table(table: &mlua::Table, key: &str) -> Option<mlua::Table> {
    table.get::<mlua::Table>(key).ok()
}
