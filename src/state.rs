//! Persists playback state (queue, position, shuffle/repeat, volume)
//! across app restarts, so closing and repoening Orpheus resumes roughly
//! where you left off. State is saved on `LocalBackend`'s `Drop (see
//! `local_backend.rs`) and restored once at startup, right after
//! `LocalBackend::new` (see `spawn_player_bridge` in `player_bridge.rs`).

use crate::local_backend::RepeatMode;
use crate::local_backend::LocalBackend;
use crate::local_backend::Song;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::path::Path;
use std::sync::Arc;

/// The subset of `LocalBackend`'s state that gets serialized to disk.
/// Stores songs paths rather than resolved `Song`s (same reasoning as
/// `Playlist.songs`): the library may have changed since last save, so
/// restoring re-resolves each path against the current library instead
/// of trusting stale data.
#[derive(Serialize, Deserialize)]
struct PlaybackState {
    queue: Vec<PathBuf>,
    /// Position within `queue` (not `order`) of the track that was
    /// playing when state was saved. Unlike `LocalBackend.index`, this is
    /// already resolved to a real queue position, so restoring doesn't
    /// need to know what `order` looked like.
    index: usize,
    /// Playback position within the current track, in seconds.
    position: u64,
    shuffle: bool,
    repeat: RepeatMode,
    volume: f32,
}

/// Where playback state is saved: `<cache_dir>/orpheus/state.json`.
/// Returns `None` if the platform has no cache directory, in which case
/// state simply isn't persisted.
fn get_state_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("orpheus").join("state.json"))
}

/// Serializes `state` to `path` as pretty-printed JSON, creating the
/// parent directory first if it doesn't exist.
fn save_state_to_disk(
    path: &Path,
    state: &PlaybackState,
) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(state)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)?;
    Ok(())
}

/// Reads and deserializes `PlaybackState` from `path`. Returns `None` on
/// any failure (missing file, invalid JSON, schema mismatch from an older
/// version), rather than erroring: a corrupt or absent state file should
/// never prevent Orpheus from starting.
fn load_state_from_disk(path: &Path) -> Option<PlaybackState> {
    let json = fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Saves `local_backend`'s current playback state to disk. Called from
/// `LocalBackend`'s `Drop` impl, so this runs once, on app shutdown.
///
/// If the queue is empty (nothing was ever played this session), any
/// existing saved state is deleted instead of overwritten with an empty
/// one: en empty queue isn't something worth restoring into, and this
/// voids resuming into a queue if a queue was cleared before the
/// last session ended.
pub(crate) fn save_playback_state(local_backend: &LocalBackend) {
    if local_backend.queue.is_empty() {
        if let Some(path) = get_state_path() {
            let _ = fs::remove_file(&path);
        }
        return;
    }

    let state = PlaybackState {
        queue: local_backend.queue.iter().map(|s| s.path.clone()).collect(),
        index: local_backend
            .order
            .get(local_backend.index)
            .copied()
            .unwrap_or(0),
        position: local_backend.get_current_position().as_secs(),
        shuffle: local_backend.is_shuffle(),
        repeat: local_backend.get_repeat(),
        volume: local_backend.get_volume(),
    };

    if let Some(path) = get_state_path()
        && let Err(e) = save_state_to_disk(&path, &state)
    {
        eprintln!("Failed to save playback state: {e}");
    }
}

/// Restores previously saved playback state into `backend`, if any
/// exists. Called once at startup, right after `LocalBackend::new`.
///
/// Each saved song path is re-resolved against the current library via
/// `find_song_by_path`: any that no longer exist (deleted. moved. or the
/// library changed since last save) are silently dropped rather than
/// failing the entire restore. If every song failed to resolve, restoring
/// is abandoned entirely and playback starts fresh, same as if no saved
/// state existed.
///
/// The restored track is loaded and immediately paused (rather than
/// resuming playback automatically) at the saved position and volume, so
/// reopening Orpheus doesn't unexpectedly start audio playing.
pub(crate) fn restore_state(backend: &mut LocalBackend) {
    let Some(state_path) = get_state_path() else {
        return;
    };
    let Some(state) = load_state_from_disk(&state_path) else {
        return;
    };

    let restored_queue: Vec<Arc<Song>> = state
        .queue
        .iter()
        .filter_map(|path| backend.find_song_by_path(path))
        .cloned()
        .collect();

    if restored_queue.is_empty() {
        return;
    }

    backend.queue = restored_queue;
    backend.order = (0..backend.queue.len()).collect();
    backend.index = state.index.min(backend.queue.len().saturating_sub(1));
    backend.set_shuffle(state.shuffle);
    backend.set_repeat(state.repeat);
    backend.set_volume(0.0);

    if state.shuffle {
        let mut rng = rand::rng();
        backend.order.shuffle(&mut rng);
        let new_pos = backend
            .order
            .iter()
            .position(|&x| x == backend.index)
            .unwrap_or(0);
        backend.order.swap(0, new_pos);
        backend.index = 0;
    }

    if let Err(e) = backend.load_track() {
        eprintln!("Failed to restore track: {e}");
        backend.set_volume(state.volume);
        return;
    }

    let _ = backend.set_position(state.position as usize);
    backend.pause();
    backend.set_volume(state.volume);
}
