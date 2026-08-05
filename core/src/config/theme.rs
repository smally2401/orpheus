//! The `theme` block of `config.lua`: three nested sub-tables (`bg`,
//! `text_color`, `text_size`), each keyed by UI region/element name. Every
//! lookup falls back independently to `Theme::default()` for that one
//! value, so a `config.lua` that only sets `theme.bg.sidebar` still gets
//! sensible defaults for everything else.

use strum::Display;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub(crate) const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// One region/element of the UI that can have its background, text
/// color, or text size set, both from `config.lua`'s `theme` table at
/// startup, and live via `set_property` (see `set_property` in
/// `scripting.rs`) from `on_song_change`/`on_song_halfway`. Not every
/// variant supports every property (e.g. `PlaylistsView` has no
/// `text_color`), see `set_property` below for which combinations are
/// valid.
#[derive(EnumString, Display, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum UiElement {
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

/// A themeable value for some `UiElement`, already parsed and validated
/// by the time it's constructed (see `register_set_property` in
/// `scripting.rs`, the only place that builds one from raw Lua input).
/// Carried inside `PlayerCommand::SetProperty` from the script runtime
/// thread to `handle_command`, which resolves it against a specific
/// `UiElement` via `set_property`.
#[derive(Debug, Clone)]
pub enum UiProperty {
    Bg(Rgb),
    TextColor(Rgb),
    TextSize(i32),
}

/// Background colors for every major UI region. These are applied in
/// `main.rs` via `apply_theme`.
pub(crate) struct Background {
    pub(crate) sidebar: Rgb,
    pub(crate) now_playing_bar: Rgb,
    pub(crate) library_view: Rgb,
    pub(crate) album_view: Rgb,
    pub(crate) playlists_view: Rgb,
    pub(crate) open_playlist_view: Rgb,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            sidebar: Rgb::new(0x0e, 0x10, 0x1d),
            now_playing_bar: Rgb::new(0x0a, 0x0a, 0x0f),
            library_view: Rgb::new(0x1f, 0x1d, 0x2f),
            album_view: Rgb::new(0x1f, 0x1d, 0x2f),
            playlists_view: Rgb::new(0x1f, 0x1d, 0x2f),
            open_playlist_view: Rgb::new(0x1f, 0x1d, 0x2f),
        }
    }
}

/// Text colors for every label/title/subtitle in the UI.
pub(crate) struct TextColor {
    pub(crate) sidebar: Rgb,
    pub(crate) now_playing_song: Rgb,
    pub(crate) now_playing_artist: Rgb,
    pub(crate) detail_view_header_title: Rgb,
    pub(crate) detail_view_header_subtitle: Rgb,
    pub(crate) library_list_title: Rgb,
    pub(crate) library_list_subtitle: Rgb,
    pub(crate) album_list_title: Rgb,
    pub(crate) album_list_subtitle: Rgb,
}

impl Default for TextColor {
    fn default() -> Self {
        Self {
            sidebar: Rgb::new(0xcd, 0xd6, 0xf4),
            now_playing_song: Rgb::new(0xcd, 0xd6, 0xf4),
            now_playing_artist: Rgb::new(0xcd, 0xd6, 0xf4),
            detail_view_header_title: Rgb::new(0xcd, 0xd6, 0xf4),
            detail_view_header_subtitle: Rgb::new(0xcd, 0xd6, 0xf4),
            library_list_title: Rgb::new(0xcd, 0xd6, 0xf4),
            library_list_subtitle: Rgb::new(0xcd, 0xd6, 0xf4),
            album_list_title: Rgb::new(0xcd, 0xd6, 0xf4),
            album_list_subtitle: Rgb::new(0xcd, 0xd6, 0xf4),
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
pub struct Theme {
    pub(crate) bg: Background,
    pub(crate) text_color: TextColor,
    pub(crate) text_size: TextSize,
}

impl Theme {
    pub fn properties(&self) -> [(UiElement, UiProperty); 24] {
        use crate::config::theme::UiElement::*;
        use crate::config::theme::UiProperty::*;

        [
            // BACKGROUNDS
            (Sidebar, Bg(self.bg.sidebar)),
            (NowPlayingBar, Bg(self.bg.now_playing_bar)),
            (LibraryView, Bg(self.bg.library_view)),
            (AlbumView, Bg(self.bg.album_view)),
            (PlaylistsView, Bg(self.bg.playlists_view)),
            (OpenPlaylistView, Bg(self.bg.open_playlist_view)),
            // TEXT COLORS
            (Sidebar, TextColor(self.text_color.sidebar)),
            (NowPlayingSong, TextColor(self.text_color.now_playing_song)),
            (
                NowPlayingArtist,
                TextColor(self.text_color.now_playing_artist),
            ),
            (
                DetailViewHeaderTitle,
                TextColor(self.text_color.detail_view_header_title),
            ),
            (
                DetailViewHeaderSubtitle,
                TextColor(self.text_color.detail_view_header_subtitle),
            ),
            (
                LibraryListTitle,
                TextColor(self.text_color.library_list_title),
            ),
            (
                LibraryListSubtitle,
                TextColor(self.text_color.library_list_subtitle),
            ),
            (AlbumListTitle, TextColor(self.text_color.album_list_title)),
            (
                AlbumListSubtitle,
                TextColor(self.text_color.album_list_subtitle),
            ),
            // TEXT SIZES
            (Sidebar, TextSize(self.text_size.sidebar)),
            (NowPlayingSong, TextSize(self.text_size.now_playing_song)),
            (
                NowPlayingArtist,
                TextSize(self.text_size.now_playing_artist),
            ),
            (
                DetailViewHeaderTitle,
                TextSize(self.text_size.detail_view_header_title),
            ),
            (
                DetailViewHeaderSubtitle,
                TextSize(self.text_size.detail_view_header_subtitle),
            ),
            (
                LibraryListTitle,
                TextSize(self.text_size.library_list_title),
            ),
            (
                LibraryListSubtitle,
                TextSize(self.text_size.library_list_subtitle),
            ),
            (AlbumListTitle, TextSize(self.text_size.album_list_title)),
            (
                AlbumListSubtitle,
                TextSize(self.text_size.album_list_subtitle),
            ),
        ]
    }
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
fn get_color_or_default(table: Option<&mlua::Table>, key: &str, default: Rgb) -> Rgb {
    let Some(table) = table else {
        return default;
    };

    match table.get::<String>(key) {
        Ok(value) if is_valid_hex_color(&value) => hex_to_color(&value),
        _ => default,
    }
}

/// True if `str` is a 6-digit hex color, with or without a leading `#`.
pub(crate) fn is_valid_hex_color(s: &str) -> bool {
    let stripped = s.strip_prefix("#").unwrap_or(s);
    stripped.len() == 6 && stripped.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses a `#rrbbgg` (or `rrbbgg`) string into a Slint `Color`.
/// Malformed hex digits fall back to `0` for that channel rather than
/// erroring, since `is_valid_hex_color` should already have filtered out
/// anything that would fail here.
pub(crate) fn hex_to_color(hex: &str) -> Rgb {
    let stripped = hex.strip_prefix("#").unwrap_or(hex);
    let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
    Rgb::new(r, g, b)
}

/// Looks up a nested table by key, returning `None` (rather than an
/// empty table) if it's missing or not a table. Callers treat `None` the
/// same way as an absent key and fall through to their own default.
fn get_table(table: &mlua::Table, key: &str) -> Option<mlua::Table> {
    table.get::<mlua::Table>(key).ok()
}
