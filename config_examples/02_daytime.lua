-- Demonstrates why Orpheus uses Lua instead of a static format like JSON
-- or TOML: config.lua is a real script, so it can compute values instead
-- of just declaring them. Here, the theme is chosen based on the time of
-- day, which a static format has no way to express, you'd need Orpheus
-- itself to grow "time-of-day theme" as a built-in feature. With Lua, it's
-- just an ordinary if/else over os.date().

local t = os.date("*t")
local hour = t.hour

if hour >= 22 or hour <= 7 then
    -- nighttime
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
    }
else
    -- daytime
    theme = {
        bg = {
            sidebar = "#e6e8f5",
            now_playing_bar = "#f4f5fb",
            library_view = "#d6d4eb",
            album_view = "#d6d4eb",
            playlists_view = "#d6d4eb",
            open_playlist_view = "#d6d4eb",
        },

        text_color = {
            sidebar = "#1a1a1a",
            now_playing_song = "#1a1a1a",
            now_playing_artist = "#1a1a1a",
            detail_view_header_title = "#1a1a1a",
            detail_view_header_subtitle = "#1a1a1a",
            library_list_title = "#1a1a1a",
            library_list_subtitle = "#1a1a1a",
            album_list_title = "#1a1a1a",
            album_list_subtitle = "#1a1a1a",
        },
    }
end
