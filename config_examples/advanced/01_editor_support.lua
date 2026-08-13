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
