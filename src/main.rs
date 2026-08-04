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

#![windows_subsystem = "windows"]

mod audio_player;
mod config;
mod local_backend;
mod macros;
mod mpris;
mod player_bridge;
mod state;
mod utils;

use crate::config::WindowState;
use crate::config::keys::KeyAction;
use crate::config::keys::KeyCombo;
use crate::config::keys::key_string_to_key_name;
use crate::config::load_config;
use crate::config::scripting::ScriptEvent;
use crate::config::scripting::build_runtime;
use crate::config::theme::Theme;
use crate::config::theme::set_property;
use crate::player_bridge::PlayerCommand;
use crate::player_bridge::spawn_player_bridge;
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

    let (tx, library, playlists) = spawn_player_bridge(
        &ui,
        &config.music_dir,
        config.playlists,
        config.default_volume,
        script_tx,
    );

    build_script_runtime_thread(contents, script_rx, tx.clone());

    apply_window_config(&ui, &config.window_state);
    apply_theme(&ui, &config.theme);
    ui.set_current_volume(config.default_volume);

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
    tx: tokio::sync::mpsc::Sender<PlayerCommand>,
) {
    std::thread::spawn(move || {
        let script_runtime = build_runtime(&contents, tx);
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
    use KeyAction::*;

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

        if let Some(command) = key_action.to_command() {
            let _ = tx_clone.try_send(command);
            return;
        }

        match key_action {
            OpenLibrary => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_current_view(View::Library);
                }
            }

            OpenPlaylists => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_current_view(View::Playlists);
                }
            }

            _ => {}
        }
    });
}

/// Applies the user's color theme to the UI.
fn apply_theme(ui: &AppWindow, theme: &Theme) {
    for (element, property) in theme.properties() {
        set_property(ui, element, &property);
    }
}

/// Attaches every Slint UI callback to a `PlayerCommand` sent over `tx`.
///
/// Most callbacks are simple fire-and-forget: they clone `tx`, move it into
/// the closure, and `try_send` the corresponding command. Callbacks that
/// need to read UI state (e.g. which album is currently being viewed) take
/// a `Weak<AppWindows>` and `upgrade()` it inside the closure.
fn wire_callbacks(ui: &AppWindow, tx: &Sender<PlayerCommand>) {
    use crate::player_bridge::PlayerCommand::*;

    ui.on_play_paused_clicked(wire_cmd(tx, TogglePlay));
    ui.on_next_track_clicked(wire_cmd(tx, NextTrack));
    ui.on_prev_track_clicked(wire_cmd(tx, PrevTrack));

    ui.on_play_album_clicked(wire_cmd_ui(ui, tx, |ui, tx| {
        let album_index = ui.get_viewing_album_index() as usize;
        let _ = tx.try_send(SelectAlbum(album_index));
    }));

    ui.on_play_track_clicked(wire_cmd_ui_1arg(ui, tx, |ui, tx, song_index| {
        let album_index = ui.get_viewing_album_index() as usize;
        let _ = tx.try_send(SelectTrack(album_index, song_index as usize));
    }));

    ui.on_seek_requested(wire_cmd_1arg(tx, |value| SetPosition(value as usize)));

    ui.on_volume_changed(wire_cmd_ui_1arg(ui, tx, |ui, tx, volume| {
        let _ = tx.try_send(SetVolume(volume));
        ui.set_current_volume(volume);
    }));

    ui.on_play_playlist_clicked(wire_cmd_ui(ui, tx, |ui, tx| {
        let playlist_index = ui.get_viewing_playlist_index() as usize;
        let _ = tx.try_send(SelectPlaylist(playlist_index));
    }));

    ui.on_play_playlist_track_clicked(wire_cmd_ui_1arg(ui, tx, |ui, tx, song_index| {
        let playlist_index = ui.get_viewing_playlist_index() as usize;
        let _ = tx.try_send(SelectPlaylistTrack(playlist_index, song_index as usize));
    }));

    ui.on_shuffle_clicked(wire_cmd(tx, ToggleShuffle));
    ui.on_repeat_clicked(wire_cmd(tx, ToggleRepeat));

    ui.on_playlist_opened(wire_cmd_1arg(tx, |playlist_index| {
        OpenPlaylist(playlist_index as usize)
    }));
}

/// Returns a closure that sends `cmd` over `tx`. For callbacks that only
/// need to fire a command and don't touch the UI.
fn wire_cmd(tx: &Sender<PlayerCommand>, cmd: PlayerCommand) -> impl Fn() + use<> {
    let tx = tx.clone();
    move || {
        let _ = tx.try_send(cmd.clone());
    }
}

/// Same as `wire_cmd` but accepts one argument from Slint and maps it to a
/// command.
fn wire_cmd_1arg<T, F>(tx: &Sender<PlayerCommand>, make_cmd: F) -> impl Fn(T) + use<T, F>
where
    F: Fn(T) -> PlayerCommand + 'static,
{
    let tx = tx.clone();
    move |arg| {
        let _ = tx.try_send(make_cmd(arg));
    }
}

/// Returns a closure that upgrades `ui` and calls `f` with the live handle
/// and a cloned sender. For callbacks that need to read or write UI state.
fn wire_cmd_ui<F>(ui: &AppWindow, tx: &Sender<PlayerCommand>, f: F) -> impl Fn() + use<F>
where
    F: Fn(&AppWindow, &Sender<PlayerCommand>) + 'static,
{
    let tx = tx.clone();
    let ui_weak = ui.as_weak();
    move || {
        if let Some(ui) = ui_weak.upgrade() {
            f(&ui, &tx);
        }
    }
}

/// Same as `wire_cmd_ui` but accepts one argument from Slint.
fn wire_cmd_ui_1arg<T, F>(
    ui: &AppWindow,
    tx: &Sender<PlayerCommand>,
    f: F,
) -> impl Fn(T) + use<T, F>
where
    F: Fn(&AppWindow, &Sender<PlayerCommand>, T) + 'static,
{
    let tx = tx.clone();
    let ui_weak = ui.as_weak();
    move |arg| {
        if let Some(ui) = ui_weak.upgrade() {
            f(&ui, &tx, arg);
        }
    }
}
