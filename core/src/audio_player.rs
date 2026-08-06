//! Platform agnostic audio playback interface.
//! 
//! Desktop uses `rodio` via `RodioPlayer`, while mobile/Android targets can
//! implement `AudioPlayer` using platform native audio engines (oboe/aaudio)
//! without leaking implementation details.

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

/// Anything that can load, play, pause, and seek audio tracks.
///
/// `LocalBackend` holds a `Box<dyn AudioPlayer>` rather than a concrete
/// `RodioPlayer` for two reasons: it lets tests substitute a fake player
/// without touching real audio hardware, and it leaves room for a
/// lower level platform backend later if `rodio`'s `cpal`/`oboe` path
/// ever proves insufficient on Android, not because Android needs a
/// different impl today, it doesn't.
pub(crate) trait AudioPlayer: Send {
    fn play(&mut self);
    fn pause(&mut self);
    fn stop(&mut self);
    fn set_volume(&mut self, volume: f32);
    fn try_seek(&mut self, position: Duration) -> Result<(), Box<dyn Error>>;
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    fn empty(&self) -> bool;
    /// Takes ownership of an open audio file, decodes it, and appends it to
    /// the player's internal queue.
    fn append(&mut self, file: File) -> Result<(), Box<dyn Error>>;
    fn volume(&self) -> f32;
}

/// Desktop  implementation backed by `rodio::Player` +
/// `rodio::MixerDeviceSink`.
pub(crate) struct RodioPlayer {
    _stream: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl RodioPlayer {
    /// Opens the default audio device and returns a ready player.
    /// 
    /// # Panics
    /// Panics if no default audio output device is available.
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

    fn append(&mut self, file: File) -> Result<(), Box<dyn Error>> {
        let reader = BufReader::new(file);
        let decoder = rodio::Decoder::new(reader)?;
        self.player.append(decoder);
        Ok(())
    }
    

    fn volume(&self) -> f32 {
        self.player.volume()
    }
}
