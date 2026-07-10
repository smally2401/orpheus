// mod mpd_backend;
mod local_backend;
mod player_bridge;
mod utils;

use slint::{ModelRc, VecModel};

use crate::player_bridge::PlayerCommand;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = AppWindow::new()?;
    let (tx, library) = player_bridge::spawn_player_bridge(&ui);
    
    let tx_clone = tx.clone();
    ui.on_play_paused_clicked(move || {
        let tx = tx_clone.clone();
        tokio::spawn(async move {
            let _ = tx.send(PlayerCommand::TogglePlay).await;
        });
    });

    let tx_clone = tx.clone();
    ui.on_next_track_clicked(move || {
        let tx = tx_clone.clone();
        tokio::spawn(async move {
            let _ = tx.send(PlayerCommand::NextTrack).await;
        });
    });

    let tx_clone = tx.clone();
    ui.on_prev_track_clicked(move || {
        let tx = tx_clone.clone();
        tokio::spawn(async move {
            let _ = tx.send(PlayerCommand::PrevTrack).await;
        });
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_album_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let album_index = ui.get_viewing_album_index() as usize;
            let tx = tx_clone.clone();
            tokio::spawn(async move {
                let _ = tx.send(PlayerCommand::SelectAlbum(album_index)).await;
            });
        }
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_track_clicked(move |song_index| {
        if let Some(ui) = ui_weak.upgrade() {
            let album_index = ui.get_viewing_album_index() as usize;
            let tx = tx_clone.clone();
            tokio::spawn(async move {
                let _ = tx.send(PlayerCommand::SelectTrack(album_index, song_index as usize)).await;
            });
        }
    });

    let tx_clone = tx.clone();
    ui.on_seek_requested(move |value| {
        let tx = tx_clone.clone();
        tokio::spawn(async move {
            let _ = tx.send(PlayerCommand::Seek(value as usize)).await;
        });
    });

    let model = ModelRc::new(VecModel::from(library));
    ui.set_albums(model);

    ui.run()
}
