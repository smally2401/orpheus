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
use rand::seq::SliceRandom;
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
    // todo: disc number
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

/// Controls what `next` does once it's called with `from_click: false`,
/// i.e. after a track finishes on its own (see `TickState::on_tick` in
/// `player_bridge.rs`). Has no effect on a manual "next" click, which
/// always advances regardless of this setting (see `next`).
#[derive(Clone, Copy)]
pub enum RepeatMode {
    /// Stop advancing once the queue's last track finishes.
    Off,
    /// Once the last track finishes, wrap back around to the first.
    Queue,
    /// Replay the same track from the start every time it finishes.
    Track,
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
    /// The currently active play queue, in canonical (unshuffled) order.
    /// Populated by `select_album`, `select_playlist`, etc. Always
    /// resolved via `order`, never indexed into directly.
    queue: Vec<Arc<Song>>,
    /// A permutation of `queue`'s indices: playback walks `order`, and
    /// `order[i]` gives the real position in `queue` for slot `i`. When
    /// `shuffle` is off this is just `0..queue.len()` (identity); when on,
    /// it's shuffled, with the currently playing track's real index
    /// swapped into `order[0]`, so toggling shuffle mid-song doesn't change
    /// what's playinh (see `toggle_shuffle`). Rebuilt any time `queue` is
    /// replaced, so it's always the same length as `queue`.
    order: Vec<usize>,
    /// Position *within `order`* (not directly within `queue`) of the
    /// currently playing (or paused) track. Resolve the actual song via
    /// `queue[order[index]]`.
    index: usize,

