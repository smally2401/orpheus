# Changelog

All notable changes to Orpheus will be documented in this file.

## [0.2.0] - Unreleased

### Added

- Shuffle mode, toggleable during playback
- Repeat mode, toggleable between off, queue and track modes
- Playlists now show album art for each track
- Custom images for playlists
- Keymaps
- lua-language-server support
- Lua functions:
  - on_song_change
  - on_song_halfway
- Other Lua settings:
  - Text color
  - Text size
  - Default volume
  - Window size

### Known limitations

- macOS is not yet supported (Linux and Windows only)
- Windows installer support hasn't been tested on a real Windows machine
  yet
- Playlists can't be created or edited from within the app
- Control buttons don't render correctly if user doesn't have a font
  that supports them installed

---

## [0.1.0] - 2026-07-20

First testing release.

### Added

- Local music library scanning (`.mp3`, `.flac`), with albums grouped
  automatically from file tags
- Playback controls: play/pause, next/previous track, seek, volume
- Playlists, defined in `config.lua`:
  - Manually listed songs, in a fixed order
  - Auto-generated from a folder of songs (e.g. everything under an
    artist's folder), with optional sorting by artist/album/track number
- Configuration via `config.lua`, a real Lua script, not a static file:
  - Customizable UI colors
  - Customizable music library location
  - See `config_examples/` for example configs, including a time-of-day
    color scheme
- MPRIS integration (Linux): playback appears in your desktop's media
  controls, lock screen, and media keys, and responds to them
- Cover art display, pulled from embedded file tags
- Standalone installer (`cargo run --bin install`) that builds the app
  and installs a binary + shortcut/launcher entry
  - Linux: installs to your local bin directory, adds a `.desktop`
    launcher entry
  - Windows: experimental, untested: installs a `.lnk` shortcut to the
    Start Menu

### Known limitations

- macOS is not yet supported (Linux and experimental Windows only)
- Windows installer support hasn't been tested on a real Windows machine
  yet
- No shuffle or repeat modes yet
- Playlists can't be created or edited from within the app
