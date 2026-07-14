// mod mpd_backend;
mod local_backend;
mod player_bridge;
mod utils;
mod config;

use slint::{ModelRc, VecModel};

use crate::player_bridge::PlayerCommand;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let config = config::load_config();
    println!("{:?}", config);

    let ui = AppWindow::new()?;
    let (tx, library, playlists) = player_bridge::spawn_player_bridge(&ui);
    
    let tx_clone = tx.clone();
    ui.on_play_paused_clicked(move || {
        let _ = tx_clone.try_send(PlayerCommand::TogglePlay);
    });

    let tx_clone = tx.clone();
    ui.on_next_track_clicked(move || {
        let _ = tx_clone.try_send(PlayerCommand::NextTrack);
    });

    let tx_clone = tx.clone();
    ui.on_prev_track_clicked(move || {
        let _ = tx_clone.try_send(PlayerCommand::PrevTrack);
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_album_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let album_index = ui.get_viewing_album_index() as usize;
            let _ = tx_clone.try_send(PlayerCommand::SelectAlbum(album_index));
        }
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_track_clicked(move |song_index| {
        if let Some(ui) = ui_weak.upgrade() {
            let album_index = ui.get_viewing_album_index() as usize;
            let _ = tx_clone.try_send(PlayerCommand::SelectTrack(album_index, song_index as usize));
        }
    });

    let tx_clone = tx.clone();
    ui.on_seek_requested(move |value| {
        let _ = tx_clone.try_send(PlayerCommand::Seek(value as usize));
    });

    let tx_clone = tx.clone();
    ui.on_volume_changed(move |volume| {
        let _ = tx_clone.try_send(PlayerCommand::SetVolume(volume));
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_playlist_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let playlist_index = ui.get_viewing_playlist_index() as usize;
            let _ = tx_clone.try_send(PlayerCommand::SelectPlaylist(playlist_index));
        }
    });

    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_play_playlist_track_clicked(move |song_index| {
        if let Some(ui) = ui_weak.upgrade() {
            let playlist_index = ui.get_viewing_playlist_index() as usize;
            let _ = tx_clone.try_send(PlayerCommand::SelectPlaylistTrack(playlist_index, song_index as usize));
        }
    });

    let library_model = ModelRc::new(VecModel::from(library));
    ui.set_albums(library_model);

    let playlists_model = ModelRc::new(VecModel::from(playlists));
    ui.set_playlists(playlists_model);

    ui.run()
}
