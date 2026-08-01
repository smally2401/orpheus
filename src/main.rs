//! Application entry point: builds the UI, loads config, wires every
//! Slint UI callback to a `PlayerCommand`, and hands off to the Slint
//! event loop.
//!
//! This file deliberately contains no logic of its own, it just connects
//! UI events to `player_bridge`'s command channel. See `player_bridge.rs`
//! for what actyally happens when a command is sent.
//!
//! It also spawns one extra, non-Slint/non-tokio thread: a dedicated
//! script runtime thread that owns a `ScriptRuntime` (see
//! `config::scripting`) for the app's entire lifetime. This exists
//! because `mlua::Lua` is `!Send`, so it can never be built on one thread
//! and handed to another, the thread below builds its own `Lua` instance
//! via `build_runtime` and then just waits on a channel for song changes,
//! reported by `player_bridge`'s tick loop.

mod config;
mod local_backend;
mod macros;
mod mpris;
mod player_bridge;
mod utils;

use crate::config::WindowState;
use crate::config::keys::KeyAction;
use crate::config::keys::KeyCombo;
use crate::config::keys::key_string_to_key_name;
use crate::config::load_config;
use crate::config::scripting::ScriptEvent;
use crate::config::scripting::build_runtime;
use crate::config::theme::Theme;
use crate::player_bridge::PlayerCommand;
use crate::player_bridge::spawn_player_bridge;
use paste::paste;
use slint::LogicalSize;
use slint::ModelRc;
use slint::VecModel;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let (script_tx, script_rx) = std::sync::mpsc::channel::<ScriptEvent>();

    let (config, contents) = load_config();
    build_script_runtime_thread(contents, script_rx);

    let (tx, library, playlists) = spawn_player_bridge(
        &ui,
        &config.music_dir,
        config.playlists,
        config.default_volume,
        script_tx,
    );

    apply_window_config(&ui, &config.window_state);
    apply_theme(&ui, &config.theme);

    wire_callbacks(&ui, &tx);
    handle_keymaps(&config.keymaps, &ui, &tx);

    let library_model = ModelRc::new(VecModel::from(library));
    ui.set_albums(library_model);

    let playlists_model = ModelRc::new(VecModel::from(playlists));
    ui.set_playlists(playlists_model);

    ui.run()
}

/// Dedicated thread for `ScriptRuntime`: built here, on this thread,
/// rather than passed in, since `mlua::Lua` can't cross threads.
/// `fire_song_change` is called after receiving a message so a script's
/// `on_song_change` sees consistent state if it calls `current_song()`
/// itself.
fn build_script_runtime_thread(
    contents: String,
    script_rx: std::sync::mpsc::Receiver<ScriptEvent>,
) {
    std::thread::spawn(move || {
        let script_runtime = build_runtime(&contents);
        while let Ok(event) = script_rx.recv() {
            match event {
                ScriptEvent::SongChanged(song) => {
                    script_runtime.fire_song_change(song);
                }
                ScriptEvent::SongHalfway => {
                    script_runtime.fire_song_halfway();
                }
            }
        }
    });
}

/// Applies the user's window config.
fn apply_window_config(ui: &AppWindow, window_state: &WindowState) {
    if let (Some(width), Some(height)) = (window_state.width, window_state.height) {
        ui.window().set_size(LogicalSize::new(width, height));
    }
    ui.window().set_maximized(window_state.maximized);
}

fn handle_keymaps(
    keymaps: &HashMap<KeyCombo, KeyAction>,
    ui: &AppWindow,
    tx: &Sender<PlayerCommand>,
) {
    let keymaps = keymaps.clone();
    let tx_clone = tx.clone();
    let ui_weak = ui.as_weak();

    ui.on_key_pressed_event(move |key, ctrl, shift, alt| {
        let Some(key) = key_string_to_key_name(key) else {
            return;
        };

        let key_combo = KeyCombo {
            key,
            ctrl,
            shift,
            alt,
        };

        let Some(key_action) = keymaps.get(&key_combo) else {
            return;
        };

        match key_action {
            KeyAction::TogglePlay => {
                let _ = tx_clone.try_send(PlayerCommand::TogglePlay);
            }
            KeyAction::NextTrack => {
                let _ = tx_clone.try_send(PlayerCommand::NextTrack);
            }
            KeyAction::PrevTrack => {
                let _ = tx_clone.try_send(PlayerCommand::PrevTrack);
            }
            KeyAction::VolumeUp => {
                let _ = tx_clone.try_send(PlayerCommand::VolumeUp);
            }
            KeyAction::VolumeDown => {
                let _ = tx_clone.try_send(PlayerCommand::VolumeDown);
            }
            KeyAction::SeekForward => {
                let _ = tx_clone.try_send(PlayerCommand::SeekForward);
            }
            KeyAction::SeekBackward => {
                let _ = tx_clone.try_send(PlayerCommand::SeekBackward);
            }
            KeyAction::OpenLibrary => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_current_view(View::Library);
                }
            }
            KeyAction::OpenPlaylists => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_current_view(View::Playlists);
                }
            }
        }
    });
}

/// Applies the user's color theme to the UI.
fn apply_theme(ui: &AppWindow, theme: &Theme) {
    theme_apply! {ui, theme,
        sidebar, bg;
        now_playing_bar, bg;
        library_view, bg;
        album_view, bg;
        playlists_view, bg;
        open_playlist_view, bg;

        sidebar, text_color;
        now_playing_song, text_color;
        now_playing_artist, text_color;
        detail_view_header_title, text_color;
        detail_view_header_subtitle, text_color;
        library_list_title, text_color;
        library_list_subtitle, text_color;
        album_list_title, text_color;
        album_list_subtitle, text_color;

        sidebar, text_size;
        now_playing_song, text_size;
        now_playing_artist, text_size;
        detail_view_header_title, text_size;
        detail_view_header_subtitle, text_size;
        library_list_title, text_size;
        library_list_subtitle, text_size;
        album_list_title, text_size;
        album_list_subtitle, text_size;
    }
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
    let ui_weak = ui.as_weak();
    ui.on_volume_changed(move |volume| {
        let _ = tx_clone.try_send(PlayerCommand::SetVolume(volume));
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_volume(volume);
        }
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
