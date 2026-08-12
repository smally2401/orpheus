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
