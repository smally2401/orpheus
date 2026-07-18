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

pub fn song_rust_to_slint(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
    }
}

pub fn tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist.iter().map(|s| song_rust_to_slint(s)).collect();

    ModelRc::new(VecModel::from(slint_tracklist))
}

pub fn album_rust_to_slint(album: &Album) -> SlintAlbum {
    SlintAlbum {
        title: album.title.clone().into(),
        artist: album.artist.clone().into(),
        track_count: album.tracklist.len() as i32,
        tracks: tracklist_rust_to_slint(&album.tracklist),
        art: art_rust_to_slint(album.art.as_deref().map(Vec::as_slice)),
    }
}

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

pub fn playlist_rust_to_slint(playlist_index: usize, backend: &LocalBackend) -> SlintPlaylist {
    let resolved_playlist = backend.resolve_playlist(playlist_index);

    SlintPlaylist {
        name: backend.playlists[playlist_index].name.clone().into(),
        track_count: resolved_playlist.len() as i32,
        tracks: tracklist_rust_to_slint(&resolved_playlist),
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir().map_or_else(|| PathBuf::from(path), |home| home.join(stripped))
    } else {
        PathBuf::from(path)
    }
}
