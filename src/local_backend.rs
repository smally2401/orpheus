use std::{collections::HashMap, ffi::OsStr, path::PathBuf};
use lofty::{file::TaggedFileExt, tag::{Accessor, ItemKey}};
use rodio::{MixerDeviceSink, Player};
use walkdir::WalkDir;

const ALLOWED_EXTENSIONS: &[&str] = &["mp3", "flac"]; // todo: add more

#[derive(Clone)]
pub struct Song {
    path: PathBuf,

    title: String,
    artist: String,

    album: String,
    album_artist: String,

    // todo: lyrics
}

pub struct Album {
    title: String,
    artist: String,
    tracklist: Vec<Song>,

    // todo: art, year and genres
}

pub struct LocalBackend {
    stream: MixerDeviceSink,
    player: Player,

    library: Vec<Album>,
    queue: Vec<Song>,
    index: usize, // index in current queue
}

impl LocalBackend {
    fn new() -> Self {

        // todo: change the hardcoded path
        let entries = WalkDir::new("/home/iris/music").into_iter().filter_map(|e| e.ok());
        let audio_files = entries
            .filter(|e| {
                if !e.file_type().is_file() {
                    return false;
                }

                e.path()
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .map(|e| e.path());

        let mut albums: HashMap<(String, String), Album> = HashMap::new();
        for path in audio_files {

            let tagged_file = match lofty::read_from_path(path) {
                Ok(file) => file,
                Err(_) => continue,
            };
            let tag = tagged_file.primary_tag()
                .or_else(|| tagged_file.first_tag());

            let title = tag
                .and_then(|t| t.title())
                .map(|a| a.to_string())
                .unwrap_or_else(|| {
                    path.file_name()
                        .map(|os_str| os_str.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Unknown Title".to_string())
                });

            let artist = tag
                .and_then(|t| t.artist())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown Artist".to_string());

            let album = tag
                .and_then(|t| t.album())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown Album".to_string());

            let album_artist = tag
                .and_then(|t| t.get_string(ItemKey::AlbumArtist))
                .map(|a| a.to_string())
                .unwrap_or_else(|| artist);

            let song = Song {
                path: path.to_path_buf(),
                title,
                artist,
                album,
                album_artist,
            };

            if let Some(album) = albums.get(&(song.album, song.album_artist)) {
                album.tracklist.push(song);

            } else {
                let album = Album {
                    title: song.album,
                    artist: song.album_artist,
                    tracklist: Vec::new(),
                };
                album.tracklist.push(song);
                albums.insert((song.album, song.album_artist), album);
            }
        }
        let library: Vec<Album> = albums.into_values().collect();

        println!("haiii :3");
    }

    fn toggle_play(&mut self) {
        // self.playing = !self.playing;
    }

    fn next(&mut self) {
        if self.index < self.queue.len() - 1 {
            self.index += 1;
        }
    }

    fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }
}
