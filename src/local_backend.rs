use std::{collections::HashMap, error::Error, ffi::OsStr, path::{Path, PathBuf}, time::Duration};
use lofty::{file::{AudioFile, TaggedFileExt}, tag::{Accessor, ItemKey}};
use rodio::{MixerDeviceSink, Player};
use walkdir::WalkDir;

const ALLOWED_EXTENSIONS: &[&str] = &["mp3", "flac"]; // todo: add more

#[derive(Clone)]
pub struct Song {
    path: PathBuf,

    pub title: String,
    pub artist: String,

    album_title: String,
    album_artist: String,
    track_number: Option<u32>,
    pub duration: Duration,

    // todo: lyrics and disc number
}

pub struct Album {
    pub title: String,
    pub artist: String,
    pub tracklist: Vec<Song>,

    // todo: art, year and genres
}

pub struct LocalBackend {
    _stream: MixerDeviceSink,
    player: Player,

    pub library: Vec<Album>,
    queue: Vec<Song>,
    index: usize, // index in current queue
}

impl LocalBackend {
    pub fn new(path: &Path) -> Self {

        let entries = WalkDir::new(path).into_iter().filter_map(|e| e.ok());

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
            .map(|e| e.into_path());

        let mut albums: HashMap<(String, String), Album> = HashMap::new();
        for path in audio_files {

            let tagged_file = match lofty::read_from_path(&path) {
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

            let album_title = tag
                .and_then(|t| t.album())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown Album".to_string());

            let album_artist = tag
                .and_then(|t| t.get_string(ItemKey::AlbumArtist))
                .map(|a| a.to_string())
                .unwrap_or_else(|| artist.clone());

            let track_number = tag
                .and_then(|t| t.track());

            let duration = tagged_file.properties().duration();

            let song = Song {
                path: path.to_path_buf(),
                title,
                artist,
                album_title,
                album_artist,
                track_number,
                duration,
            };

            if let Some(album) = albums.get_mut(&(song.album_title.clone(), song.album_artist.clone())) {
                album.tracklist.push(song);

            } else {
                let mut album = Album {
                    title: song.album_title.clone(),
                    artist: song.album_artist.clone(),
                    tracklist: Vec::new(),
                };
                album.tracklist.push(song.clone());
                albums.insert((song.album_title, song.album_artist), album);
            }
        }
        let mut library: Vec<Album> = albums.into_values().collect();

        // todo: add more ordering options
        library.sort_by_key(|album| album.title.clone());
        library.iter_mut()
            .for_each(|album| album.tracklist.sort_by_key(|s| s.track_number));

        let stream = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let mixer = stream.mixer();
        let player = rodio::Player::connect_new(mixer);

        LocalBackend {
            _stream: stream,
            player,
            library,
            queue: Vec::new(),
            index: 0,
        }
    }

    // todo: more precise error returns
    pub fn load_track(&mut self) -> Result<(), Box<dyn Error>> {
        self.player.stop();

        let track = std::fs::File::open(&self.queue[self.index].path)?;
        let source = rodio::Decoder::try_from(track)?;
        self.player.append(source);
        self.player.play();

        Ok(())
    }

    pub fn next(&mut self) -> Result<(), Box<dyn Error>> {
        if self.index + 1 < self.queue.len() {
            self.index += 1;
            self.load_track()?;
        }

        Ok(())
    }

    pub fn prev(&mut self) -> Result<(), Box<dyn Error>> {
        if self.index > 0 {
            self.index -= 1;
            self.load_track()?;
        }

        Ok(())
    }

    pub fn select_album(&mut self, album_index: usize) -> Result<(), Box<dyn Error>> {
        self.queue = self.library[album_index].tracklist.clone();
        self.index = 0;
        self.load_track()?;
        Ok(())
    } 

    pub fn select_track(&mut self, album_index: usize, track_index: usize) -> Result<(), Box<dyn Error>> {
        self.queue = self.library[album_index].tracklist.clone();
        self.index = track_index;
        self.load_track()?;
        Ok(())
    }

    pub fn toggle_play(&mut self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn get_current_song(&self) -> Option<&Song> {
        if self.queue.is_empty() { None } 
        else { Some(&self.queue[self.index]) }
    }

    pub fn track_finished(&self) -> bool {
        self.player.empty()
    }

    pub fn get_current_position(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn seek(&mut self, position: usize) -> Result<(), Box<dyn Error>> {
        self.player.try_seek(Duration::from_secs(position as u64))?;
        Ok(())
    }
}
