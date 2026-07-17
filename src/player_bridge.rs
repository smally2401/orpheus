use crate::AppWindow;
use crate::SlintAlbum;
use crate::SlintPlaylist;
use crate::local_backend::LocalBackend;
use crate::mpris::MprisCommand;
use crate::mpris::spawn_mpris;
use crate::mpris::track_id_for_path;
use crate::mpris::write_art_cache;
use crate::utils::album_rust_to_slint;
use crate::utils::art_rust_to_slint;
use crate::utils::playlist_rust_to_slint;
use crate::utils::expand_tilde;
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

#[allow(clippy::too_many_lines)]
pub fn spawn_player_bridge(
    ui: &AppWindow,
    music_dir: &str
) -> (
    mpsc::Sender<PlayerCommand>,
    Vec<SlintAlbum>,
    Vec<SlintPlaylist>,
) {
    let path = expand_tilde(&music_dir);

    if !path.exists() {
        eprintln!("error: path {} could not be found", path.to_string_lossy());
    }

    let mut local_backend = LocalBackend::new(&path);
    let (tx, mut rx) = mpsc::channel::<PlayerCommand>(100);
    let ui = ui.as_weak();

    let mut library: Vec<SlintAlbum> = Vec::new();
    for album in &local_backend.library {
        let slint_album = album_rust_to_slint(album);
        library.push(slint_album);
    }

    let mut playlists: Vec<SlintPlaylist> = Vec::new();
    for (i, _) in local_backend.playlists.iter().enumerate() {
        playlists.push(playlist_rust_to_slint(i, &local_backend));
    }

    let mpris_tx = spawn_mpris(tx.clone());

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut last_song_info: Option<(String, String)> = None;
        let mut last_song_path: Option<PathBuf> = None;
        let mut was_paused = true;
        let mut queue_exhausted = false;

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        match command {
                            // todo: remove let _ and handle stuff
                            PlayerCommand::TogglePlay => { local_backend.toggle_play(); },
                            PlayerCommand::Play => { local_backend.play(); },
                            PlayerCommand::Pause => { local_backend.pause(); },
                            PlayerCommand::NextTrack => { let _ = local_backend.next(); },
                            PlayerCommand::PrevTrack => {
                                let _ = local_backend.prev();
                                queue_exhausted = false;
                            },
                            PlayerCommand::SelectAlbum(i) => {
                                let _ = local_backend.select_album(i);
                                queue_exhausted = false;
                            },
                            PlayerCommand::SelectTrack(album_i, track_i) => {
                                let _ = local_backend.select_album_track(album_i, track_i);
                                queue_exhausted = false;
                            },
                            PlayerCommand::SetPosition(dur) => {
                                let _ = local_backend.set_position(dur);
                                let _ = mpris_tx.try_send(MprisCommand::Seeked(dur as u64));
                            },
                            PlayerCommand::SetVolume(vol) => { local_backend.set_volume(vol); },
                            PlayerCommand::SelectPlaylist(i) => {
                                let _ = local_backend.select_playlist(i);
                                queue_exhausted = false;
                            },
                            PlayerCommand::SelectPlaylistTrack(playlist_i, track_i) => {
                                let _ = local_backend.select_playlist_track(playlist_i, track_i);
                                queue_exhausted = false;
                            },
                        }
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {

                    if local_backend.track_finished() && !queue_exhausted {
                        // todo: include loop back to track 0 option
                        queue_exhausted = !local_backend.next().unwrap_or(false);
                    }

                    let is_paused = local_backend.is_paused();
                    if is_paused != was_paused {
                        was_paused = is_paused;
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
                        let new_art_bytes = if last_song_info.as_ref() == Some(&current_album) {
                            None
                        } else {
                            last_song_info = Some(current_album);
                            track.art.clone()
                        };

                        let current_song_path = track.path.clone();
                        if last_song_path != Some(current_song_path) {
                            last_song_path = Some(track.path.clone());

                            let art_url = track.art
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
                        last_song_info = None;
                        last_song_path = None;
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
        }
    });

    (tx, library, playlists)
}
