# Orpheus

A local-first music player built in Rust, with a UI powered by
[Slint](https://slint.dev) and a configuration system that's a real Lua
script instead of a static file.

> **Status:** early, actively developed (`v0.3.0`). Expect rough edges.

<!-- todo: screenshots -->
<!-- library view -->
<!-- album view -->
<!-- playlists view -->

## Features

- Local library scanning (`.mp3`, `.flac`) with tag-based metadata,
  cover art, and automatic album grouping
- Playlists defined in a Lua config file, including playlists generated
  programmatically from folder contents (see `config_examples/`)
- MPRIS integration (Linux), so playback shows up in your desktop's media
  controls, lock screen widget, and media keys
- Configurable UI colors and music library location, all via one
  `config.lua` file

## Building and running

### Desktop

Requires a recent [Rust toolchain](https://rustup.rs).

```sh
cargo run --release
```

On first run, Orpheus creates a default config at
`~/.config/orpheus/config.lua` (Linux) and points itself at `~/Music`.

### Android (using pre-compiled binaries)

Requires **JDK 17+** and the **Android SDK** (`ANDROID_HOME` exported or
`sdk.dir` defined in `android/local.properties`).

1. Navigate to the Android project directory:
   ```sh
   cd android
   ```

2. Build the debug APK:
   ```sh
   # Linux / macOS
   ./gradlew assembleDebug

   # Windows
   gradlew.bat assembleDebug
   ```
3. The generated APL will be at 
`android/app/build/outputs/apk/debug/app-debug.apk`.

### Compiling `.so` shared libraries from source

If you modify the native backend in `core/`, you must recompile the `.so`
shared libraries and update the Kotlin UniFFI bindings.

**Prerequisites:**
- Android NDK installed
- Cross-compilation targets added via `rustup`:
  ```sh
  rustup target add aarch64-linux-android x86_64-linux-android
  ```

**Steps:**

1. Build native shared libraries for target architectures;
   ```sh
   cargo ndk -t aarch64-linux-android -t x86_64-linux-android --platform 26 -o ./android/app/src/main/jniLibs build -p orpheus-core --release
   ```

2. Copy the generated `.so` binaries to the corresponding JNI target
   directories:
   - `target/aarch64-linux-android/release/liborpheus_core.so` ->
     `android/app/src/main/jniLibs/arm64-v8a/`
   - `target/x86_64-linux-android/release/liborpheus_core.so` ->
     `android/app/src/main/jniLibs/x86_64/`

3. Generate updated UniFFI Kotlin bindings:
   ```sh
   cargo run -p orpheus-core --bin uniffi-bindgen -- generate \
    target/aarch64-linux-android/release/liborpheus_core.so \
    --language kotlin \
    --out-dir ./android/app/src/main/java/com/orpheus/ffi
   ```

## Configuration

Orpheus is configured with Lua rather than a static format like JSON or
TOML. This means your config can compute values, not just declare them.
See [`config_examples/README.md`](config_examples/README.md) for the full
reference, including:

- All available config keys (colors, music directory, playlists)
- How to define playlists, including auto-generating one from a folder
  with `list_music_files`
- Scripting hooks for reacting to playback, like `on_song_change`.
- Runnable example scripts, from a plain key-value file to a
  time of day aware color scheme

## Platform support

| Platform | Support |
| --- | --- |
| Linux | Working: primary development platform |
| Windows | Working |
| macOS | Not yet tested |

## Roadmap / known limitations

This is early: expect missing features and rough edges. See inline
`// todo` comments throughout the codebase for specific planned
improvements. Contributions, testing and bug reports are welcome.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) for release notes.
