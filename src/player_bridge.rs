use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use slint::{ComponentHandle};

use crate::{ AppWindow, SlintAlbum };
use crate::local_backend::LocalBackend;
use crate::utils::{album_rust_to_slint, art_rust_to_slint};

pub enum PlayerCommand {
    TogglePlay,
    NextTrack,
    PrevTrack,
    SelectAlbum(usize),
    SelectTrack(usize, usize),
    Seek(usize),
    SetVolume(f32),
    // ToggleShuffle,
}

pub fn spawn_player_bridge(ui: &AppWindow) -> (mpsc::Sender<PlayerCommand>, Vec<SlintAlbum>) {

    let raw = std::env::var("ORPHEUS_MUSIC_DIR").ok();
    let path = raw.map(|s| {
        if let Some(stripped) = s.strip_prefix("~/") {
            dirs::home_dir()
                .map(|home| home.join(stripped))
                .unwrap_or_else(|| PathBuf::from(s.clone()))
        } else {
            PathBuf::from(s)
        }
    }).unwrap_or_else(|| PathBuf::from("."));

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

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        match command {
                            // todo: remove let _ and handle stuff
                            PlayerCommand::TogglePlay => { local_backend.toggle_play(); },
                            PlayerCommand::NextTrack => { let _ = local_backend.next(); },
                            PlayerCommand::PrevTrack => { let _ = local_backend.prev(); },
                            PlayerCommand::SelectAlbum(i) => { let _ = local_backend.select_album(i); },
                            PlayerCommand::SelectTrack(album_i, track_i) => { let _ = local_backend.select_track(album_i, track_i); },
                            PlayerCommand::Seek(dur) => { let _ = local_backend.seek(dur); },
                            PlayerCommand::SetVolume(vol) => { local_backend.set_volume(vol); },
                        }
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {
                    if local_backend.track_finished() {
                        // todo: include loop back to track 0 option
                        let _ = local_backend.next();
                    }

                    let ui_weak_clone = ui.clone();
                    if let Some(track) = local_backend.get_current_song() {
                        let title = track.title.clone();
                        let artist = track.artist.clone();
                        let current_position = local_backend.get_current_position().as_secs();
                        let total_duration = track.duration.as_secs();
                        let album_art_bytes = local_backend.get_current_album_art().map(|b| b.to_vec());

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui_instance) = ui_weak_clone.upgrade() {
                                ui_instance.set_current_track_title(title.into());
                                ui_instance.set_current_artist(artist.into());
                                ui_instance.set_current_position(current_position as i32);
                                ui_instance.set_total_duration(total_duration as i32);
                                ui_instance.set_current_art(art_rust_to_slint(album_art_bytes.as_deref()));
                            }
                        });

                    } else {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui_instance) = ui_weak_clone.upgrade() {
                                ui_instance.set_current_track_title("No song playing".into());
                                ui_instance.set_current_artist("---".into());
                                ui_instance.set_current_position(0);
                                ui_instance.set_total_duration(0);
                            }
                        });
                    }
                }
            }
        }
    });

    (tx, library)
}
