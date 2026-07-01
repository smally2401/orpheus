mod mpd_backend;
mod local_backend;
mod player_bridge;

use crate::player_bridge::PlayerCommand;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = AppWindow::new()?;
    let tx = player_bridge::spawn_player_bridge(&ui);
    
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

    ui.run()
}
