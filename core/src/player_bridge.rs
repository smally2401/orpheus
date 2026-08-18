//! Bridges the local audio backend (`LocalBackend`) to the Slint UI and to
//! MPRIS, and owns the app's main runtime loop.
//!
//! `spawn_player_bridge` sets everything up once at startup, then hands
//! back a command channel the UI (and MPRIS, indirectly) use to drive
//! playback (see `PlayerCommand`). A single background task then runs
//! forever, alternating between handling incoming commands and polling
//! playback state on a fixed interval (see `TickState::on_tick`).

use crate::config::playlist::PlaylistDef;
use crate::config::scripting::CurrentSong;
use crate::config::scripting::PlaybackState;
use crate::config::scripting::PlaybackStateHandle;
use crate::config::scripting::ScriptEvent;
use crate::config::theme::UiProperty;
use crate::local_backend::Album;
use crate::local_backend::LocalBackend;
use crate::local_backend::RepeatMode;
use crate::player_bridge::PlayerEvent::EqualizerReady;
use crate::state::restore_state;
use crate::utils::DecodedSong;
use crate::utils::expand_tilde;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct PlaylistPreview {
    pub name: String,
    pub track_count: usize,
    pub art_path: Option<PathBuf>,
}

pub enum PlayerEvent {
    UpdateVolume(f32),
    SetProperty(String, UiProperty),
    SetCurrentTrackInfo {
        title: String,
        artist: String,
        current_position: u64,
        total_duration: u64,
        album_changed: bool,
        track_art: Option<Arc<Vec<u8>>>,
    },
    PlaylistOpened {
        index: usize,
        name: String,
        track_count: i32,
        art_path: Option<PathBuf>,
    },
    PlaylistTrackDecoded {
        playlist_index: usize,
        track_index: usize,
        decoded: DecodedSong,
    },
    EqualizerReady(Vec<f32>),
}

/// Something the UI (or MPRIS) wants the player to do. Sent over the
/// channel returned by `spawn_player_bridge` and handled by
/// `handle_command`.
#[derive(Clone)]
pub enum PlayerCommand {
    TogglePlay,
    Play,
    Pause,
    NextTrack,
    PrevTrack,
    SelectAlbum(usize),
    SelectTrack(usize, usize),
    SetPosition(usize),
    SeekForward,
    SeekBackward,
    SeekBy(i64),
    SetVolume(f32),
    VolumeUp,
    VolumeDown,
    SelectPlaylist(usize),
    SelectPlaylistTrack(usize, usize),
    ToggleRepeat,
    ToggleShuffle,
    SetRepeat(RepeatMode),
    SetShuffle(bool),
    OpenPlaylist(usize),
    SetProperty(String, UiProperty),
}

/// State that needs to persist *between* ticks of the polling loop, so
/// `on_tick` can tell what's changed since last time and avoid redundant
/// work (or redundant MPRIS/UI updates).
struct TickState {
    /// The `(album_title, album_artist)` of the last track we decoded
    /// cover art for. Art only needs re-decoding when the *album*
    /// changes, not on every track change: many albums share one embedded
    /// image across all their tracks, so this avoids re-running the
    /// image resize on every track change within the same album.
    last_song_info: Option<(String, String)>,
    /// The path of the last track we reported to MPRIS. Unlike art, MPRIS
    /// metadata (title/artist/track id) genuinely changes per *track*, so
    /// this is tracked separately from `last_song_info` at a finer
    /// granularity.
    last_song_path: Option<PathBuf>,
    /// Whether playback was paused as of the last tick. Used to detect
    /// play/pause transitions so we only notify MPRIS when the status
    /// actually changes, not every 100ms.
    was_paused: bool,
    /// True once we've established the current queue has no more tracks
    /// to advance to. Prevents calling `LocalBackend::next` every single
    /// tick once playback reaches the end. Without this, `next()` would
    /// be called 10 times a second doing nothing until the user picks a
    /// new queue. Reset to `false` whenever a command changes the queue
    /// (see `handle_command`).
    queue_exhausted: bool,
    /// Reports song changes to the dedicated script runtime thread (see
    /// `main.rs`), so `config.lua`'s `on_song_change` can be called and
    /// `current_song()` can stay up to date. Sent alongside the existing
    /// MPRIS metadata update in `on_tick`, since both fire on exactly the
    /// same "track changed" condition.
    script_tx: std::sync::mpsc::Sender<ScriptEvent>,
    /// Live playback state, kept in sync every tick, read by
    /// `player.get_state()` on the script thread. See
    /// `config::scripting::PlaybackState`.
    playback_state: PlaybackStateHandle,
}

