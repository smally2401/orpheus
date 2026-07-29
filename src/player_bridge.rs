//! Bridges the local audio backend (`LocalBackend`) to the Slint UI and to
//! MPRIS, and owns the app's main runtime loop.
//!
//! `spawn_player_bridge` sets everything up once at startup, then hands
//! back a command channel the UI (and MPRIS, indirectly) use to drive
//! playback (see `PlayerCommand`). A single background task then runs
//! forever, alternating between handling incoming commands and polling
//! playback state on a fixed interval (see `TickState::on_tick`).

use crate::AppWindow;
use crate::SlintAlbum;
use crate::SlintPlaylist;
use crate::SlintSongWithArt;
use crate::config::playlist::PlaylistDef;
use crate::local_backend::LocalBackend;
use crate::local_backend::RepeatMode;
use crate::mpris::MprisCommand;
use crate::mpris::repeat_mode_to_loop_status;
use crate::mpris::spawn_mpris;
use crate::mpris::track_id_for_path;
use crate::mpris::write_art_cache;
use crate::utils::album_rust_to_slint;
use crate::utils::art_rust_to_slint;
use crate::utils::expand_tilde;
use crate::utils::get_art_from_path;
use crate::utils::playlist_rust_to_slint;
use mpris_server::PlaybackStatus;
use slint::ComponentHandle;
use slint::Model;
use slint::ModelRc;
use slint::VecModel;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Something the UI (or MPRIS) wants the player to do. Sent over the
/// channel returned by `spawn_player_bridge` and handled by
/// `handle_command`.
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
}

impl TickState {
    fn new() -> Self {
        TickState {
            last_song_info: None,
            last_song_path: None,
            was_paused: true,
            queue_exhausted: true,
        }
    }

    /// Runs one polling cycle: advances the queue if the current track
    /// finished, syncs playback status/position to MPRIS, and pushes
    /// updated track info (title, artist, position, art) to the UI.
    /// Called on a fixed interval from `spawn_player_bridge`'s main loop
    fn on_tick(
        &mut self,
        local_backend: &mut LocalBackend,
        mpris_tx: &mpsc::Sender<MprisCommand>,
        ui: &slint::Weak<AppWindow>,
    ) {
        if local_backend.track_finished() && !self.queue_exhausted {
            self.queue_exhausted = !local_backend.next(false).unwrap_or(false);
        }

        let is_paused = local_backend.is_paused();
        if is_paused != self.was_paused {
            self.was_paused = is_paused;
            let status = if is_paused {
                PlaybackStatus::Paused
            } else {
                PlaybackStatus::Playing
            };
            let _ = mpris_tx.try_send(MprisCommand::UpdateStatus(status));
        }

        let current_position = local_backend.get_current_position().as_secs();
        let _ = mpris_tx.try_send(MprisCommand::UpdatePosition(current_position));

        let ui_weak_clone = ui.clone();
        if let Some(track) = local_backend.get_current_song() {
            let title = track.title.clone();
            let artist = track.artist.clone();
            let total_duration = track.duration.as_secs();

            let current_album = (track.album_title.clone(), track.album_artist.clone());
            let album_changed = self.last_song_info.as_ref() != Some(&current_album);
            if album_changed {
                self.last_song_info = Some(current_album);
            }

            let current_song_path = track.path.clone();
            if self.last_song_path != Some(current_song_path) {
                self.last_song_path = Some(track.path.clone());

                let art_url = track
                    .art
                    .as_ref()
                    .and_then(|bytes| write_art_cache(&track.path, bytes));

                let _ = mpris_tx.try_send(MprisCommand::UpdateMetadata {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album_title.clone(),
                    track_id: track_id_for_path(&track.path),
                    length: track.duration.as_secs(),
                    art_url,
                });
            }

            let track_art = track.art.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui_instance) = ui_weak_clone.upgrade() {
                    ui_instance.set_current_track_title(title.into());
                    ui_instance.set_current_artist(artist.into());
                    ui_instance.set_current_position(current_position as i32);
                    ui_instance.set_total_duration(total_duration as i32);
                    if album_changed {
                        ui_instance.set_current_art(art_rust_to_slint(
                            track_art.as_deref().map(Vec::as_slice),
                        ));
                    }
                }
            });
        } else {
            self.last_song_info = None;
            self.last_song_path = None;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui_instance) = ui_weak_clone.upgrade() {
                    ui_instance.set_current_track_title("No song playing".into());
                    ui_instance.set_current_artist("---".into());
                    ui_instance.set_current_position(0);
                    ui_instance.set_total_duration(0);
                    ui_instance.set_current_art(art_rust_to_slint(None));
                }
            });
        }
    }
}

