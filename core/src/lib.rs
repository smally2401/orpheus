pub(crate) mod audio_player;
pub mod config;
pub mod local_backend;
pub(crate) mod mpris;
pub mod player_bridge;
pub(crate) mod state;
pub mod utils;
mod android_ffi;

uniffi::setup_scaffolding!();