impl TickState {
    fn new(
        script_tx: std::sync::mpsc::Sender<ScriptEvent>,
        playback_state: PlaybackStateHandle,
    ) -> Self {
        TickState {
            last_song_info: None,
            last_song_path: None,
            was_paused: true,
            queue_exhausted: true,
            script_tx,
            playback_state,
        }
    }

    /// Runs one polling cycle: advances the queue if the current track
    /// finished, syncs playback status/position to MPRIS, reports song
    /// changes to the script runtime, and pushes updated track info
    /// (title, artist, position, art) to the UI. Called on a fixed
    /// interval from `spawn_player_bridge`'s main loop.
    fn on_tick(&mut self, local_backend: &mut LocalBackend, player_tx: &mpsc::Sender<PlayerEvent>) {
        if local_backend.track_finished() && !self.queue_exhausted {
            self.queue_exhausted = !local_backend.next(false).unwrap_or(false);
        }

        let is_paused = local_backend.is_paused();
        if is_paused != self.was_paused {
            self.was_paused = is_paused;
        }

        let current_position = local_backend.get_current_position().as_secs();

        let eq_bars = if is_paused {
            None
        } else {
            local_backend.player.equalizer_tick()
        };

        if let Some(track) = local_backend.get_current_song() {
            let title = track.title.clone();
            let artist = track.artist.clone();
            let total_duration = track.duration.as_secs();

            *self.playback_state.lock().unwrap() = Some(PlaybackState {
                title: title.clone(),
                artist: artist.clone(),
                album: track.album_title.clone(),
                position: current_position,
                duration: total_duration,
                paused: is_paused,
            });

            if let Some(bars) = eq_bars {
                let _ = player_tx.try_send(EqualizerReady(bars));
            }

            let current_album = (track.album_title.clone(), track.album_artist.clone());
            let album_changed = self.last_song_info.as_ref() != Some(&current_album);
            if album_changed {
                self.last_song_info = Some(current_album);
            }

            if self.last_song_path.as_ref() != Some(&track.path) {
                self.last_song_path = Some(track.path.clone());

                let current_song = CurrentSong::from(track.as_ref());
                let _ = self.script_tx.send(ScriptEvent::SongChanged(current_song));
            }

            let track_art = track.art.clone();
            let _ = player_tx.try_send(PlayerEvent::SetCurrentTrackInfo {
                title,
                artist,
                current_position,
                total_duration,
                album_changed,
                track_art,
            });
        } else {
            self.last_song_info = None;
            self.last_song_path = None;

            *self.playback_state.lock().unwrap() = None;

            let _ = player_tx.try_send(PlayerEvent::SetCurrentTrackInfo {
                title: String::from("No song playing"),
                artist: String::from("---"),
                current_position: 0,
                total_duration: 0,
                album_changed: true,
                track_art: None,
            });
        }
    }
}

