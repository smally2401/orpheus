-- Demonstrates why Orpheus uses Lua instead of a static format like JSON
-- or TOML: config.lua is a real script, so it can compute values instead
-- of just declaring them. Here, the theme is chosen based on the time of
-- day, which a static format has no way to express, you'd need Orpheus
-- itself to grow "time-of-day theme" as a built-in feature. With Lua, it's
-- just an ordinary if/else over os.date().

local t = os.date("*t")
local hour = t.hour

function on_startup()
    if hour >= 22 or hour <= 7 then
        -- nighttime
        set_property("sidebar", "bg", "#0e101d")
        set_property("now_playing_bar", "bg", "#0a0a0f")
        set_property("library_view", "bg", "#1f1d2f")
        set_property("album_view", "bg", "#1f1d2f")
        set_property("playlists_view", "bg", "#1f1d2f")
        set_property("open_playlist_view", "bg", "#1f1d2f")

        set_property("sidebar", "text_color", "#cdd6f4")
        set_property("now_playing_song", "text_color", "#cdd6f4")
        set_property("now_playing_artist", "text_color", "#cdd6f4")
        set_property("detail_view_header_title", "text_color", "#cdd6f4")
        set_property("detail_view_header_subtitle", "text_color", "#cdd6f4")
        set_property("library_list_title", "text_color", "#cdd6f4")
        set_property("library_list_subtitle", "text_color", "#cdd6f4")
        set_property("album_list_title", "text_color", "#cdd6f4")
        set_property("album_list_subtitle", "text_color", "#cdd6f4")
    else
        -- daytime
        set_property("sidebar", "bg", "#e6e8f5")
        set_property("now_playing_bar", "bg", "#f4f5fb")
        set_property("library_view", "bg", "#d6d4eb")
        set_property("album_view", "bg", "#d6d4eb")
        set_property("playlists_view", "bg", "#d6d4eb")
        set_property("open_playlist_view", "bg", "#d6d4eb")

        set_property("sidebar", "text_color", "#1a1a1a")
        set_property("now_playing_song", "text_color", "#1a1a1a")
        set_property("now_playing_artist", "text_color", "#1a1a1a")
        set_property("detail_view_header_title", "text_color", "#1a1a1a")
        set_property("detail_view_header_subtitle", "text_color", "#1a1a1a")
        set_property("library_list_title", "text_color", "#1a1a1a")
        set_property("library_list_subtitle", "text_color", "#1a1a1a")
        set_property("album_list_title", "text_color", "#1a1a1a")
        set_property("album_list_subtitle", "text_color", "#1a1a1a")
    end
end