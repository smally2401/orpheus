use crate::config::PlaylistDef;
use lofty::file::AudioFile;
use lofty::file::TaggedFile;
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

    pub album_title: String,
    pub album_artist: String,
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
    pub sort: bool,
}

pub struct LocalBackend {
    _stream: MixerDeviceSink,
    player: Player,

    pub library: Vec<Album>,
    pub playlists: Vec<Playlist>,
    song_paths: HashMap<PathBuf, Arc<Song>>,
    queue: Vec<Arc<Song>>,
    index: usize, // index in current queue
}

impl LocalBackend {
    pub fn new(path: &Path, playlist_defs: Vec<PlaylistDef>) -> Self {
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
        let mut song_paths: HashMap<PathBuf, Arc<Song>> = HashMap::new();
        for path in audio_files {
            let Ok(tagged_file) = lofty::read_from_path(&path) else {
                continue;
            };

            let song = Arc::new(song_from_tagged_file(&path, &tagged_file));
            song_paths.insert(path.clone(), song.clone());

            let album = albums
                .entry((song.album_title.clone(), song.album_artist.clone()))
                .or_insert_with(|| Album {
                    title: song.album_title.clone(),
                    artist: song.album_artist.clone(),
                    tracklist: Vec::new(),
                    art: song.art.clone(),
                });
            album.tracklist.push(song.clone());
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

        let playlists = build_playlists(path, playlist_defs);

        LocalBackend {
            _stream: stream,
            player,
            library,
            song_paths,
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

    pub fn next(&mut self) -> Result<bool, Box<dyn Error>> {
        if self.index + 1 < self.queue.len() {
            self.index += 1;
            self.load_track()?;
            Ok(true)
        } else {
            Ok(false)
        }
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

    pub fn play(&mut self) {
        self.player.play();
    }

    pub fn pause(&mut self) {
        self.player.pause();
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

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn set_position(&mut self, position: usize) -> Result<(), Box<dyn Error>> {
        self.player.try_seek(Duration::from_secs(position as u64))?;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    pub fn find_song_by_path(&self, path: &Path) -> Option<&Arc<Song>> {
        self.song_paths.get(path)
    }

    pub fn resolve_playlist(&self, playlist_index: usize) -> Vec<Arc<Song>> {
        let mut songs: Vec<Arc<Song>> = self.playlists[playlist_index]
            .songs
            .iter()
            .filter_map(|path| self.find_song_by_path(path))
            .cloned()
            .collect();

        if self.playlists[playlist_index].sort {
            songs.sort_by(|a, b| {
                a.album_artist
                    .cmp(&b.album_artist)
                    .then(a.album_title.cmp(&b.album_title))
                    .then(a.track_number.cmp(&b.track_number))
            });
        }

        songs
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

fn song_from_tagged_file(path: &Path, tagged_file: &TaggedFile) -> Song {
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag.and_then(lofty::tag::Accessor::title).map_or_else(
        || {
            path.file_name().map_or_else(
                || String::from("Unknown Title"),
                |os_str| os_str.to_string_lossy().into_owned(),
            )
        },
        |a| a.to_string(),
    );

    let artist = tag
        .and_then(lofty::tag::Accessor::artist)
        .map_or_else(|| String::from("Unknown Artist"), |a| a.to_string());

    let album_title = tag
        .and_then(lofty::tag::Accessor::album)
        .map_or_else(|| String::from("Unknown Album"), |a| a.to_string());

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

    Song {
        path: path.to_path_buf(),
        title,
        artist,
        album_title,
        album_artist,
        track_number,
        duration,
        art,
    }
}

fn build_playlists(path: &Path, playlist_defs: Vec<PlaylistDef>) -> Vec<Playlist> {
    playlist_defs
        .into_iter()
        .map(|def| Playlist {
            name: def.name,
            songs: def.songs.iter().map(|s| path.join(s)).collect(),
            sort: def.sort,
        })
        .collect()
}
