use slint::{ModelRc, VecModel};

use crate::{ SlintAlbum, SlintSong };
use crate::local_backend::{ Album, Song };

pub fn song_rust_to_slint(song: &Song) -> SlintSong {
    SlintSong {
        title: song.title.clone().into(),
        artist: song.artist.clone().into(),
    }
}

pub fn tracklist_rust_to_slint(tracklist: &[Song]) -> ModelRc<SlintSong> {
    let mut slint_tracklist: Vec<SlintSong> = Vec::new();

    for track in tracklist {
        slint_tracklist.push(song_rust_to_slint(track));
    }
    
    ModelRc::new(VecModel::from(slint_tracklist))
}

pub fn album_rust_to_slint(album: &Album) -> SlintAlbum {
    SlintAlbum {
        title: album.title.clone().into(),
        artist: album.artist.clone().into(),
        track_count: album.tracklist.len() as i32,
        tracks: tracklist_rust_to_slint(&album.tracklist),
    }
}