/// Sets up the local backend, MPRIS, and the app's runtime loop, returning
/// a command channel for the UI to drive playback plus the initial
/// library/playlist data to populate the UI with at startup.
///
/// `song_tx` is where song changes get reported to, for `config.lua`'s
/// `on_song_change`/`current_song()` support: see `main.rs`'s dedicated
/// script runtime thread, which owns the other end of this channel.
#[must_use]
pub fn spawn_player_bridge(
    music_dir: &str,
    playlist_defs: Vec<PlaylistDef>,
    default_volume: f32,
    script_tx: std::sync::mpsc::Sender<ScriptEvent>,
) -> (
    mpsc::Sender<PlayerCommand>,
    mpsc::Receiver<PlayerEvent>,
    Vec<Album>,
    Vec<PlaylistPreview>,
    PlaybackStateHandle,
) {
    let path = setup_music_dir(music_dir);
    let mut local_backend = LocalBackend::new(&path, playlist_defs, default_volume);

    let library = local_backend.library.clone();
    let playlists = build_playlist_previews(&local_backend);

    restore_state(&mut local_backend);

    let (tx, mut rx) = mpsc::channel::<PlayerCommand>(100);
    let (player_tx, player_rx) = mpsc::channel::<PlayerEvent>(100);

    let playback_state: PlaybackStateHandle = Arc::new(Mutex::new(None));
    let playback_state_for_tick = playback_state.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut tick_state = TickState::new(script_tx, playback_state_for_tick);

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        handle_command(&command, &mut local_backend, &mut tick_state, &player_tx);
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {
                    tick_state.on_tick(&mut local_backend, &player_tx);
                }
            }
        }
    });

    (tx, player_rx, library, playlists, playback_state)
}

/// Applies a single `Playercommand` to the backend.
///
/// Commands that select a new queue (`SelectAlbum`, `SelectTrack`,
/// `SelectPlaylist`, `SelectPlaylistTrack`) and `PrevTrack` all reset
/// `queue_exhausted`, so the tick loop knows it's fine to try advancing
/// again (see `TickState::queue_exhausted`). `NextTrack` deliberately
/// doesn't: if it also hits the end, `next()` returns `false` again and
/// the flag stays accurate without needing an explicit reset.
///
/// `ToggleShuffle`/`ToggleRepeat` also push the new state to MPRIS after
/// updating the backend, so an in-app click stays in sync with any
/// lock-screen/media-key widget showing shuffle/repeat state.
fn handle_command(
    command: &PlayerCommand,
    local_backend: &mut LocalBackend,
    tick_state: &mut TickState,
    event_tx: &mpsc::Sender<PlayerEvent>,
) {
    use PlayerCommand::*;

    const VOLUME_STEP: f32 = 0.05;
    const SEEK_STEP: u64 = 10;

    match command {
        // todo: remove let _ and handle stuff
        TogglePlay => {
            local_backend.toggle_play();
        }
        Play => {
            local_backend.play();
        }
        Pause => {
            local_backend.pause();
        }
        NextTrack => {
            let _ = local_backend.next(true);
        }
        PrevTrack => {
            let _ = local_backend.prev();
            tick_state.queue_exhausted = false;
        }
        SelectAlbum(i) => {
            let _ = local_backend.select_album(*i);
            tick_state.queue_exhausted = false;
        }
        SelectTrack(album_i, track_i) => {
            let _ = local_backend.select_album_track(*album_i, *track_i);
            tick_state.queue_exhausted = false;
        }
        SetPosition(dur) => {
            let _ = local_backend.set_position(*dur);
        }
        SeekForward => {
            seek_by(local_backend, SEEK_STEP, false);
        }
        SeekBackward => {
            seek_by(local_backend, SEEK_STEP, true);
        }
        SeekBy(secs) => {
            seek_by(local_backend, secs.unsigned_abs(), *secs < 0);
        }
        SetVolume(vol) => {
            set_volume_and_update_ui(local_backend, *vol, event_tx);
        }
        VolumeUp => {
            let vol = (local_backend.get_volume() + VOLUME_STEP).clamp(0.0, 1.0);
            set_volume_and_update_ui(local_backend, vol, event_tx);
        }
        VolumeDown => {
            let vol = (local_backend.get_volume() - VOLUME_STEP).clamp(0.0, 1.0);
            set_volume_and_update_ui(local_backend, vol, event_tx);
        }
        SelectPlaylist(i) => {
            let _ = local_backend.select_playlist(*i);
            tick_state.queue_exhausted = false;
        }
        SelectPlaylistTrack(playlist_i, track_i) => {
            let _ = local_backend.select_playlist_track(*playlist_i, *track_i);
            tick_state.queue_exhausted = false;
        }
        ToggleRepeat => {
            local_backend.toggle_repeat();
        }
        ToggleShuffle => {
            local_backend.toggle_shuffle();
        }
        SetRepeat(mode) => {
            local_backend.set_repeat(*mode);
        }
        SetShuffle(shuffle) => {
            local_backend.set_shuffle(*shuffle);
        }
        OpenPlaylist(i) => {
            open_playlist(*i, local_backend, event_tx);
        }
        SetProperty(element, property) => {
            let element = element.clone();
            let property = property.clone();
            let _ = event_tx.try_send(PlayerEvent::SetProperty(element, property));
        }
    }
}

