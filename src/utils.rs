use slint::{ModelRc, VecModel};
use std::sync::Arc;

use crate::{ SlintAlbum, SlintSong };
use crate::local_backend::{ Album, Song };

pub fn song_rust_to_slint(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
    }
}

pub fn tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist
        .iter()
        .map(|s| song_rust_to_slint(s))
        .collect();

    ModelRc::new(VecModel::from(slint_tracklist))
}

pub fn album_rust_to_slint(album: &Album) -> SlintAlbum {
    SlintAlbum {
        title: album.title.clone().into(),
        artist: album.artist.clone().into(),
        track_count: album.tracklist.len() as i32,
        tracks: tracklist_rust_to_slint(&album.tracklist),
        art: art_rust_to_slint(album.art.as_deref().map(|v| v.as_slice())),
    }
}

pub fn art_rust_to_slint(art: Option<&[u8]>) -> slint::Image {
    match art {
        Some(art) => {
            match image::load_from_memory(art) {
                Ok(image) => {
                    let image = image.into_rgb8();
                    let width = image.width();
                    let height = image.height();
                    let raw = image.into_raw();
                    let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(&raw, width, height);

                    slint::Image::from_rgb8(buffer)
                },
                Err(_) => slint::Image::default()
            }
        },
        None => slint::Image::default()
    }
}
