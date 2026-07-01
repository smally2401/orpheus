use std::time::Duration;
use tokio::sync::mpsc;
use slint::ComponentHandle;

use crate::AppWindow;

pub enum PlayerCommand {
    TogglePlay,
    NextTrack,
    PrevTrack,
    ToggleShuffle,
}

pub fn spawn_player_bridge(ui: &AppWindow) -> mpsc::Sender<PlayerCommand>{

    let (mut tx, mut rx) = mpsc::channel::<PlayerCommand>(100);
    let ui = ui.as_weak();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {

                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        match command {
                            PlayerCommand::TogglePlay => {  },
                            PlayerCommand::NextTrack => {  },
                            PlayerCommand::PrevTrack => {  },
                            PlayerCommand::ToggleShuffle => {  },
                        }
                    } else {
                        break;
                    }
                }

                _ = interval.tick() => {

                }
            }
        }
    });

    tx
}
