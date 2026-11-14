# orpheus

A small, scriptable music player written in C with SDL3, miniaudio and Lua.

The C core plays audio, reads tags and draws basic shapes. Everything you
see (the layout, widgets, playlists, keybindings) is written in Lua, so it
can be changed from a config file.

Supported formats: **MP3, FLAC, Ogg Vorbis, Wav**

---

## Installing on Linux

### 1. Install the dependencies

You need a C compiler, `make`, `pkg-config` and these libraries:
SDL3, SDL3_image, GLib/GIO, Lua 5.4 and fontconfig.

**Debian / Ubuntu** (Debian 13+, Ubuntu 25.04+):

```sh
sudo apt install build-essential pkg-config libsdl3-dev libsdl3-image-dev \
                 libglib2.0-dev liblua5.4-dev libfontconfig-dev
```

**Arch Linux**:

```sh
sudo pacman -S base-devel sdl3 sdl3_image glib2 lua54 fontconfig
```

**Fedora**:
```sh
sudo dnf install gcc make pkgconf-pkg-config SDL3-devel SDL_image-devel \
                 glib2-devel lua-devel fontconfig-devel
```

### 2. Build

```sh
git clone https://github.com/smally2401/orpheus.git
cd orpheus
make
```

This produces the `orpheus` executable in the project folder.

### 3. Run

**Run orpheus from inside the project folder.** In this version the Lua
part of the program is loaded from the `runtime/` folder next to it.

```sh
./orpheus                    # opens your library (~/Music)
./orpheus song.mp3           # plays a file
./orpheus ~/Music/SomeAlbum  # plays a folder
```
---

## Building on Windows

Windows builds use [MSYS2](https://www.msys2.org), which provides GCC and
prebuild Windows versions of all the libraries orpheus needs.

### 1. Install MSYS2 and the dependencies

1. Install MSYS2 from https://www.msys2.org
2. Open the **MSYS2 UCRT64** terminal from the start menu
3. Install the build tools and libraries:


```sh
pacman -S --needed git make \
 mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-pkgconf \
 mingw-w64-ucrt-x86_64-sdl3 mingw-w64-ucrt-x86_64-sdl3-image \
 mingw-w64-ucrt-x86_64-glib2 mingw-w64-ucrt-x86_64-lua54 \
 mingw-w64-ucrt-x86_64-fontconfig
```

### 2. Build

In the same UCRT64 terminal:

```sh
git clone https://github.com/smally2401/orpheus.git
cd orpheus
make
./orpheus.exe
```

As on Linux, start it from inside the project folder so it finds `runtime/`.

### 3. Make a folder

`orpheus.exe` only runs inside the MSYS2 terminal until you copy the DLLs
it needs next to it. To build a self-contained folder:

```sh
mkdir -p dist/licenses
cp orpheus.exe LICENSE dist/
cp -r runtime dist/
ldd orpheus.exe | awk '/ucrt64/ {print $3}' | xargs -I{} cp {} dist/
cp /ucrt64/bin/gspawn-win64-helper*.exe dist/
for d in $(ldd orpheus.exe | awk '/ucrt64/ {print $3}'); do
  pkg=$(pacman -Qoq "$d")
  cp -r "/ucrt64/share/licenses/${pkg#mingw-w64-ucrt-x86_64-}" dist/licenses/ 2>/dev/null
done
```

## Using it

On startup orpheus scans your music folder (`~/Music` or whatever your
system's XDG music directory is) and lists every track. Next time it starts,
the scan is almost instant because tags are cached.

- **Double-click** a track (or select it and press **Enter**) to play it.
- **Drag and drop** files or folders onto the window to add them.
- Click a row to select it, **ctrl-click** and **shift-click** select several.
- Use the bar at the bottom for previous / play-pause / next, seeking and volume.
- Closing the windows remembers the volume and the last track and position.

### Keyboard shortcuts

| key | action |
|-----|--------|
| `space` | play / pause |
| `←` / `→` | seek 5 seconds back / forward |
| `ctrl+←` / `ctrl+→` | previous / next track |
| `-` / `=` | volume down / up |
| `↑` `↓` `PgUp` `PgDn` `Home` `End` | move in the track list (after clicking it) |
| `ctrl+a` | select all tracks in the list |
| `ctrl+q` | quit |

### Using a different music folder

Create `~/.config/orpheus/init.lua`:

```lua
orpheus.config.music_dirs = { "/mnt/music", orpheus.paths.home .. "/Downloads/albums" }
```

This file is your config. It's plain Lua and runs at startup, after the
default interface is built, so it can also change colours and keys:

```lua
local theme, ui = orpheus.theme, orpheus.ui

theme.bg = "#16171d"
theme.accent = "#e0a458"
ui.bind("n", function() orpheus.player.next() end)
```

If the config has an error, orpheus prints it in the terminal and keeps
running with the default setup.

### Where orpheus keeps its files

| what | where |
|------|-------|
| config | `~/.config/orpheus/init.lua` |
| playlists | `~/.local/share/orpheus/playlists/*.m3u` |
| last session | `~/.local/share/orpheus/session.lua` |
| tag cache | `~/.cache/orpheus/library.lua` (safe to delete) |

On Windows the config, playlists and session live in
`%LOCALAPPDATA%\orpheus\` instead.

---

## License

MIT, see [LICENSE](LICENSE). Bundled libraries: miniaudio, stb_truetype and
stb_vorbis (public domain / MIT-0).
