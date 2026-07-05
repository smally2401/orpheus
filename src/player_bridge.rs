use std::time::Duration;
use tokio::sync::mpsc;
use slint::{ComponentHandle};

use crate::{ AppWindow, SlintAlbum };
use crate::local_backend::LocalBackend;
use crate::utils::{album_rust_to_slint};

pub enum PlayerCommand {
    TogglePlay,
    NextTrack,
    PrevTrack,
    SelectAlbum(usize),
    // ToggleShuffle,
}

pub fn spawn_player_bridge(ui: &AppWindow) -> (mpsc::Sender<PlayerCommand>, Vec<SlintAlbum>) {

    let mut local_backend = LocalBackend::new();
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
                        }
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {
                    let ui_weak_clone = ui.clone();
                    if let Some(track) = local_backend.get_current_song() {
                        let title = track.title.clone();
                        let artist = track.artist.clone();

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui_instance) = ui_weak_clone.upgrade() {
                                ui_instance.set_current_track_title(title.into());
                                ui_instance.set_current_artist(artist.into());
                            }
                        });

                    } else {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui_instance) = ui_weak_clone.upgrade() {
                                ui_instance.set_current_track_title("No song playing".into());
                                ui_instance.set_current_artist("---".into());
                            }
                        });
                    }
                }
            }
        }
    });

    (tx, library)
}
