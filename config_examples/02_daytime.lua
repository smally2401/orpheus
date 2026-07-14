local t = os.date("*t")
local hour = t.hour

if hour >= 22 or hour <= 7 then
    -- nighttime
    sidebar_bg = "#e6e8f5"
    now_playing_bar_bg = "f4f5fb"
    library_view_bg = "#d6d4eb"
    album_view_bg = "#d6d4eb"
    playlists_view_bg = "#d6d4eb"
    open_playlist_view_bg = "#d6d4eb"
else
    -- daytime
    sidebar_bg = "#0e101d"
    now_playing_bar_bg = "#0a0a0f"
    library_view_bg = "#1f1d2f"
    album_view_bg = "#1f1d2f"
    playlists_view_bg = "#1f1d2f"
    open_playlist_view_bg = "#1f1d2f"
end
