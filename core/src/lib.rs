pub(crate) mod audio_player;
pub mod config;
pub mod local_backend;
pub(crate) mod mpris;
pub mod player_bridge;
pub(crate) mod state;
pub mod utils;

uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn android_text() -> String {
    String::from("Hello, World!")
}