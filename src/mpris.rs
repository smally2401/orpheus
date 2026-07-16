use crate::player_bridge::PlayerCommand;
use async_executor::LocalExecutor;
use std::rc::Rc;
use tokio::sync::mpsc;

pub enum MprisCommand {
    UpdateStatus(mpris_server::PlaybackStatus),
    // todo: metadata
}

pub fn spawn_mpris(app_tx: mpsc::Sender<PlayerCommand>) -> mpsc::Sender<MprisCommand> {
    let (mpris_tx, mut mpris_rx) = mpsc::channel::<MprisCommand>(32);

    std::thread::spawn(move || {
        let local_ex = LocalExecutor::new();

        futures_lite::future::block_on(local_ex.run(async {
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

            let app_tx_clone = app_tx.clone();
            player.connect_play(move |_| {
                let _ = app_tx_clone.try_send(PlayerCommand::Play);
            });

            let app_tx_clone = app_tx.clone();
            player.connect_pause(move |_| {
                let _ = app_tx_clone.try_send(PlayerCommand::Pause);
            });

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
                }
            }
        }));
    });

    mpris_tx
}
