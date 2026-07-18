use crate::AppWindow;
use crate::SlintAlbum;
use crate::SlintPlaylist;
use crate::config::PlaylistDef;
use crate::local_backend::LocalBackend;
use crate::mpris::MprisCommand;
use crate::mpris::spawn_mpris;
use crate::mpris::track_id_for_path;
use crate::mpris::write_art_cache;
use crate::utils::album_rust_to_slint;
use crate::utils::art_rust_to_slint;
use crate::utils::expand_tilde;
use crate::utils::playlist_rust_to_slint;
use mpris_server::PlaybackStatus;
use slint::ComponentHandle;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

pub enum PlayerCommand {
    TogglePlay,
    Play,
    Pause,
    NextTrack,
    PrevTrack,
    SelectAlbum(usize),
    SelectTrack(usize, usize),
    SetPosition(usize),
    SetVolume(f32),
    SelectPlaylist(usize),
    SelectPlaylistTrack(usize, usize),
    // ToggleShuffle,
}

struct TickState {
    last_song_info: Option<(String, String)>,
    last_song_path: Option<PathBuf>,
    was_paused: bool,
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

    fn on_tick(
        &mut self,
        local_backend: &mut LocalBackend,
        mpris_tx: &mpsc::Sender<MprisCommand>,
        ui: &slint::Weak<AppWindow>,
    ) {
        if local_backend.track_finished() && !self.queue_exhausted {
            // todo: include loop back to track 0 option
            self.queue_exhausted = !local_backend.next().unwrap_or(false);
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
            let new_art_bytes = if self.last_song_info.as_ref() == Some(&current_album) {
                None
            } else {
                self.last_song_info = Some(current_album);
                track.art.clone()
            };

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

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui_instance) = ui_weak_clone.upgrade() {
                    ui_instance.set_current_track_title(title.into());
                    ui_instance.set_current_artist(artist.into());
                    ui_instance.set_current_position(current_position as i32);
                    ui_instance.set_total_duration(total_duration as i32);
                    if let Some(bytes) = new_art_bytes {
                        ui_instance.set_current_art(art_rust_to_slint(Some(bytes.as_slice())));
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
                    ui_instance.set_current_art(slint::Image::default());
                }
            });
        }
    }
}

pub fn spawn_player_bridge(
    ui: &AppWindow,
    music_dir: &str,
    playlist_defs: Vec<PlaylistDef>,
) -> (
    mpsc::Sender<PlayerCommand>,
    Vec<SlintAlbum>,
    Vec<SlintPlaylist>,
) {
    let path = setup_music_dir(music_dir);
    let mut local_backend = LocalBackend::new(&path, playlist_defs);
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
                        handle_command(&command, &mut local_backend, &mpris_tx, &mut tick_state);
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

fn handle_command(
    command: &PlayerCommand,
    local_backend: &mut LocalBackend,
    mpris_tx: &mpsc::Sender<MprisCommand>,
    tick_state: &mut TickState,
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
            let _ = local_backend.next();
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
        PlayerCommand::SetVolume(vol) => {
            local_backend.set_volume(*vol);
        }
        PlayerCommand::SelectPlaylist(i) => {
            let _ = local_backend.select_playlist(*i);
            tick_state.queue_exhausted = false;
        }
        PlayerCommand::SelectPlaylistTrack(playlist_i, track_i) => {
            let _ = local_backend.select_playlist_track(*playlist_i, *track_i);
            tick_state.queue_exhausted = false;
        }
    }
}

fn setup_music_dir(music_dir: &str) -> PathBuf {
    let path = expand_tilde(music_dir);
    if !path.exists() {
        eprintln!("error: path {} could not be found", path.to_string_lossy());
    }
    path
}

fn build_slint_library(local_backend: &LocalBackend) -> Vec<SlintAlbum> {
    let mut library: Vec<SlintAlbum> = Vec::new();
    for album in &local_backend.library {
        let slint_album = album_rust_to_slint(album);
        library.push(slint_album);
    }
    library
}

fn build_slint_playlists(local_backend: &LocalBackend) -> Vec<SlintPlaylist> {
    let mut playlists: Vec<SlintPlaylist> = Vec::new();
    for (i, _) in local_backend.playlists.iter().enumerate() {
        playlists.push(playlist_rust_to_slint(i, local_backend));
    }
    playlists
}
