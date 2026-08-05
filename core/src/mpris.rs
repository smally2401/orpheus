//! Integrates Orpheus with the OS-level MPRIS media control interface
//! (used by desktop environments for lock-screen controls, media keys,
//! notification widgets, etc.).
//!
//! Communicates with the rest of the app via two independent channels:
//! `MprisCommand`s flow in (app -> MPRIS, e.g. "track changed"), and
//! `PlayerCommand`s flow out (MPRIS -> app, e.g. "the OS media key was
//! pressed").

use crate::local_backend::RepeatMode;
use crate::player_bridge::PlayerCommand;
use async_executor::LocalExecutor;
use image::ImageFormat;
use mpris_server::LoopStatus;
use mpris_server::Metadata;
use mpris_server::Time;
use mpris_server::TrackId;
use std::fs;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::rc::Rc;
use tokio::sync::mpsc;

/// A state change to report to the OS MPRIS interface.
pub(crate) enum MprisCommand {
    UpdateStatus(mpris_server::PlaybackStatus),
    UpdateMetadata {
        title: String,
        artist: String,
        album: String,
        track_id: TrackId,
        length: u64,
        /// A `file://` URL pointing at cached cover art, if any.
        /// See `write_art_cache`.
        art_url: Option<String>,
    },
    UpdatePosition(u64),
    /// A user-initiated seek just completed, to be reported back to MPRIS
    /// listeners (distinct from `UpdatePosition`, which is the regular
    /// polling update: see the `Seeked` signal in the MPRIS spec).
    Seeked(u64),
    UpdateShuffle(bool),
    UpdateLoopStatus(LoopStatus),
}

/// Spawns the MPRIS server on its own OS thread and returns a channel to
/// send it `MprisCommand`s.
///
/// `app_tx` is used the other direction: MPRIS control events (play/pause,
/// next/previous, seek, volume: see `setup_controls`) are translated into
/// `PlayerCommand`s and sent back into the app's main command channel, so
/// OS-level media controls behave identically to in-app UI controls.
pub(crate) fn spawn_mpris(app_tx: mpsc::Sender<PlayerCommand>) -> mpsc::Sender<MprisCommand> {
    let (mpris_tx, mut mpris_rx) = mpsc::channel::<MprisCommand>(32);

    std::thread::spawn(move || {
        let local_ex = LocalExecutor::new();

        futures_lite::future::block_on(local_ex.run(async {
            let player = match mpris_server::Player::builder("orpheus")
                .can_play(true)
                .can_pause(true)
                .can_go_next(true)
                .can_go_previous(true)
                .can_seek(true)
                .build()
                .await
            {
                Ok(p) => Rc::new(p),
                Err(e) => {
                    eprintln!("Could not initialise MPRIS server: {e}");
                    return;
                }
            };

            setup_controls(&player, &app_tx);

            let player_run_loop = Rc::clone(&player);
            let run_task = local_ex.spawn(async move {
                player_run_loop.run().await;
            });
            run_task.detach();

            while let Some(cmd) = mpris_rx.recv().await {
                match cmd {
                    MprisCommand::UpdateStatus(status) => {
                        if let Err(e) = player.set_playback_status(status).await {
                            eprintln!("Failed to update playback status: {e}");
                        }
                    }
                    MprisCommand::UpdateMetadata {
                        title,
                        artist,
                        album,
                        track_id,
                        length,
                        art_url,
                    } => {
                        let mut builder = Metadata::builder()
                            .trackid(track_id)
                            .title(title)
                            .artist([artist])
                            .album(album)
                            .length(Time::from_secs(length as i64));

                        if let Some(url) = art_url {
                            builder = builder.art_url(url);
                        }

                        let metadata = builder.build();
                        if let Err(e) = player.set_metadata(metadata).await {
                            eprintln!("Failed to update metadata: {e}");
                        }
                    }
                    MprisCommand::UpdatePosition(pos) => {
                        player.set_position(Time::from_secs(pos as i64));
                    }
                    MprisCommand::Seeked(pos) => {
                        if let Err(e) = player.seeked(Time::from_secs(pos as i64)).await {
                            eprintln!("Failed to emit seeked signal: {e}");
                        }
                    }
                    MprisCommand::UpdateShuffle(shuffle) => {
                        if let Err(e) = player.set_shuffle(shuffle).await {
                            eprintln!("Failed to update shuffle: {e}");
                        }
                    }
                    MprisCommand::UpdateLoopStatus(loop_status) => {
                        if let Err(e) = player.set_loop_status(loop_status).await {
                            eprintln!("Failed to update loop status: {e}");
                        }
                    }
                }
            }
        }));
    });

    mpris_tx
}

