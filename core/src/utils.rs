//! Conversions between the app's internal Rust data (from `local_backend`)
//! and the Slint-generated UI types (`SlintAlbum`, `SlintSong`, etc.), plus
//! small standalone helpers used across the codebase.

use crate::local_backend::Song;
use image::imageops::FilterType;
use std::path::PathBuf;

/// `Send`-safe stand-in for `SlintSongWithArt`, for use across a
/// `spawn_blocking` boundary (see `DecodedArt` for why `SlintSongWithArt`
/// itself can't cross one). Convert to the real Slint struct with
/// `raw_art_to_slint_image` once back on the UI thread: see
/// `player_bridge.rs`'s streaming playlist-loading, which is what this
/// exists for.
pub struct DecodedSong {
    pub title: String,
    pub artist: String,
    pub art: DecodedArt,
}

/// Plain, `Send`-safe decoded art: width/height/RGB bytes, no
/// `slint::Image` involved. `slint::Image` itself isn't `Send` (it's
/// backed by an `Rc`-like handle internally), so it can only ever be
/// constructed on the thread that will actually use it. This exists so
/// the expensive decode/resize work can still happen on a
/// `tokio::task::spawn_blocking` thread. See `decode_art` (safe anywhere)
/// and `raw_art_to_slint_image` (UI thread only).
pub struct DecodedArt {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Placeholder cover art, embedded at compile time.
const PLACEHOLDER_ART: &[u8] = include_bytes!("../../assets/images/cover_placeholder.png");

impl From<&Song> for DecodedSong {
    /// The `Send`-safe half of `song_rust_to_slint_with_art`. Safe to call
    /// from `spawn_blocking`.
    fn from(song: &Song) -> DecodedSong {
        DecodedSong {
            title: song.title.clone(),
            artist: song.artist.clone(),
            art: decode_art(song.art.as_deref().map(Vec::as_slice)),
        }
    }
}

/// Returns the provided art bytes if present and valid, otherwise the
/// bundled placeholder. Used anywhere that needs a guaranteed non-empty
/// image (playlist tracks, album covers, now playing art).
pub(crate) fn art_or_placeholder(art: Option<&[u8]>) -> &[u8] {
    art.filter(|b| !b.is_empty()).unwrap_or(PLACEHOLDER_ART)
}

/// The `Send`-safe half of art conversion: decodes and resizes to a fixed
/// 100x100 thumbnail, same as `art_rust_to_slint`, but stops short of
/// building a `slint::Image`. Safe to call from any thread, including
/// `spawn_blocking`.
#[must_use]
pub fn decode_art(art: Option<&[u8]>) -> DecodedArt {
    let bytes = art_or_placeholder(art);
    let image = image::load_from_memory(bytes).expect("placeholder art is a valid image");
    let image = image.resize(100, 100, FilterType::Lanczos3).into_rgb8();
    DecodedArt {
        width: image.width(),
        height: image.height(),
        rgb: image.into_raw(),
    }
}

/// Expands a leading `~/` in a path string to the user's home directory.
///
/// Paths without a `~/` prefix are returned as-is. Used for user-supplied
/// paths from `config.lua` (`music_dir`), which shouldn't require users to
/// spell out their home directory.
pub(crate) fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir().map_or_else(|| PathBuf::from(path), |home| home.join(stripped))
    } else {
        PathBuf::from(path)
    }
}
