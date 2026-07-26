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

use crate::config::Theme;
use crate::config::load_config;
use crate::player_bridge::PlayerCommand;
use crate::player_bridge::spawn_player_bridge;
use slint::ModelRc;
use slint::VecModel;
use tokio::sync::mpsc::Sender;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let config = load_config();
    let (tx, library, playlists) = spawn_player_bridge(&ui, &config.music_dir, config.playlists);

    apply_theme(&ui, &config.theme);

    wire_callbacks(&ui, &tx);

    let library_model = ModelRc::new(VecModel::from(library));
    ui.set_albums(library_model);

    let playlists_model = ModelRc::new(VecModel::from(playlists));
    ui.set_playlists(playlists_model);

    ui.run()
}

/// Applies the user's color theme to the UI.
fn apply_theme(ui: &AppWindow, theme: &Theme) {
    ui.set_sidebar_bg(theme.bg.sidebar);
    ui.set_now_playing_bg(theme.bg.now_playing_bar);
    ui.set_library_view_bg(theme.bg.library_view);
    ui.set_album_view_bg(theme.bg.album_view);
    ui.set_playlists_view_bg(theme.bg.playlists_view);
    ui.set_open_playlist_view_bg(theme.bg.open_playlist_view);

    ui.set_now_playing_song_text_color(theme.text_color.now_playing_song);
    ui.set_now_playing_artist_text_color(theme.text_color.now_playing_artist);
    ui.set_detail_view_header_title_text_color(theme.text_color.detail_view_header_title);
    ui.set_detail_view_header_subtitle_text_color(theme.text_color.detail_view_header_subtitle);
    ui.set_library_list_title_text_color(theme.text_color.library_list_title);
    ui.set_library_list_subtitle_text_color(theme.text_color.library_list_subtitle);
    ui.set_album_list_title_text_color(theme.text_color.album_list_title);
    ui.set_album_list_subtitle_text_color(theme.text_color.album_list_subtitle);
}

/// Attaches every Slint UI callback to a `PlayerCommand` sent over `tx`.
///
/// Most callbacks are simple fire-and-forget: they clone `tx`, move it into
/// the closure, and `try_send` the corresponding command. Callbacks that
/// need to read UI state (e.g. which album is currently being viewed) take
/// a `Weak<AppWindows>` and `upgrade()` it inside the closure.
fn wire_callbacks(ui: &AppWindow, tx: &Sender<PlayerCommand>) {
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

    let tx_clone = tx.clone();
    ui.on_playlist_opened(move |playlist_index| {
        let _ = tx_clone.try_send(PlayerCommand::OpenPlaylist(playlist_index as usize));
    });
}