/// Wires up MPRIS control events (play/pause, next/previous, seek, volume)
/// to `PlayerCommand`s sent back over `app_tx`, so OS-level media controls
/// (lock screen, media keys, etc.) drive the app the same way in-app
/// buttons do.
fn setup_controls(player: &Rc<mpris_server::Player>, app_tx: &mpsc::Sender<PlayerCommand>) {
    let app_tx_clone = app_tx.clone();
    player.connect_play_pause(move |_| {
        let _ = app_tx_clone.try_send(PlayerCommand::TogglePlay);
    });

    let app_tx_clone = app_tx.clone();
    player.connect_play(move |_| {
        let _ = app_tx_clone.try_send(PlayerCommand::Play);
    });

    let app_tx_clone = app_tx.clone();
    player.connect_pause(move |_| {
        let _ = app_tx_clone.try_send(PlayerCommand::Pause);
    });

    let app_tx_clone = app_tx.clone();
    player.connect_next(move |_| {
        let _ = app_tx_clone.try_send(PlayerCommand::NextTrack);
    });

    let app_tx_clone = app_tx.clone();
    player.connect_previous(move |_| {
        let _ = app_tx_clone.try_send(PlayerCommand::PrevTrack);
    });

    let app_tx_clone = app_tx.clone();
    player.connect_set_volume(move |_player, vol| {
        let _ = app_tx_clone.try_send(PlayerCommand::SetVolume(vol as f32));
    });

    let app_tx_clone = app_tx.clone();
    player.connect_set_position(move |_player, _track_id, pos| {
        let _ = app_tx_clone.try_send(PlayerCommand::SetPosition(pos.as_secs() as usize));
    });

    let app_tx_clone = app_tx.clone();
    player.connect_set_shuffle(move |_player, shuffle| {
        let _ = app_tx_clone.try_send(PlayerCommand::SetShuffle(shuffle));
    });

    let app_tx_clone = app_tx.clone();
    player.connect_set_loop_status(move |_player, loop_status| {
        let _ = app_tx_clone.try_send(PlayerCommand::SetRepeat(loop_status_to_repeat_mode(
            loop_status,
        )));
    });
}

/// Converts an MPRIS `LoopStatus` (from an incoming `connect_set_loop_status`
/// control) into our own `RepeatMode`, so the rest of the app only ever has
/// to deal with one repeat-mode type.
fn loop_status_to_repeat_mode(status: LoopStatus) -> RepeatMode {
    match status {
        LoopStatus::None => RepeatMode::Off,
        LoopStatus::Playlist => RepeatMode::Queue,
        LoopStatus::Track => RepeatMode::Track,
    }
}

/// The inverse of `loop_status_to_repeat_mode`, used when reporting our
/// current `RepeatMode` back out to MPRIS via `MprisCommand::UpdateLoopStatus`
/// (see `player_bridge.rs`).
pub(crate) fn repeat_mode_to_loop_status(mode: RepeatMode) -> LoopStatus {
    match mode {
        RepeatMode::Off => LoopStatus::None,
        RepeatMode::Track => LoopStatus::Track,
        RepeatMode::Queue => LoopStatus::Playlist,
    }
}

/// Hashes a song path into a stable `u64`, used as the basis for both
/// MPRIS track IDs (`track_id_for_path`) and cached art filenames
/// (`write_art_cache`), sharing this one hash keeps the two guaranteed to
/// agree on "which song is this" without any risk of drifting apart.
fn track_id_hash(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// Builds an MPRIS `TrackId` (a D-Bus object path) for a song, derived from
/// a hash of its path. Not a real D-Bus object, just a stable identifier
/// MPRIS clients use to distinguish tracks.
pub(crate) fn track_id_for_path(path: &Path) -> TrackId {
    let id = track_id_hash(path);
    TrackId::try_from(format!("/org/orpheus/track/{id}")).expect("valid object path")
}

// todo: evict old cache entries
/// Writes a song's embedded cover art to a per-user cache directory and
/// returns a `file://` URL pointing at it.
///
/// MPRIS metadata's `art_url` field expects a URL, not raw image bytes, so
/// embedded art (which only exists as bytes read from the audio file) has
/// to be written out to disk once before it can be reported. Returns
/// `None` if the cache directory can't be determined or written to,
/// rather than erroring.
pub(crate) fn write_art_cache(path: &Path, bytes: &[u8]) -> Option<String> {
    let cache_dir = dirs::cache_dir()?.join("orpheus").join("art");
    fs::create_dir_all(&cache_dir).ok()?;

    let ext = match image::guess_format(bytes) {
        Ok(ImageFormat::Png) => "png",
        Ok(ImageFormat::Jpeg) => "jpg",
        _ => "img",
    };

    let file_path = cache_dir.join(format!("{}.{ext}", track_id_hash(path)));
    fs::write(&file_path, bytes).ok()?;

    Some(format!("file://{}", file_path.display()))
}
