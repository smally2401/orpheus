//! Conversions between the app's internal Rust data (from `local_backend`)
//! and the Slint-generated UI types (`SlintAlbum`, `SlintSong`, etc.), plus
//! small standalone helpers used across the codebase.

use crate::SlintAlbum;
use crate::SlintPlaylist;
use crate::SlintSong;
use crate::local_backend::Album;
use crate::local_backend::LocalBackend;
use crate::local_backend::Song;
use image::imageops::FilterType;
use slint::ModelRc;
use slint::VecModel;
use std::path::PathBuf;
use std::sync::Arc;

/// Converts a single `Song` into its Slint-facing representation, with
/// `art` left as a default (empty) image.
///
/// Used for album tracklists (see `tracklist_rust_to_slint`):
/// `album_view.slint` doesn't render per-track art (it shows the album's
/// own cover in its header instead), so decoding art here would be pure
/// wasted work. See `song_rust_to_slint_with_art` for the variant that
/// actually decodes it, used for playlists.
pub fn song_rust_to_slint(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
        art: slint::Image::default(),
    }
}

/// Same as `song_rust_to_slint`, but also decodes the song's own embedded
/// art via `art_rust_to_slint`. Used for playlist tracks, since a playlist
/// can mix songs from different albums, so each row needs its own art
/// rather than falling back to a single playlist level cover (see
/// `open_playlist_view.slint`).
pub fn song_rust_to_slint_with_art(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
        art: art_rust_to_slint(song.art.as_deref().map(Vec::as_slice)),
    }
}

/// Converts a tracklist into the `ModelRc` Slint expects for list items.
/// Uses the art-free `song_rust_to_slint`, see `playlist_tracklist_rust_to_slint`
/// for the playlist equivalent that includes per-track art,
pub fn tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist.iter().map(|s| song_rust_to_slint(s)).collect();

    ModelRc::new(VecModel::from(slint_tracklist))
}

/// Same as `tracklist_rust_to_slint`, but includes each track's own
/// decoded art (see `song_rust_to_slint_with_art`). Used for playlists.
pub fn playlist_tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist
        .iter()
        .map(|s| song_rust_to_slint_with_art(s))
        .collect();

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

/// Decodes raw ebedded cover-art bytes into a Slint `Image`, resizing to a
/// fixed 100x100 thumbnail.
///
/// Returns a default (empty) image if there's no art, or if the bytes fail
/// to decode as an image, so callers don't need to handle that case
/// separately.
pub fn art_rust_to_slint(art: Option<&[u8]>) -> slint::Image {
    match art {
        Some(art) => match image::load_from_memory(art) {
            Ok(image) => {
                let image = image.resize(100, 100, FilterType::Lanczos3).into_rgb8();
                let width = image.width();
                let height = image.height();
                let raw = image.into_raw();
                let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                    &raw, width, height,
                );

                slint::Image::from_rgb8(buffer)
            }
            Err(_) => slint::Image::default(),
        },
        None => slint::Image::default(),
    }
}

/// Converts a playlist into its Slint-facing representation.
///
/// Unlike albums, a playlist's tracklist isn't stored directly so it has to
/// be resolved from the backend's library each time, since playlists only
/// store song paths (see `LocalBackend::resolve_playlist`).
pub fn playlist_rust_to_slint(playlist_index: usize, backend: &LocalBackend) -> SlintPlaylist {
    let resolved_playlist = backend.resolve_playlist(playlist_index);

    SlintPlaylist {
        name: backend.playlists[playlist_index].name.clone().into(),
        track_count: resolved_playlist.len() as i32,
        tracks: playlist_tracklist_rust_to_slint(&resolved_playlist),
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