fn seek_by(backend: &mut LocalBackend, offset_secs: u64, backwards: bool) {
    let current_pos = backend.get_current_position().as_secs();
    let new_pos = if backwards {
        current_pos.saturating_sub(offset_secs)
    } else {
        current_pos + offset_secs
    };
    let _ = backend.set_position(new_pos as usize);
}

fn set_volume_and_update_ui(
    backend: &mut LocalBackend,
    volume: f32,
    event_tx: &mpsc::Sender<PlayerEvent>,
) {
    backend.set_volume(volume);
    let _ = event_tx.try_send(PlayerEvent::UpdateVolume(volume));
}

/// Loads a playlist's tracklist with per-song art, decoding cover images
/// in parallel on a background task while preserving track order.
///
/// The UI is updated immediately with an empty playlist, then tracks are
/// streamed in as their art finishes decoding. Track order is preserved
/// by collecting all results, sorting by original index, then pushing
/// sequentially.
fn open_playlist(i: usize, local_backend: &LocalBackend, event_tx: &mpsc::Sender<PlayerEvent>) {
    let resolved = local_backend.resolve_playlist(i);
    let name = local_backend.playlists[i].name.clone();
    let track_count = resolved.len() as i32;
    let art_path = local_backend.playlists[i].art.clone();

    let _ = event_tx.try_send(PlayerEvent::PlaylistOpened {
        index: i,
        name,
        track_count,
        art_path,
    });

    let event_tx = event_tx.clone();
    tokio::spawn(async move {
        use crate::utils::DecodedSong;

        let handles: Vec<_> = resolved
            .into_iter()
            .enumerate()
            .map(|(idx, song)| {
                tokio::task::spawn(async move {
                    let decoded =
                        tokio::task::spawn_blocking(move || DecodedSong::from(song.as_ref()))
                            .await
                            .ok()?;
                    Some((idx, decoded))
                })
            })
            .collect();

        let mut results: Vec<(usize, DecodedSong)> = Vec::new();
        for handle in handles {
            if let Some((idx, decoded)) = handle.await.ok().flatten() {
                results.push((idx, decoded));
            }
        }
        results.sort_by_key(|(idx, _)| *idx);

        for (idx, decoded) in results {
            let _ = event_tx.try_send(PlayerEvent::PlaylistTrackDecoded {
                playlist_index: i,
                track_index: idx,
                decoded,
            });
        }
    });
}

/// Expands `music_dir` (e.g. a leading `~/`) into a real path, warning
/// (but not failing) if it doesn't actually exist: playback will simply
/// find no songs rather than crash.
fn setup_music_dir(music_dir: &str) -> PathBuf {
    let path = expand_tilde(music_dir);
    if !path.exists() {
        eprintln!("error: path {} could not be found", path.to_string_lossy());
    }
    path
}

fn build_playlist_previews(local_backend: &LocalBackend) -> Vec<PlaylistPreview> {
    local_backend
        .playlists
        .iter()
        .enumerate()
        .map(|(i, def)| PlaylistPreview {
            name: def.name.clone(),
            track_count: local_backend.resolve_playlist(i).len(),
            art_path: def.art.clone(),
        })
        .collect()
}