    shuffle: bool,
    repeat: RepeatMode,
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
            order: Vec::new(),
            playlists,
            index: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
        }
    }

    // todo: more precise error returns
    /// Stops whatever's currently playing and starts the track at
    /// `self.queue[self.order[self.index]]`.
    ///
    /// Panics if `self.queue ` is empty, `self.index` is out of bounds for
    /// `self.order`, or `self.order` is a different length than
    /// `self.que` (shouldn't happen: `order` is always rebuilt alongside
    /// `queue`, see `select_*` and `toggle_shuffle`). Callers (`next`,
    /// `prev`, `select_*`) are responsible for only calling this when the
    /// queue is known to be non-empty and `index` valid. `select_playlist`/
    /// `select_playlist_track` already guard for the empty case, see the
    /// todo there about `select_playlist_track` not yet validating
    /// `track_index`.
    pub fn load_track(&mut self) -> Result<(), Box<dyn Error>> {
        self.player.stop();

        let track = std::fs::File::open(&self.queue[self.order[self.index]].path)?;
        let source = rodio::Decoder::try_from(track)?;
        self.player.append(source);
        self.player.play();

        Ok(())
    }

    /// Advances to the next track in the queue, if there is one.
    /// Returns `Ok(true)` if playback moved/restarted, `Ok(false)` if
    /// already at the end and nothing happened. Callers use this to avoid
    /// repeatedly trying to advance once the queue is exhaused (see
    /// `TickState` in `player_bridge.rs`).
    ///
    /// `from_click` distinguishes a manual "next" press from an automatic
    /// advance after a track finishes on its own:
    /// - `from_click: true` (manual): always steps forward one track,
    ///   ignoring `self.repeat` entirely. Even in `RepeatMode::Track`,
    ///   clicking next should skip to the next track, not replay the
    ///   current one.
    /// - `from_click: false` (automatic, called from the tick loop):
    ///   behaviour depends on `self.repeat`. `Off` behaves like the manual
    ///   case. `Queue` wraps back to index `0` instead of stopping, and
    ///   always returns `Ok(true)` so `queue_exhausted` never latches.
    ///   `Track` reloads the same track in plave via `load_track`.
    pub fn next(&mut self, from_click: bool) -> Result<bool, Box<dyn Error>> {
        if from_click {
            if self.index + 1 < self.queue.len() {
                self.index += 1;
                self.load_track()?;
                return Ok(true);
            }
            return Ok(false);
        }

        match self.repeat {
            RepeatMode::Off => {
                if self.index + 1 < self.queue.len() {
                    self.index += 1;
                    self.load_track()?;
                    return Ok(true);
                }
                Ok(false)
            }
            RepeatMode::Queue => {
                self.index = if self.index + 1 == self.queue.len() {
                    0
                } else {
                    self.index + 1
                };
                self.load_track()?;
                Ok(true)
            }
            RepeatMode::Track => {
                self.load_track()?;
                Ok(true)
            }
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
    /// playing from the first track (in `order`, not necessarily
    /// `queue`'s own first track: if shuffle is on, `order` is freshly
    /// shuffled here, so playback starts from whichever track lands at
    /// `order[0]`).
    pub fn select_album(&mut self, album_index: usize) -> Result<(), Box<dyn Error>> {
        self.queue = self.library[album_index].tracklist.clone();
        self.order = (0..self.queue.len()).collect();

        if self.shuffle {
            let mut rng = rand::rng();
            self.order.shuffle(&mut rng);
        }

        self.index = 0;
        self.load_track()?;
        Ok(())
    }

    /// Same as `select_album`, but starts from a specific track within
    /// the album instead of the first. If shuffle is on, the rest of the
    /// queue is still shuffled around it: `track_index`'s real position is
    /// swapped into `order[0]` after shuffling, same trick as
    /// `toggle_shuffle` uses to keep a chosen track pinned in place.
    pub fn select_album_track(
        &mut self,
        album_index: usize,
        track_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.queue = self.library[album_index].tracklist.clone();
        self.order = (0..self.queue.len()).collect();

        if self.shuffle {
            let mut rng = rand::rng();
            self.order.shuffle(&mut rng);

            let new_pos = self
                .order
                .iter()
                .position(|&x| x == track_index)
                .unwrap_or(0);
            self.order.swap(0, new_pos);
            self.index = 0;
        } else {
            self.index = track_index;
        }

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
            Some(&self.queue[self.order[self.index]])
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

    /// Resolves and plays an entire playlist from its first track (or,
    /// with shuffle on, from whichever track a fresh shuffle of `order`
    /// puts first, see `select_album` for the same behavior). No-ops
    /// (rather than erroring) if every song in the playlist failed to
    /// resolve (e.g. all referenced files were deleted).
    pub fn select_playlist(&mut self, playlist_index: usize) -> Result<(), Box<dyn Error>> {
        let queue = self.resolve_playlist(playlist_index);
        if queue.is_empty() {
            return Ok(()); // todo: maybe return an empty playlist error or something idk
        }
        self.queue = queue;
        self.order = (0..self.queue.len()).collect();

        if self.shuffle {
            let mut rng = rand::rng();
            self.order.shuffle(&mut rng);
        }

        self.index = 0;
        self.load_track()?;
        Ok(())
    }

    /// Same as `select_playlist`, but starts from a specific track index
    /// within the resolved playlist. Same shuffle-pinning behaviour as
    /// `select_album_track`: with shuffle on, `tracl index`'s real
    /// position get swapped into `order[0]` so playback still starts on
    /// the chosen track.
    pub fn select_playlist_track(
        &mut self,
        playlist_index: usize,
        track_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.queue = self.resolve_playlist(playlist_index);
        self.order = (0..self.queue.len()).collect();

        if self.shuffle {
            let mut rng = rand::rng();
            self.order.shuffle(&mut rng);

            let new_pos = self
                .order
                .iter()
                .position(|&x| x == track_index)
                .unwrap_or(0);
            self.order.swap(0, new_pos);
            self.index = 0;
        } else {
            self.index = track_index;
        }

        self.load_track()?;
        Ok(())
    }

    /// Flips `shuffle` and rebuilds `order` to match, without
    /// interrupting whatever's currently playing.
    ///
    /// Turning shuffle *on*: reshuffles `order`, then finds wherever the
    /// currently playing track ended up and swaps it into `order[0]`,
    /// resetting `index` to `0` to match. Without this swap, toggling
    /// shuffle mid-song would either restart the current track from a
    /// different queue position or silently jump to a random one.
    ///
    /// Turning shuffle *off*: resets `order` to identity (`0..len`) and
    /// restores `index` to the current track's real, unshuffled position,
    /// so playback continues uninterrupted in canonical order from here.
    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;

        if self.queue.is_empty() {
            return;
        }

        let real_index = self.order[self.index];

        if self.shuffle {
            let mut rng = rand::rng();
            self.order.shuffle(&mut rng);
            let new_pos = self
                .order
                .iter()
                .position(|&x| x == real_index)
                .unwrap_or(0);
            self.order.swap(0, new_pos);
            self.index = 0;
        } else {
            self.order = (0..self.queue.len()).collect();
            self.index = real_index;
        }
    }

    /// Cycles `self.reoeat`: `Off` -> `Queue` -> `Track` -> `Off`. Only
    /// changes what happens the *next* time a track finishes naturally,
    /// doesn't touch anything currently playing (see `next`).
    pub fn toggle_repeat(&mut self) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::Queue,
            RepeatMode::Queue => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Off,
        }
    }

    /// Sets shuffle to an explicit value (as opposed to `toggle_shuffle`,
    /// which flips it). Used when MPRIS reports a `Shuffle` property set
    /// rather than a toggle (see `PlayerCommand::SetShuffle`). No-ops if
    /// already at the requested value; otherwise just defers to
    /// `toggle_shuffle` so both paths share the same order rebuilding/
    /// current track pinning logic rather than duplicating it here.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        if shuffle != self.shuffle {
            self.toggle_shuffle();
        }
    }

    /// Sets repeat mode to an explicit value (see `set_shuffle` for why
    /// this exists alongside `toggle_repeat`).
    pub fn set_repeat(&mut self, repeat_mode: RepeatMode) {
        self.repeat = repeat_mode;
    }

    pub fn is_shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn get_repeat(&self) -> RepeatMode {
        self.repeat
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
