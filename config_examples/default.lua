music_dir = "~/Music"
default_volume = 1.0
window_maximized = true

keymaps = {
    toggle_play = "Space",
    next_track = "N",
    prev_track = "P",
    volume_up = "UpArrow",
    volume_down = "DownArrow",
}

function on_startup()
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

    set_property("sidebar", "text_size", 18)
    set_property("now_playing_song", "text_size", 15)
    set_property("now_playing_artist", "text_size", 12)
    set_property("detail_view_header_title", "text_size", 28)
    set_property("detail_view_header_subtitle", "text_size", 18)
    set_property("library_list_title", "text_size", 15)
    set_property("library_list_subtitle", "text_size", 12)
    set_property("album_list_title", "text_size", 15)
    set_property("album_list_subtitle", "text_size", 12)
end