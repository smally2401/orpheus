use crate::player_bridge::PlayerCommand;
use std::rc::Rc;
use tokio::sync::mpsc;

pub enum MprisCommand {
    UpdateStatus(mpris_server::PlaybackStatus),
    // todo: metadata
}

pub fn spawn_mpris(app_tx: mpsc::Sender<PlayerCommand>) -> mpsc::Sender<MprisCommand> {
    let (mpris_tx, mut mpris_rx) = mpsc::channel::<MprisCommand>(32);

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                eprintln!("Failed to build tokio runtime: {e}");
                return;
            }
        };

        let local = tokio::task::LocalSet::new();

        local.block_on(&rt, async move {
            let player = match mpris_server::Player::builder("orpheus")
                .can_play(true)
                .can_pause(true)
                .build()
                .await
            {
                Ok(p) => Rc::new(p),
                Err(e) => {
                    eprintln!("Could not initialise MPRIS server: {e}");
                    return;
                }
            };

            let app_tx_clone = app_tx.clone();
            player.connect_play_pause(move |_| {
                let _ = app_tx_clone.try_send(PlayerCommand::TogglePlay);
            });

            let player_run_loop = Rc::clone(&player);
            tokio::task::spawn_local(async move {
                player_run_loop.run().await;
            });

            while let Some(cmd) = mpris_rx.recv().await {
                match cmd {
                    MprisCommand::UpdateStatus(status) => {
                        if let Err(e) = player.set_playback_status(status).await {
                            eprintln!("Failed to update playback status: {e}");
                        }
                    }
                }
            }
        });
    });

    mpris_tx
}