/// Sets up the local backend, MPRIS, and the app's runtime loop, returning
/// a command channel for the UI to drive playback plus the initial
/// library/playlist data to populate the UI with at startup.
pub fn spawn_player_bridge(
    ui: &AppWindow,
    music_dir: &str,
    playlist_defs: Vec<PlaylistDef>,
    default_volume: f32,
) -> (
    mpsc::Sender<PlayerCommand>,
    Vec<SlintAlbum>,
    Vec<SlintPlaylist>,
) {
    let path = setup_music_dir(music_dir);
    let mut local_backend = LocalBackend::new(&path, playlist_defs, default_volume);
    let library = build_slint_library(&local_backend);
    let playlists = build_slint_playlists(&local_backend);

    let (tx, mut rx) = mpsc::channel::<PlayerCommand>(100);
    let ui = ui.as_weak();
    let mpris_tx = spawn_mpris(tx.clone());

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut tick_state = TickState::new();

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        handle_command(&command, &mut local_backend, &mpris_tx, &mut tick_state, &ui);
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {
                    tick_state.on_tick(&mut local_backend, &mpris_tx, &ui);
                }
            }
        }
    });

    (tx, library, playlists)
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
    mpris_tx: &mpsc::Sender<MprisCommand>,
    tick_state: &mut TickState,
    ui: &slint::Weak<AppWindow>,
) {
    match command {
        // todo: remove let _ and handle stuff
        PlayerCommand::TogglePlay => {
            local_backend.toggle_play();
        }
        PlayerCommand::Play => {
            local_backend.play();
        }
        PlayerCommand::Pause => {
            local_backend.pause();
        }
        PlayerCommand::NextTrack => {
            let _ = local_backend.next(true);
        }
        PlayerCommand::PrevTrack => {
            let _ = local_backend.prev();
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::SelectAlbum(i) => {
            let _ = local_backend.select_album(*i);
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::SelectTrack(album_i, track_i) => {
            let _ = local_backend.select_album_track(*album_i, *track_i);
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::SetPosition(dur) => {
            let _ = local_backend.set_position(*dur);
            let _ = mpris_tx.try_send(MprisCommand::Seeked(*dur as u64));
        }
        PlayerCommand::SeekForward => {
            let current_pos = local_backend.get_current_position().as_secs();
            let new_pos = current_pos + 10;
            let _ = local_backend.set_position(new_pos as usize);
            let _ = mpris_tx.try_send(MprisCommand::Seeked(new_pos));
        }
        PlayerCommand::SeekBackward => {
            let current_pos = local_backend.get_current_position().as_secs();
            let new_pos = current_pos.saturating_sub(10);
            let _ = local_backend.set_position(new_pos as usize);
            let _ = mpris_tx.try_send(MprisCommand::Seeked(new_pos));
        }
        PlayerCommand::SetVolume(vol) => {
            local_backend.set_volume(*vol);
        }
        PlayerCommand::VolumeUp => {
            let vol = (local_backend.get_volume() + 0.05).clamp(0.0, 1.0);
            local_backend.set_volume(vol);
            if let Some(ui) = ui.upgrade() {
                ui.set_current_volume(vol);
            }
        }
        PlayerCommand::VolumeDown => {
            let vol = (local_backend.get_volume() - 0.05).clamp(0.0, 1.0);
            local_backend.set_volume(vol);
            if let Some(ui) = ui.upgrade() {
                ui.set_current_volume(vol);
            }
        }
        PlayerCommand::SelectPlaylist(i) => {
            let _ = local_backend.select_playlist(*i);
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::SelectPlaylistTrack(playlist_i, track_i) => {
            let _ = local_backend.select_playlist_track(*playlist_i, *track_i);
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::ToggleRepeat => {
            local_backend.toggle_repeat();
            let loop_status = repeat_mode_to_loop_status(local_backend.get_repeat());
            let _ = mpris_tx.try_send(MprisCommand::UpdateLoopStatus(loop_status));
        }
        PlayerCommand::ToggleShuffle => {
            local_backend.toggle_shuffle();
            let _ = mpris_tx.try_send(MprisCommand::UpdateShuffle(local_backend.is_shuffle()));
        }
        PlayerCommand::SetRepeat(mode) => {
            local_backend.set_repeat(*mode);
            let loop_status = repeat_mode_to_loop_status(local_backend.get_repeat());
            let _ = mpris_tx.try_send(MprisCommand::UpdateLoopStatus(loop_status));
        }
        PlayerCommand::SetShuffle(shuffle) => {
            local_backend.set_shuffle(*shuffle);
            let _ = mpris_tx.try_send(MprisCommand::UpdateShuffle(local_backend.is_shuffle()));
        }
        PlayerCommand::OpenPlaylist(i) => {
            open_playlist(*i, local_backend, ui);
        }
    }
}

/// Loads a playlist's tracklist with per-song art, decoding cover images
/// in parallel on a background task while preserving track order.
///
/// The UI is updated immediately with an empty playlist, then tracks are
/// streamed in as their art finishes decoding. Track order is preserved
/// by collecting all results, sorting by original index, then pushing
/// sequentially.
fn open_playlist(i: usize, local_backend: &LocalBackend, ui: &slint::Weak<AppWindow>) {
    let resolved = local_backend.resolve_playlist(i);
    let name = local_backend.playlists[i].name.clone();
    let track_count = resolved.len() as i32;
    let art_path = local_backend.playlists[i].art.clone();

    let ui_weak = ui.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui_instance) = ui_weak.upgrade() {
            ui_instance.set_viewing_playlist_index(i as i32);
            ui_instance.set_viewing_playlist(SlintPlaylist {
                name: name.into(),
                track_count,
                tracks: ModelRc::new(VecModel::<SlintSongWithArt>::from(Vec::new())),
                art: get_art_from_path(art_path),
            });
        }
    });

    let ui_weak = ui.clone();
    tokio::spawn(async move {
        use crate::utils::DecodedSong;
        use crate::utils::decode_song_with_art;
        use crate::utils::raw_art_to_slint_image;

        let handles: Vec<_> = resolved
            .into_iter()
            .enumerate()
            .map(|(idx, song)| {
                tokio::task::spawn(async move {
                    let decoded = tokio::task::spawn_blocking(move || decode_song_with_art(&song))
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

        for (_, decoded) in results {
            let ui_weak = ui_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui_instance) = ui_weak.upgrade() {
                    if ui_instance.get_viewing_playlist_index() != i as i32 {
                        return;
                    }

                    let slint_song = SlintSongWithArt {
                        title: decoded.title.into(),
                        artist: decoded.artist.into(),
                        art: raw_art_to_slint_image(&decoded.art),
                    };

                    let tracks = ui_instance.get_viewing_playlist().tracks;
                    if let Some(vec_model) =
                        tracks.as_any().downcast_ref::<VecModel<SlintSongWithArt>>()
                    {
                        vec_model.push(slint_song);
                    }
                }
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

/// Converts the backend's scanned library into the Slint-facing album
/// list, for populating the UI at startup.
fn build_slint_library(local_backend: &LocalBackend) -> Vec<SlintAlbum> {
    let mut library: Vec<SlintAlbum> = Vec::new();
    for album in &local_backend.library {
        let slint_album = album_rust_to_slint(album);
        library.push(slint_album);
    }
    library
}

/// Converts the backend's configured playlists into the Slint-facing
/// playlist list, for populating the UI at startup.
fn build_slint_playlists(local_backend: &LocalBackend) -> Vec<SlintPlaylist> {
    let mut playlists: Vec<SlintPlaylist> = Vec::new();
    for (i, _) in local_backend.playlists.iter().enumerate() {
        playlists.push(playlist_rust_to_slint(i, local_backend));
    }
    playlists
}
