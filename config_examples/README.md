# Orpheus Configuration

Orpheus is configured with a Lua script instead of a static format like JSON
or TOML.

The config files lives at `~/.config/orpheus/config.lua` and is created with
sensible defaults the first time you run Orpheus.

See the `config_examples/` folder for runnable examples.

## Top-level keys

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `music_dir` | string | `"~/Music"` | Path to your music library. `~/` is expanded to your home directory. |
| `default_volume` | float (0.0-1.0) | 1.0 | Volume when opening the app. |
| `window_maximized` | bool | true | Starts the app maximized. |
| `window_width` | float | 800 | Window width on launch. |
| `window_height` | float | 600 | Window height on launch. |
| `keymaps` | table | see below | Custom keybindings. See below. |
| `playlists` | table | `{}` | A list of playlist definitions. See below. |
| `on_startup` | function | *(none)* | Called once at startup. See "Scripting hooks" below. |
| `on_song_change` | function | *(none)* | Called whenever the current track changes. See "Scripting hooks" below. |

Both `window_width` and `window_height` must exist for them to take effect.

## Editor support

Orpheus writes two files into `~/.config/orpheus/` alongside `config.lua`
that give editors autocomplete and type-checking on your config, if you have
[`lua-language-server`](https://github.com/LuaLS/lua-language-server) set up:

- `meta/orpheus.lua`: type declarations for every config field. Rewritten
  on every launch to stay in sync with the running version of Orpheus, so
  don't edit it.

- `.luarc.json`: workspace settings pointing the language server at
  `meta/`. Only written once, so feel free to extend it.

To get full completion on nested tables (`theme.bg.*`, individual
`keymaps` actions, etc.), add a `---@type` comment directly above the
relevant assignment in `config.lua`:

```lua
---@type OrpheusKeymaps
keymaps = {
    toggle_play = "Space",
    -- typing here now suggests next_track, prev_track, etc.
}
```

See `config_examples/` for a full config annotated this way.

This isn't required, `config.lua` works identically with or without these
annotations, but it catches mistakes (like a typo'd keymap action, or an
int where a hex color string is expected) before you restart Orpheus to see
them.

**One gap worth knowing:** the language server checks that fields you
provide have the right type, but doesn't flag fields that shouldn't be
there at all. So `keymaps = { frobnicate = "F" }` won't autocomplete
`frobnicate` (a good sign something's off), but also won't get a hard error
if you type it out and move on. This applies to `keymaps` and `theme`'s
nested tables alike. Orpheus itself catches this at runtime instead: check
the terminal output when running Orpheus for warnings about unrecognized
keys.

## Keybinds

`keymaps` is a table mapping an action name to a key combo string:

```lua
keymaps = {
    toggle_play = "Space",
    next_track = "N",
    prev_track = "P",
    volume_up = "UpArrow",
    volume_down = "DownArrow",
}
```

If you set `keymaps` at all, it replaces the defaults entirely: there's no
merging, so include every binding you want, not just the ones you're
changing.

### Actions

| Action | Default combo |
| --- | --- |
| `toggle_play` | `Space` |
| `next_track` | `N` |
| `prev_track` | `P` |
| `volume_up` | `UpArrow` |
| `volume_down` | `DownArrow` |
| `seek_forward` | *(unbound)* |
| `seek_backward` | *(unbound)* |
| `open_library` | *(unbound)* |
| `open_playlists` | *(unbound)* |

An unrecognized action name is skipped with a warning printed to the
terminal, and everything else in `keymaps` still loads normally.

### Combo syntax

A combo is one key name, optionally preceded by any of `ctrl`/`control`,
`shift` and `alt`, joined with `+` (e.g. `"ctrl+shift+P"`). Modifier names
are case-insensitive, the key name itself is not.

If a combo's key name isn't recognized, or the same combo is bound to more
than one action, that entry is skipped with a warning and the rest of the
table still loads.

[List of valid key names](KEYS.txt)

## Defining playlists

Each entry in `playlists` is a table with:

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | string | yes | Display name of the playlist. |
| `songs` | table of strings | yes | Song paths, relative to `music_dir`. |
| `sort` | boolean | no (default `false`) | If `true`, songs are sorted by album artist, then album name, then track number, instead of kept in the order lsited. |
| `art` | string | no | Path to a custom cover image, relative to `music_dir`. |

```lua
playlists = {
    {
        name = "My Mix",
        songs = {
            "joy-division/unknown-pleasures/01-disorder.mp3",
            "pink-floyd/animals/02-dogs.flac",
        },
        art = "covers/unknown-pleasures.jpg"
    }
}
```

Use `sort = true` for playlists you generate programmatically (see
`list_music_files` below), where the order they come back in isn't
necessarily the order you want to listen in.

## The `list_music_files` function

Orpheus exposes one helper function to your config script:

```lua
list_music_files(relative_dir) -> table of strings
```

Given a folder path relative to `music_dir`, it returns every song found
inside it (recursively), as paths relative to `music_dir`, the same format
`songs` expects. This lets you build a playlist out of an entire folder
without listing every file by hand:

```lua
playlists = {
    {
        name = "Joy Division",
        songs = list_music_files("joy-division"),
        sort = true
    }
}
```

### Important: `music_dir` must be set before you call it

Orpheus reads your config in two passes:

1. **First pass** - runs your script far enough to read `music_dir`, without
   `list_music_files` available yet.
2. **Second pass** - runs your whole script again, this time with
   `list_music_files` registered and bound to whatever `music_dir` resolved
   to in the first pass.

In practice, this just means: **assign `music_dir` as a plain top-level
statement before any `list_music_files(...)` calls.** If you don't set
`music_dir` explicitly, the default (`~/Music`) is used instead. You don't
need to set it just to use `list_music_files`, only if you want a custom
location.

```lua
-- correct: music_dir is set first
music_dir = "~/my_stuff/music"
playlists = {
    {
        name = "Joy Division",
        songs = list_music_files("joy-division"),
        sort = true
    }
}
```

## Scripting hooks

Orpheus exposes a small live runtime API for reacting to playback from
your config, separate from everything above (which is only read once at
startup). This is what powers things like scrobbling or a custom
now-playing display, entirely from Lua.

### `on_startup`

Define this as a top-level function in `config.lua` to run once, right
after Orpheus finishes loading your config.

```lua
function on_startup()
    set_property("sidebar", "bg", "#0e101d")
    set_property("now_playing_bar", "bg", "#0a0a0f")
    set_property("sidebar", "text_color", "#cdd6f4")
    set_property("sidebar", "text_size", 18)
end
```

### `on_song_change`

Define this as a top-level function in `config.lua` to be notified every
time the track changes. Orpheus calls it automatically, you don't call it
yourself.

```lua
function on_song_change(song)
    print("now playing: " .. song.title .. " by " .. song.artist)
end
```

`song` has the fields `title`, `artist` and `album`. Leaving `on_song_change`
undefined is fine, it's simply never called.

### `set_property`

Scripts can change UI properties at runtime from inside `on_song_change` or
`on_song_halfway`. This lets you do things like changing colors depending
on the currenly playing song's artist.

```lua
function on_song_change(song)
    if song.artist == "Joy Division" then
        set_property("sidebar", "bg", "#1a1a2e")
    else
        set_property("sidebar", "bg", "#0e101d")
    end
end
```

Signature:

```lua
set_property(element, property, value) -> nil
```

- `element`: the UI element name, e.g. `"sidebar"`, `"now_playing_bar"`,
  `"library_view"`, etc. Same names as `theme.bg` / `theme.text_color` /
  `theme.text_size` keys.
- `proerty`: one of `"bg"`, `"text_color"` or `"text_size"`.
- `value`: a hex colro string (with or without `#`) for `"bg"` and
  `"text_color"`, or a number for `"text_size"`.

Malformed calls (wrong value type, invalid hex, unknown property) are
logged and ignored, so a mistake here can't crash a script or interrupt
playback.

### `player`

Scripts can control playback directly via the `player` table, from any
scripting context (`on_startup`, `on_song_change`, `on_song_halfway`, or
any function you define and call yourself).

Available functions:

| Function | Description |
| --- | --- |
| `player.play()` | Resume playback. |
| `player.pause()` | Pause playback. |
| `player.toggle_play()` | Toggle between playing and paused. |
| `player.next_track()` | Skip to the next track in the queue. |
| `player.prev_track()` | Go back to the previous track in the queue. |
| `player.seek(seconds)` | Jump to an absolute position (in seconds) within the current track. |
| `player.seek_by(seconds)` | Seek relative to the current position. Negative values seek backward. |
| `player.set_volume(volume)` | Set volume, from `0.0` to `1.0`. |
| `player.toggle_repeat()` | Cycle repeat mode: off -> queue -> track -> off. |
| `player.toggle_shuffle()` | Toggle shuffle on/off. |
| `player.get_state()` | Returns the current playback state as a table (`title`, `artist`, `album`, `position`, `duration`, `paused`), or `nil` if nothing's played yet. |

All `player` functions except `get_state` return nothing. Malformed 
calls (wrong argument type or count) raise a Lua error the normal way.
Playback commands themselves fail silently if issued in an invalid state
(e.g. seeking with no track loaded), rather than crashing the script.

### `defer` and `every`

Scripts can schedule code to run later, from any scripting context, using
two global functions:

```lua
defer(seconds, fn) -> nil
every(seconds, fn) -> nil
```

`defer` runs `fn` once, after `seconds` have passed. `every` runs `fn`
repeatedly, once every `seconds`, indefinitely.

```lua
function on_startup()
    every(60, function()
        local state = player.get_state()
        if state then
            print("still playing: " .. state.title .. " (" .. state.position .. "/" .. state.duration .. "s)")
        end
    end)
end
```

`seconds` accepts fractional values (e.g. `0.5`). Timer callbacks can call
any other scripting API, including `player.*` and `set_property`. Errors
raised inside a timer callback are logged and otherwise ignored, same as
`on_song_change`/`on_song_halfway`: a bug in one timer can't crash the
script or stop other timers from firing. An `every` timer keeps
re-scheduling itself even if a given firing errors.

### A note on trust

Unlike the rest of `config.lua`, code inside `on_startup`, `on_song_change` 
and `on_song_halfway` runs with full access to Lua's standard library,
including `os` and `io`, and can load native (C-compiled) Lua modules via
`require`. This is necessary for things like the Last.fm scrobbling example,
which needs an HTTP client. It also means a hook can run shell commands
or read/write arbitrary files on your system. Treat scripts you didn't
write yourself, especially ones using `require`, with the same caution
you'd give any other shell script or executable before running them.

See `config_examples/` for both a minimal example and a Last.fm scrobbling
example built on this hook.

## Why Lua?

A static format like JSON or TOML can only describe data. Lua lets the config
compute that data. The `config_examples/` folder shows examples ranging from
a simple key-value file to a script that changes the UI's colors depending on
the time of day.
