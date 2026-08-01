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

Requires a recent [Rust toolchain](https://rustup.rs).

```sh
cargo run --release
```

On first run, Orpheus creates a default config at
`~/.config/orpheus/config.lua` (Linux) and points itself at `~/Music`.

## Installing

A standalone installer is included as a separate binary. It builds
Orpheus in release mode, then installs the binary and a shortcut/launcher
entry into standard per-platform locations.

```sh
cargo run --bin install
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
| Linux | Working: primary development platform
| Windows | Installer support added, not yet tested on a real Windows machine |
| macOS | Not yet supported |

## Roadmap / known limitations

This is early: expect missing features and rough edges. See inline
`// todo` comments throughout the codebase for specific planned
improvements. Contributions, testing and bug reports are welcome.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) for release notes.
