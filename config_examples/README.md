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
| `sidebar_bg` | string (hex color) | `"#0e101d"` | Background color of the sidebar. |
| `now_playing_bar_bg` | string (hex color) | `"#0a0a0f"` | Background color of the bottom bat. |
| `library_view_bg` | string (hex color) | `"#1f1d2f"` | Background color of the library. |
| `album_view_bg` | string (hex color) | `"#1f1d2f"` | Background color of an opened album. |
| `playlists_view_bg` | string (hex color) | `"#1f1d2f"` | Background color of the playlists section. |
| `open_playlist_view_bg` | string (hex color) | `"#1f1d2f"` | Background color of an opened playlist. |
| `sidebar_text_color` | string (hex color) | `#cdd6f4` | Color of sidebar text. |
| `now_playing_song_text_color` | string (hex color) | `#cdd6f4` | Color of song title text in bottom bar. |
| `now_playing_artist_text_color` | string (hex color) | `#cdd6f4` | Color of artist name text in bottom bar. |
| `detail_view_header_title_text_color` | string (hex color) | `#cdd6f4` | Color of the album/playlist title at the top of an open album/playlist. |
| `detail_view_header_subtitle_text_color` | string (hex color) | `#cdd6f4` | Color of the subtitle below the header title (artist/track count). |
| `library_list_title_text_color` | string (hex color) | `#cdd6f4` | Color of the album/playlist title in the list views. |
| `library_list_subtitle_text_color` | string (hex color) | `#cdd6f4` | Color of the subtitle below the entry title (artist/track count). |
| `album_list_title_text_color` | string (hex color) | `#cdd6g4` | Color of the song titles inside open album/playlist views. |
| `album_list_subtitle_text_color` | string (hex color) | `#cdd6g4` | Color of the artist names inside open album/playlist views. |
| `sidebar_text_size` | int (px) | 18 | Size of sidebar text. |
| `now_playing_song_text_size` | int (px) | 15 | Size of song title text in bottom bar. |
| `now_playing_artist_text_size` | int (px) | 12 | Size of artist name size in bottom bar. |
| `detail_view_header_title_text_size` | int (px) | 28 | Size of the album/playlist title at the top of an open album/playlist. |
| `detail_view_header_subtitle_text_size` | int (px) | 18 | Size of the subtitle below the header title (artist/track count). |
| `library_list_title_text_size` | int (px) | 15 | Size of the album/playlist title in the list views. |
| `library_list_subtitle_text_size` | int (px) | 12 | Size of the subtitle below the entry title (artist/track count). |
| `album_list_title_text_color` | int (px) | 15 | Size of the song titles inside open album/playlist views. |
| `album_list_subtitle_text_color` | int (px) | 12 | Size of the artist names inside open album/playlist views. |
| `playlists` | table | `{}` | A list of playlist definitions. See below. |

Colors must be 6-digit hex strings, with or without a leading `#`. Anything
else falls back to the default for that key.

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

## Why Lua?

A static format like JSON or TOML can only describe data. Lua lets the config
compute that data. The `config_examples/` folder shows examples ranging from
a simple key-value file to a script that changes the UI's colors depending on
the time of day.
