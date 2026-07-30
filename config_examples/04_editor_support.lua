-- Demonstrates the `---@type` annotations that give lua-language-server
-- autocomplete and type-checking on nested tables. See the "Editor
-- support" section of the README for setup instructions.
--
-- These annotaations are optional, config.lua works identically without
-- them, but adding one directly above an assignment lets the language
-- server suggest field names inside that table and catch type mistakes
-- (e.g. a typo'd keymap action, or an int where a hex color is expected)
-- before you restart Orpheus to find out.
--
-- One gap worth knowing: the language server checks that fields you
-- provide have the right type, but doesn't flag fields that shouldn't be
-- there at all. So `keymaps = { frobnicate = "F" }` won't autocomplete
-- "frobnicate" (good signal something's off), but also won't get a hard
-- error if you type it out and move on. Orpheus itself catches this at
-- runtime instead (see the terminal output when running orpheus): the
-- two layers complement each other rather than one replacing the other.

-- Typing inside this table now suggests toggle_play, next_track, etc.
---@type OrpheusKeymaps
keymaps = {
    toggle_play = "Space",
    hello = "",
}

-- Typing `theme.bg.` (or inside text_color/text_size) now suggests the
-- matching field names.
---@type OrpheusTheme
theme = {
    bg = {
        sidebar = "#0e101d",
        now_playing_bar = "#0a0a0f",
        library_view = "#1f1d2f",
        album_view = "#1f1d2f",
        playlists_view = "#1f1d2f",
        open_playlist_view = "#1f1d2f",
    },

    text_color = {
        sidebar = "#cdd6f4",
        now_playing_song = "#cdd6f4",
        now_playing_artist = "#cdd6f4",
        detail_view_header_title = "#cdd6f4",
        detail_view_header_subtitle = "#cdd6f4",
        library_list_title = "#cdd6f4",
        library_list_subtitle = "#cdd6f4",
        album_list_title = "#cdd6f4",
        album_list_subtitle = "#cdd6f4",
    },

    text_size = {
        sidebar = 18,
        now_playing_song = 15,
        now_playing_artist = 12,
        detail_view_header_title = 28,
        detail_view_header_subtitle = 18,
        library_list_title = 15,
        library_list_subtitle = 12,
        album_list_title = 15,
        album_list_subtitle = 12,
    },
}

-- The outer ---@type is enough for LuaLS to flag a missing required field
-- (like leaving out `songs`) after the fact, but live completion while
-- typing inside an entry needs its own inline ---@type too.
---@type OrpheusPlaylist[]
playlists = {
    ---@type OrpheusPlaylist
    {
        name = "Rock Playlist",
        songs = {
            "the-beatles/abbey-road/something.mp3",
            "metallica/black-album/enter-sandman.flac",
        },
        art = "covers/cool-ass-pic.jpg",
    },
}
