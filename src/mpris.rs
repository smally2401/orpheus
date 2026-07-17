use crate::player_bridge::PlayerCommand;
use async_executor::LocalExecutor;
use mpris_server::Metadata;
use mpris_server::Time;
use mpris_server::TrackId;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::rc::Rc;
use tokio::sync::mpsc;

pub enum MprisCommand {
    UpdateStatus(mpris_server::PlaybackStatus),
    UpdateMetadata {
        title: String,
        artist: String,
        album: String,
        track_id: TrackId,
        length: u64,
    },
    UpdatePosition(u64),
}

pub fn spawn_mpris(app_tx: mpsc::Sender<PlayerCommand>) -> mpsc::Sender<MprisCommand> {
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
                    } => {
                        let metadata = Metadata::builder()
                            .trackid(track_id)
                            .title(title)
                            .artist([artist])
                            .album(album)
                            .length(Time::from_secs(length as i64))
                            .build();
                        if let Err(e) = player.set_metadata(metadata).await {
                            eprintln!("Failed to update metadata: {e}");
                        }
                    }
                    MprisCommand::UpdatePosition(pos) => {
                        player.set_position(Time::from_secs(pos as i64));
                    }
                }
            }
        }));
    });

    mpris_tx
}

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
}

pub fn track_id_for_path(path: &Path) -> TrackId {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let id = hasher.finish();
    TrackId::try_from(format!("/org/orpheus/track/{id}")).expect("valid object path")
}
