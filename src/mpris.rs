use crate::player_bridge::PlayerCommand;
use async_executor::LocalExecutor;
use image::ImageFormat;
use mpris_server::Metadata;
use mpris_server::Time;
use mpris_server::TrackId;
use std::fs;
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
        art_url: Option<String>,
    },
    UpdatePosition(u64),
    Seeked(u64),
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
                        art_url,
                    } => {
                        let mut builder = Metadata::builder()
                            .trackid(track_id)
                            .title(title)
                            .artist([artist])
                            .album(album)
                            .length(Time::from_secs(length as i64));

                        if let Some(url) = art_url {
                            builder = builder.art_url(url);
                        }

                        let metadata = builder.build();
                        if let Err(e) = player.set_metadata(metadata).await {
                            eprintln!("Failed to update metadata: {e}");
                        }
                    }
                    MprisCommand::UpdatePosition(pos) => {
                        player.set_position(Time::from_secs(pos as i64));
                    }
                    MprisCommand::Seeked(pos) => {
                        if let Err(e) = player.seeked(Time::from_secs(pos as i64)).await {
                            eprintln!("Failed to emit seeked signal: {e}");
                        }
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

fn track_id_hash(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

pub fn track_id_for_path(path: &Path) -> TrackId {
    let id = track_id_hash(path);
    TrackId::try_from(format!("/org/orpheus/track/{id}")).expect("valid object path")
}

pub fn write_art_cache(path: &Path, bytes: &[u8]) -> Option<String> {
    let cache_dir = dirs::cache_dir()?.join("orpheus").join("art");
    fs::create_dir_all(&cache_dir).ok()?;

    let ext = match image::guess_format(bytes) {
        Ok(ImageFormat::Png) => "png",
        Ok(ImageFormat::Jpeg) => "jpg",
        _ => "img",
    };

    let file_path = cache_dir.join(format!("{}.{ext}", track_id_hash(path)));
    fs::write(&file_path, bytes).ok()?;

    Some(format!("file://{}", file_path.display()))
}
