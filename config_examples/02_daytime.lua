local t = os.date("*t")
local hour = t.hour

if hour >= 22 or hour <= 7 then
    -- nighttime
    sidebar_bg = "#0e101d"
    now_playing_bar_bg = "#0a0a0f"
    library_view_bg = "#1f1d2f"
    album_view_bg = "#1f1d2f"
    playlists_view_bg = "#1f1d2f"
    open_playlist_view_bg = "#1f1d2f"

    sidebar_text_color = "#cdd6f4"
    now_playing_song_text_color = "#cdd6f4"
    now_playing_artist_text_color = "#cdd6f4"
    detail_view_header_title_text_color = "#cdd6f4"
    detail_view_header_subtitle_text_color = "#cdd6f4"
    library_list_title_text_color = "#cdd6f4"
    library_list_subtitle_text_color = "#cdd6f4"
    album_list_title_text_color = "#cdd6f4"
    album_list_subtitle_text_color = "#cdd6f4"
else
    -- daytime
    sidebar_bg = "#e6e8f5"
    now_playing_bar_bg = "#f4f5fb"
    library_view_bg = "#d6d4eb"
    album_view_bg = "#d6d4eb"
    playlists_view_bg = "#d6d4eb"
    open_playlist_view_bg = "#d6d4eb"

    sidebar_text_color = "#1a1a1a"
    now_playing_song_text_color = "#1a1a1a"
    now_playing_artist_text_color = "#1a1a1a"
    detail_view_header_title_text_color = "#1a1a1a"
    detail_view_header_subtitle_text_color = "#1a1a1a"
    library_list_title_text_color = "#1a1a1a"
    library_list_subtitle_text_color = "#1a1a1a"
    album_list_title_text_color = "#1a1a1a"
    album_list_subtitle_text_color = "#1a1a1a"
end
