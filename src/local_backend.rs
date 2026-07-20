//! The local filesystem audio backend: scans a music directory, builds an
//! in-memory library (albums, tracks, playlists), and drives actual audio
//! playback via `rodio`.
//!
//! `LocalBackend` owns the audio device and the currently-playing queue,
//! everything else in the app (UI, MPRIS) talks to it through its public
//! methods rather than touching playback state directly.

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

/// A single track, with metadata read from its file tags (falling back to
/// sensible defaults when a tag is missing).
///
/// Always wrapped in `Arc` once constructed, since the same song is shared
/// between the library's albums, the `song_paths` lookup table, and
/// whichever queue currently has it selected.
pub struct Song {
    pub path: PathBuf,

    pub title: String,
    pub artist: String,

    pub album_title: String,
    pub album_artist: String,
    /// Not `pub`: only used internally for sorting a tracklist/playlist
    /// into the right order (see `resolve_playlist` and `LocalBackend::new`).
    track_number: Option<u32>,
    pub duration: Duration,
    pub art: Option<Arc<Vec<u8>>>,
    // todo: lyrics and disc number
}

/// A group of songs sharing the same `(album_title, album_artist)`,
/// discovered by scanning the music directory, not something the user
/// defines directly (unlike `Playlist` below).
pub struct Album {
    pub title: String,
    pub artist: String,
    pub tracklist: Vec<Arc<Song>>,
    pub art: Option<Arc<Vec<u8>>>,
    // todo: year and genres
}

/// A user-defined playlist, build from `config.lua` (see
/// `config::PlaylistDef` and `build_playlists`).
///
/// Stores only song *paths*, not resolved `Song`s: a playlist may
/// reference songs that no longer exist on disk, so resolution happens
/// lazily and safely via `resolve_playlist`.
pub struct Playlist {
    pub name: String,
    pub songs: Vec<PathBuf>,
    /// If true, `resolve_playlist` sorts these songs by artist, then
    /// album, then track number, instead of returning them in the order
    /// listed. Set from `PlaylistDef.sort`.
    pub sort: bool,
}

/// Owns the audio device, the scanned library, and playback state.
///
/// `_stream`/`player` hold onto the real audio device (dropping them
/// tears down playback, which is why `_stream` is kept alive here even
/// though it's never read directly).
pub struct LocalBackend {
    _stream: MixerDeviceSink,
    player: Player,

    pub library: Vec<Album>,
    pub playlists: Vec<Playlist>,
    /// Every known song, keyed by its path, for O(1) lookup. Used by
    /// `find_song_by_path` when resolving a playlist's song path back
    /// into real `Song`s.
    song_paths: HashMap<PathBuf, Arc<Song>>,
    /// The currently active play queue. Populated by `select_album`,
    /// `select_playlist`, etc.
    queue: Vec<Arc<Song>>,
    /// Position within `queue` of the currently playing (or paused) track.
    index: usize, // index in current queue
}

impl LocalBackend {
    /// Scans `path` recursively for `.mp3/.flac` files, reads each one's
    /// tahs, and builds the library (grouped into albums) plus the
    /// playlists described by `playlist_defs`.
    ///
    /// Files that fail to read (corrupt, unsupported, permission denied,
    /// etc.) are silently skipped rather than aborting the whole scan.
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
    /// Stops whatever's currently playing and starts the track at
    /// `self.index` in `self.queue`.
    ///
    /// Panics if `self.queue` is empty or `self.index` is out of bounds:
    /// callers (`next`, `prev`, `select_*`) are responsible for only
    /// calling this when the queue is known to be non-empty and `index`
    /// valid. `select:playlist`/`select_playlist_track` already guard for
    /// the empty case; see the todo there about `select_playlist_track`
    /// not yet validating `track_index`.
    pub fn load_track(&mut self) -> Result<(), Box<dyn Error>> {
        self.player.stop();

        let track = std::fs::File::open(&self.queue[self.index].path)?;
        let source = rodio::Decoder::try_from(track)?;
        self.player.append(source);
        self.player.play();

        Ok(())
    }

    /// Advances to the next track in the queue, if there is one.
    /// Returns `Ok(true)` if it moved, `Ok(false)` if already at the end.
    /// Callers use this to avoid repeatedly trying to advance once the 
    /// queue is exhausted (see `TickState` in `player_bridge.rs`).
    pub fn next(&mut self) -> Result<bool, Box<dyn Error>> {
        if self.index + 1 < self.queue.len() {
            self.index += 1;
            self.load_track()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Moves to the previous track in the queue, if not already at the
    /// start. Unlike `next`, there's no "did it move" signal needed here:
    /// nothing currently polls for "are we at the start" the way the tick
    /// loop polls for "did the queue just end".
    pub fn prev(&mut self) -> Result<(), Box<dyn Error>> {
        if self.index > 0 {
            self.index -= 1;
            self.load_track()?;
        }

        Ok(())
    }

    /// Replaces the queue with an album's full tracklist and starts
    /// playing from the first track.
    pub fn select_album(&mut self, album_index: usize) -> Result<(), Box<dyn Error>> {
        self.queue = self.library[album_index].tracklist.clone();
        self.index = 0;
        self.load_track()?;
        Ok(())
    }

    /// Same as `select_album`, but starts from a specific track within
    /// the album instead of the first.
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

    /// The currently selected track, or `None` if no queue is active.
    pub fn get_current_song(&self) -> Option<&Arc<Song>> {
        if self.queue.is_empty() {
            None
        } else {
            Some(&self.queue[self.index])
        }
    }

    /// True once the current track has finished playing (the underlying
    /// player's buffer is empty). Doesn't distinguish "finished" from
    /// "nothing was ever loaded", both look the same to `rodio`.
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

    /// O(1) lookup of a known song by its path, backed by `song_paths`.
    pub fn find_song_by_path(&self, path: &Path) -> Option<&Arc<Song>> {
        self.song_paths.get(path)
    }

    /// Resolves a playlist's stored song paths into actual `Song`s, using
    /// the current library. Songs that no longer exist on disk (or were
    /// never in the library to begin with) are silently dropped rather
    /// than causing an error.
    ///
    /// If the playlist has `sort` set, results are additionally sorted by
    /// artist, then album, then track number (see `Playlist.sort` for why).
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

    /// Resolves and plays an entire playlist from its first track.
    /// No-ops (rather than erroring) if every song in the playlist failed
    /// to resolve (e.g. all referenced files were deleted).
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

    /// Same as `select_playlist`, but starts from a specific track index
    /// within the resolved playlist.
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

/// Builds a `Song` from a sucessfully read `TaggedFile`.
///
/// Falls back to the file's own name for a missing title, and to
/// "Unknown Artist"/"Unknown Album" for missing artist/album tags, rather
/// than leaving them blank: every song should have something displayable.
/// Cover art prefers an explicit front cover picture, falling back to
/// whatever picture is embedded first if there's no front cover tagged.
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

/// Converts config parsed `PlaylistDef`s into real `Playlist`s, resolving
/// each song's path (relative to `music_dir`) into an absolute path.
///
/// This is intentionally the only place that needs to known playlist song
/// paths are relative: `Playlist.songs` downstream is always absolute
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
