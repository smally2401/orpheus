#![allow(unused)]

//! Platform agnostic audio playback interface.
//! 
//! Desktop uses `rodio` via `RodioPlayer`, while mobile/Android targets can
//! implement `AudioPlayer` using platform native audio engines (oboe/aaudio)
//! without leaking implementation details.

use std::error::Error;
use std::ffi::CString;
use std::path::Path;
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
    fn append(&mut self, path: &Path) -> Result<(), Box<dyn Error>>;
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

    fn append(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let decoder = rodio::Decoder::new(reader)?;
        self.player.append(decoder);
        Ok(())
    }
    

    fn volume(&self) -> f32 {
        self.player.volume()
    }
}

#[repr(C)]
struct OpaqueBackend {
    _private: [u8; 0]
}

unsafe extern "C" {
    fn miniaudio_create() -> *mut OpaqueBackend;
    fn miniaudio_destroy(player: *mut OpaqueBackend);
    fn miniaudio_load_file(player: *mut OpaqueBackend, path: *const i8) -> bool;
    fn miniaudio_load_file_win(player: *mut OpaqueBackend, path: *const u16) -> bool;
    fn miniaudio_play(player: *mut OpaqueBackend);
    fn miniaudio_pause(player: *mut OpaqueBackend);
    fn miniaudio_seek_seconds(player: *mut OpaqueBackend, seconds: f64) -> bool;
    fn miniaudio_get_position_seconds(player: *mut OpaqueBackend) -> f64;
    fn miniaudio_is_empty(player: *mut OpaqueBackend) -> bool;
    fn miniaudio_is_paused(player: *mut OpaqueBackend) -> bool;
    fn miniaudio_volume(player: *mut OpaqueBackend) -> f32;
    fn miniaudio_set_volume(player: *mut OpaqueBackend, volume: f32);
}

pub(crate) struct MiniAudioPlayer {
    ptr: *mut OpaqueBackend,
    current_path: Option<String>,
}

impl MiniAudioPlayer {
    pub(crate) fn new() -> Self {
        let ptr = unsafe {
            miniaudio_create()
        };

        assert!(!ptr.is_null(), "Failed to initialize miniaudio engine");

        Self {
            ptr,
            current_path: None,
        }
    }
}

impl Drop for MiniAudioPlayer {
    fn drop(&mut self) {
        unsafe {
            miniaudio_destroy(self.ptr);
        };
    }
}

unsafe impl Send for MiniAudioPlayer {}

impl AudioPlayer for MiniAudioPlayer {
    fn play(&mut self) {
        unsafe {
            miniaudio_play(self.ptr);
        };
    }

    fn pause(&mut self) {
        unsafe {
            miniaudio_pause(self.ptr);
        };
    }

    fn stop(&mut self) {
        unsafe {
            miniaudio_pause(self.ptr);
        };
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), Box<dyn Error>> {
        let ok = unsafe {
            miniaudio_seek_seconds(self.ptr, position.as_secs_f64())
        };

        if ok {
            Ok(())
        } else {
            Err("Seek failed".into())
        }
    }

    fn position(&self) -> Duration {
        let secs = unsafe {
            miniaudio_get_position_seconds(self.ptr)
        };

        Duration::from_secs_f64(secs)
    }

    fn empty(&self) -> bool {
        unsafe {
            miniaudio_is_empty(self.ptr)
        }
    }

    fn is_paused(&self) -> bool {
        unsafe {
            miniaudio_is_paused(self.ptr)
        }
    }

    fn volume(&self) -> f32 {
        unsafe {
            miniaudio_volume(self.ptr)
        }
    }

    fn set_volume(&mut self, volume: f32) {
        unsafe {
            miniaudio_set_volume(self.ptr, volume);
        };
    }

    fn append(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {

        let ok = if cfg!(windows) {
            let os_str = path.as_os_str();

            let wide: Vec<u16> = os_str
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let ptr: *const u16 = wide.as_ptr();

            unsafe {
                miniaudio_load_file_win(self.ptr, ptr)
            }

        } else {
            let c_path = CString::new(path.as_os_str().as_encoded_bytes())?;

            unsafe {
                miniaudio_load_file(self.ptr, c_path.as_ptr())
            }
        };

        if ok {
            Ok(())
        } else {
            Err("Failed to load file".into())
        }
    }
}