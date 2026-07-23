//! Conversions between the app's internal Rust data (from `local_backend`)
//! and the Slint-generated UI types (`SlintAlbum`, `SlintSong`, etc.), plus
//! small standalone helpers used across the codebase.

use crate::SlintAlbum;
use crate::SlintPlaylist;
use crate::SlintSong;
use crate::SlintSongWithArt;
use crate::local_backend::Album;
use crate::local_backend::LocalBackend;
use crate::local_backend::Song;
use image::imageops::FilterType;
use slint::ModelRc;
use slint::VecModel;
use std::path::PathBuf;
use std::sync::Arc;

/// Converts a single `Song` into its Slint-facing representation.
pub fn song_rust_to_slint(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
    }
}

/// Converts a tracklist into the `ModelRc` Slint expects for list items.
pub fn tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist.iter().map(|s| song_rust_to_slint(s)).collect();

    ModelRc::new(VecModel::from(slint_tracklist))
}

/// Converts an `Album` into its Slint-facing representation, including
/// a decoded, UI-ready cover image via `art_rust_to_slint`.
pub fn album_rust_to_slint(album: &Album) -> SlintAlbum {
    SlintAlbum {
        title: album.title.clone().into(),
        artist: album.artist.clone().into(),
        track_count: album.tracklist.len() as i32,
        tracks: tracklist_rust_to_slint(&album.tracklist),
        art: art_rust_to_slint(album.art.as_deref().map(Vec::as_slice)),
    }
}

/// Plain, `Send`-safe decoded art: width/height/RGB bytes, no
/// `slint::Image` involved. `slint::Image` itself isn't `Send` (it's
/// backed by an `Rc`-like handle internally), so it can only ever be
/// constructed on the thread that will actually use it - this exists so
/// the expensive decode/resize work can still happen on a
/// `tokio::task::spawn_blocking` thread. See `decode_art` (safe anywhere)
/// and `raw_art_to_slint_image` (UI thread only).
pub struct DecodedArt {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// The `Send`-safe half of art conversion: decodes and resizes to a fixed
/// 100x100 thumbnail, same as `art_rust_to_slint`, but stops short of
/// building a `slint::Image`. Safe to call from any thread, including
/// `spawn_blocking`.
pub fn decode_art(art: Option<&[u8]>) -> Option<DecodedArt> {
    let image = image::load_from_memory(art?).ok()?;
    let image = image.resize(100, 100, FilterType::Lanczos3).into_rgb8();
    Some(DecodedArt {
        width: image.width(),
        height: image.height(),
        rgb: image.into_raw(),
    })
}

/// The other half: wraps already-`decode_art`-ed bytes into a real
/// `slint::Image`. Must run on the thread that will use the resulting
/// image (in practice, the UI thread, e.g. inside
/// `slint::invoke_from_event_loop`).
pub fn raw_art_to_slint_image(art: Option<DecodedArt>) -> slint::Image {
    match art {
        Some(art) => {
            let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                &art.rgb, art.width, art.height,
            );
            slint::Image::from_rgb8(buffer)
        }
        None => slint::Image::default(),
    }
}

/// Decodes raw ebedded cover-art bytes into a Slint `Image`, resizing to a
/// fixed 100x100 thumbnail.
///
/// Returns a default (empty) image if there's no art, or if the bytes fail
/// to decode as an image, so callers don't need to handle that case
/// separately. For synchronous callers on the UI thread only (e.g.
/// `album_rust_to_slint`); see `decode_art`/`raw_art_to_slint_image` for
/// the split version usable from a background thread.
pub fn art_rust_to_slint(art: Option<&[u8]>) -> slint::Image {
    raw_art_to_slint_image(decode_art(art))
}

/// `Send`-safe stand-in for `SlintSongWithArt`, for use across a
/// `spawn_blocking` boundary (see `DecodedArt` for why `SlintSongWithArt`
/// itself can't cross one). Convert to the real Slint struct with
/// `raw_art_to_slint_image` once back on the UI thread: see
/// `player_bridge.rs`'s streaming playlist-loading, which is what this
/// exists for.
pub struct DecodedSong {
    pub title: String,
    pub artist: String,
    pub art: Option<DecodedArt>,
}

/// The `Send`-safe half of `song_rust_to_slint_with_art`. Safe to call
/// from `spawn_blocking`.
pub fn decode_song_with_art(song: &Song) -> DecodedSong {
    DecodedSong {
        title: song.title.clone(),
        artist: song.artist.clone(),
        art: decode_art(song.art.as_deref().map(Vec::as_slice)),
    }
}

/// Builds a `SlintPlaylist` without decoding any track art. Used for the 
/// sidebar list at startup: art is loaded lazily when the playlist is opened.
pub fn playlist_rust_to_slint(playlist_index: usize, backend: &LocalBackend) -> SlintPlaylist {
    let resolved_playlist = backend.resolve_playlist(playlist_index);

    SlintPlaylist {
        name: backend.playlists[playlist_index].name.clone().into(),
        track_count: resolved_playlist.len() as i32,
        tracks: ModelRc::new(VecModel::<SlintSongWithArt>::from(Vec::new()))
    }
}

/// Expands a leading `~/` in a path string to the user's home directory.
///
/// Paths without a `~/` prefix are returned as-is. Used for user-supplied
/// paths from `config.lua` (`music_dir`), which shouldn't require users to
/// spell out their home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir().map_or_else(|| PathBuf::from(path), |home| home.join(stripped))
    } else {
        PathBuf::from(path)
    }
}

