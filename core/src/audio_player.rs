//! Platform agnostic audio playback interface. Desktop uses `rodio` via
//! `RodioPlayer`, Android will use a different implementation of the same
//! trait.

use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::io::BufReader;

/// Anything that can load, play, pause, and seek audio tracks.
///
/// `LocalBackend` holds a `Box<dyn AudioPlayer>` so the same playback
/// logic works on desktop (rodio) and Android (oboe/aaudio) without
/// platform specific code leaking into the backend.
pub(crate) trait  AudioPlayer: Send {
    fn play(&mut self);
    fn pause(&mut self);
    fn stop(&mut self);
    fn set_volume(&mut self, volume: f32);
    fn try_seek(&mut self, position: Duration) -> Result<(), Box<dyn Error>>;
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    fn empty(&self) -> bool;
    /// Appends a decoded audio source to the player's internal queue.
    fn append(&mut self, source: rodio::Decoder<BufReader<File>>);
    fn volume(&self) -> f32;
}

/// Desktop  implementation backed by `rodio::Player` + 
/// `rodio::MixerDeviceSink`.
pub(crate) struct RodioPlayer {
    _stream: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl RodioPlayer {
    /// Opens the default audio devide and returns a ready player. Panics
    /// if no audio device is available.
    pub(crate) fn new() -> Self {
        let stream = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let mixer = stream.mixer();
        let player = rodio::Player::connect_new(mixer);
        Self {
            _stream: stream,
            player,
        }
    }
}

impl AudioPlayer for RodioPlayer {
    fn play(&mut self) {
        self.player.play();
    }

    fn pause(&mut self) {
        self.player.pause();
    }

    fn stop(&mut self) {
        self.player.stop();
    }

    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), Box<dyn Error>> {
        self.player.try_seek(position)?;
        Ok(())
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    fn empty(&self) -> bool {
        self.player.empty()
    }

    fn append(&mut self, source: rodio::Decoder<BufReader<File>>) {
        self.player.append(source);
    }

    fn volume(&self) -> f32 {
        self.player.volume()
    }
}
