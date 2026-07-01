use std::time::Duration;
use tokio::sync::mpsc;
use slint::ComponentHandle;

use crate::AppWindow;
use crate::mpd_backend::*;

pub enum PlayerCommand {
    TogglePlay,
    NextTrack,
    PrevTrack,
    ToggleShuffle,
}

pub fn spawn_player_bridge(ui: &AppWindow) -> mpsc::Sender<PlayerCommand>{

    let (tx, mut rx) = mpsc::channel::<PlayerCommand>(100);
    let ui = ui.as_weak();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        match command {
                            PlayerCommand::TogglePlay => { let _ = toggle_mpd_play(); },
                            PlayerCommand::NextTrack => { let _ = next_mpd_track(); },
                            PlayerCommand::PrevTrack => { let _ = prev_mpd_track(); },
                            PlayerCommand::ToggleShuffle => {  },
                        }
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {
                    if let Ok(track_info) = fetch_mpd_metadata() {
                        let ui_weak_clone = ui.clone();

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui_instance) = ui_weak_clone.upgrade() {
                                ui_instance.set_current_track_title(track_info.title.into());
                                ui_instance.set_current_artist(track_info.artist.into());
                            }
                        });
                    }
                }
            }
        }
    });

    tx
}
