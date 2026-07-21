//! Application entry point: builds the UI, loads config, wires every
//! Slint UI callback to a `PlayerCommand`, and hands off to the Slint
//! event loop.
//!
//! This file deliberately contains no logic of its own, it just connects
//! UI events to `player_bridge`'s command channel. See `player_bridge.rs`
//! for what actyally happens when a command is sent.

mod config;
mod local_backend;
mod mpris;
mod player_bridge;
mod utils;

use crate::player_bridge::PlayerCommand;
use slint::ModelRc;
use slint::VecModel;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let config = config::load_config();
    let (tx, library, playlists) =
        player_bridge::spawn_player_bridge(&ui, &config.music_dir, config.playlists);

    ui.set_sidebar_bg(config.sidebar_bg);
    ui.set_now_playing_bg(config.now_playing_bar_bg);
    ui.set_library_view_bg(config.library_view_bg);
    ui.set_album_view_bg(config.album_view_bg);
    ui.set_playlists_view_bg(config.playlists_view_bg);
    ui.set_open_playlist_view_bg(config.open_playlist_view_bg);

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
        let _ = tx_clone.try_send(PlayerCommand::SetPosition(value as usize));
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
            let _ = tx_clone.try_send(PlayerCommand::SelectPlaylistTrack(
                playlist_index,
                song_index as usize,
            ));
        }
    });

    let tx_clone = tx.clone();
    ui.on_shuffle_clicked(move || {
        let _ = tx_clone.try_send(PlayerCommand::ToggleShuffle);
    });

    let tx_clone = tx.clone();
    ui.on_repeat_clicked(move || {
        let _ = tx_clone.try_send(PlayerCommand::ToggleRepeat);
    });

    let library_model = ModelRc::new(VecModel::from(library));
    ui.set_albums(library_model);

    let playlists_model = ModelRc::new(VecModel::from(playlists));
    ui.set_playlists(playlists_model);

    ui.run()
}
