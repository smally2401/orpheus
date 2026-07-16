use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::tag::ItemKey;
use rodio::MixerDeviceSink;
use rodio::Player;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Song {
    pub path: PathBuf,

    pub title: String,
    pub artist: String,

    album_title: String,
    album_artist: String,
    track_number: Option<u32>,
    pub duration: Duration,
    pub art: Option<Arc<Vec<u8>>>,
    // todo: lyrics and disc number
}

pub struct Album {
    pub title: String,
    pub artist: String,
    pub tracklist: Vec<Arc<Song>>,
    pub art: Option<Arc<Vec<u8>>>,
    // todo: art, year and genres
}

pub struct Playlist {
    pub name: String,
    pub songs: Vec<PathBuf>,
}

pub struct LocalBackend {
    _stream: MixerDeviceSink,
    player: Player,

    pub library: Vec<Album>,
    pub playlists: Vec<Playlist>,
    queue: Vec<Arc<Song>>,
    index: usize, // index in current queue
}

impl LocalBackend {
    #[allow(clippy::too_many_lines)]
    pub fn new(path: &Path) -> Self {
        let entries = WalkDir::new(path)
            .into_iter()
            .filter_map(std::result::Result::ok);

        let audio_files = entries
            .filter(|e| {
                if !e.file_type().is_file() {
                    return false;
                }

                e.path()
                    .extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("mp3") || ext.eq_ignore_ascii_case("flac")
                    })
            })
            .map(walkdir::DirEntry::into_path);

        let mut albums: HashMap<(String, String), Album> = HashMap::new();
        for path in audio_files {
            let Ok(tagged_file) = lofty::read_from_path(&path) else {
                continue;
            };
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            let title = tag.and_then(lofty::tag::Accessor::title).map_or_else(
                || {
                    path.file_name().map_or_else(
                        || "Unknown Title".to_string(),
                        |os_str| os_str.to_string_lossy().into_owned(),
                    )
                },
                |a| a.to_string(),
            );

            let artist = tag
                .and_then(lofty::tag::Accessor::artist)
                .map_or_else(|| "Unknown Artist".to_string(), |a| a.to_string());

            let album_title = tag
                .and_then(lofty::tag::Accessor::album)
                .map_or_else(|| "Unknown Album".to_string(), |a| a.to_string());

            let album_artist = tag
                .and_then(|t| t.get_string(ItemKey::AlbumArtist))
                .map_or_else(|| artist.clone(), std::string::ToString::to_string);

            let track_number = tag.and_then(lofty::tag::Accessor::track);

            let duration = tagged_file.properties().duration();

            let art: Option<Arc<Vec<u8>>> = tag.map(lofty::tag::Tag::pictures).and_then(|pics| {
                pics.iter()
                    .find(|p| p.pic_type() == PictureType::CoverFront)
                    .or_else(|| pics.first())
                    .map(|p| p.data().to_vec())
                    .map(Arc::new)
            });

            let song = Song {
                path: path.clone(),
                title,
                artist,
                album_title,
                album_artist,
                track_number,
                duration,
                art: art.clone(),
            };

            let album = albums
                .entry((song.album_title.clone(), song.album_artist.clone()))
                .or_insert_with(|| Album {
                    title: song.album_title.clone(),
                    artist: song.album_artist.clone(),
                    tracklist: Vec::new(),
                    art,
                });
            album.tracklist.push(Arc::new(song));
        }
        let mut library: Vec<Album> = albums.into_values().collect();

        // todo: add more ordering options
        library.sort_by_key(|album| album.title.clone());
        for album in &mut library {
            album.tracklist.sort_by_key(|s| s.track_number);
        }

        let stream = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let mixer = stream.mixer();
        let player = rodio::Player::connect_new(mixer);

        // PLAYLIST TEST
        let test_playlist = Playlist {
            name: "test playlist".to_string(),
            songs: vec![
                path.join("test/01 Breadcrumb Trail.mp3"),
                path.join("test/05 New Dawn Fades.mp3"),
            ],
        };
        let playlists = vec![test_playlist];

        LocalBackend {
            _stream: stream,
            player,
            library,
            queue: Vec::new(),
            playlists,
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

    pub fn select_album_track(
        &mut self,
        album_index: usize,
        track_index: usize,
    ) -> Result<(), Box<dyn Error>> {
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

    pub fn get_current_song(&self) -> Option<&Arc<Song>> {
        if self.queue.is_empty() {
            None
        } else {
            Some(&self.queue[self.index])
        }
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

    pub fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    pub fn find_song_by_path(&self, path: &Path) -> Option<&Arc<Song>> {
        for album in &self.library {
            for song in &album.tracklist {
                if song.path == path {
                    return Some(song);
                }
            }
        }

        None
    }

    pub fn resolve_playlist(&self, playlist_index: usize) -> Vec<Arc<Song>> {
        self.playlists[playlist_index]
            .songs
            .iter()
            .filter_map(|path| self.find_song_by_path(path))
            .cloned()
            .collect()
    }

    pub fn select_playlist(&mut self, playlist_index: usize) -> Result<(), Box<dyn Error>> {
        let queue = self.resolve_playlist(playlist_index);
        if queue.is_empty() {
            return Ok(()); // todo: maybe return an empty playlist error or something idk
        }
        self.queue = queue;
        self.index = 0;
        self.load_track()?;
        Ok(())
    }

    pub fn select_playlist_track(
        &mut self,
        playlist_index: usize,
        track_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.queue = self.resolve_playlist(playlist_index);
        self.index = track_index;
        self.load_track()?;
        Ok(())
    }
}
