//! Platform agnostic audio playback interface.
//! 
//! Desktop playback goes through `MiniAudioPlayer`m a thin FFI wrapper
//! around the `miniaudio` C library (see `miniaudio.c`). Mobile/Android
//! targets can implement `AudioPlayer` using platfor native audio
//! engines (oboe/aaudio) without leaking implementation details.

use std::error::Error;
use std::ffi::CString;
use std::path::Path;
use std::time::Duration;

/// Anything that can load, play, pause, and seek audio tracks.
///
/// `LocalBackend` holds a `Box<dyn AudioPlayer>` rather than a concrete
/// `MiniAudioPlayer` for two reasons: it lets tests substitute a fake
/// player without touching real audio hardware, and it leaves room for a
/// lower level platform backend later if `miniaudio`'s device handling
/// ever proves insufficient on Android, not because Android needs a
/// different impl today, it doesn't.
pub(crate) trait AudioPlayer: Send {
    fn play(&mut self);
    fn pause(&mut self);
    /// Pauses playback and resets position to the start of the current
    /// track. Distinct from `pause`, which stops advancing but leaves
    /// position where it was.
    fn stop(&mut self);
    fn set_volume(&mut self, volume: f32);
    fn try_seek(&mut self, position: Duration) -> Result<(), Box<dyn Error>>;
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    /// True once the loaded track has finished playing, or if nothing has
    /// been loaded yet: both cases look identical to the backend.
    fn empty(&self) -> bool;
    /// Loads the audio file at `path`, replacing whatever was previously
    /// loaded. Implementations own opening/decoding the file themselves,
    /// since native backends like `miniaudio` load directly from a path
    /// rather than an already open file handle.
    fn append(&mut self, path: &Path) -> Result<(), Box<dyn Error>>;
    fn volume(&self) -> f32;
}

/// Opaque handle to the C-side `MiniAudioPlayerC` struct (see
/// `miniaudio.c`). Its layout is never inspected frm Rust, only passed
/// back to the FFI functions that created it.
#[repr(C)]
struct OpaqueBackend {
    _private: [u8; 0]
}

// FFI bindings to `miniaudio.c`. Every function takes the
// `OpaqueBackend` pointer returned by `miniaudio_create` as its first
// argument and is a thin wrapper around the underlying `ma_engine`/
// `ma_sound` calls: see `miniaudio.c` for what each one actually does.
unsafe extern "C" {
    fn miniaudio_create() -> *mut OpaqueBackend;
    fn miniaudio_destroy(player: *mut OpaqueBackend);
    /// `path` must be a NUL-terminated, platform native narrow C string.
    /// Use `miniaudio_load_file_win` on Windows instead: see
    /// `MiniAudioPlayer::append`.
    fn miniaudio_load_file(player: *mut OpaqueBackend, path: *const i8) -> bool;
    /// Windows-only counterpart to `miniaudio_load_file` taking a
    /// NUL-terminated UTF-16 string, since Windows paths aren't reliably
    /// representable as narrow C strings.
    fn miniaudio_load_file_win(player: *mut OpaqueBackend, path: *const u16) -> bool;
    fn miniaudio_play(player: *mut OpaqueBackend);
    fn miniaudio_pause(player: *mut OpaqueBackend);
    fn miniaudio_stop(player: *mut OpaqueBackend);
    fn miniaudio_seek_seconds(player: *mut OpaqueBackend, seconds: f64) -> bool;
    fn miniaudio_get_position_seconds(player: *mut OpaqueBackend) -> f64;
    fn miniaudio_is_empty(player: *mut OpaqueBackend) -> bool;
    fn miniaudio_is_paused(player: *mut OpaqueBackend) -> bool;
    fn miniaudio_volume(player: *mut OpaqueBackend) -> f32;
    fn miniaudio_set_volume(player: *mut OpaqueBackend, volume: f32);
}

/// Desktop implementation backed by `miniaudio` (see `miniaudio.c`),
/// wrapping a single `ma_engine` + `ma_sound` pair behind an opaque
/// pointer.
pub(crate) struct MiniAudioPlayer {
    ptr: *mut OpaqueBackend,
}

impl MiniAudioPlayer {
    /// Initializes the underlying `miniaudio` engine on the default audio
    /// device.
    /// 
    /// # Panics
    /// Panics if `miniaudio` fails to initialize (e.g. no default audio
    /// output device is available).
    pub(crate) fn new() -> Self {
        let ptr = unsafe {
            miniaudio_create()
        };

        assert!(!ptr.is_null(), "Failed to initialize miniaudio engine");

        Self {
            ptr,
        }
    }
}

impl Drop for MiniAudioPlayer {
    /// Tears down the `ma_sound` (if any) and `ma_engine` on the C side.
    fn drop(&mut self) {
        unsafe {
            miniaudio_destroy(self.ptr);
        };
    }
}

// Safe: `ptr` is only ever touched from the single dedicated audio
// thread that owns this `MiniAudioPlayer` (same pattern as `mlua::Lua`
// elsewhere in the app). `MiniAudioPlayer` is never `Sync`, so nothing
// else can reach `ptr` concurrently.
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
            miniaudio_stop(self.ptr);
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
        // Windows paths aren't reliably representable as narrow C
        // strings, so route through the wide (UTF-16) variant there.
        // Everywhere else a plain CString works.
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