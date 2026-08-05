use crate::AppWindow;
use orpheus_core::config::theme::Rgb;
use orpheus_core::config::theme::UiElement;
use orpheus_core::config::theme::UiProperty;

fn color_from_rgb(rgb: Rgb) -> slint::Color {
    slint::Color::from_rgb_u8(rgb.r, rgb.g, rgb.b)
}

/// Applies `property` to `element` on `ui`, if `element` actually has
/// that property. Elements that don't (e.g. `PlaylistsView` has no
/// `TextColor`) are logged and ignored rather than treated as an error.
/// Called from `handle_command`'s `PlayerCommand::SetProperty` arm.
pub(crate) fn set_property(ui: &AppWindow, element: UiElement, property: &UiProperty) {
    use orpheus_core::config::theme::UiElement::*;
    use orpheus_core::config::theme::UiProperty::*;

    match property {
        Bg(color) => match element {
            Sidebar => ui.set_sidebar_bg(color_from_rgb(*color)),
            NowPlayingBar => ui.set_now_playing_bar_bg(color_from_rgb(*color)),
            LibraryView => ui.set_library_view_bg(color_from_rgb(*color)),
            AlbumView => ui.set_album_view_bg(color_from_rgb(*color)),
            PlaylistsView => ui.set_playlists_view_bg(color_from_rgb(*color)),
            OpenPlaylistView => ui.set_open_playlist_view_bg(color_from_rgb(*color)),
            other => eprintln!("{other} doesn't have a bg property"),
        },

        TextColor(color) => match element {
            Sidebar => ui.set_sidebar_text_color(color_from_rgb(*color)),
            NowPlayingSong => ui.set_now_playing_song_text_color(color_from_rgb(*color)),
            NowPlayingArtist => ui.set_now_playing_artist_text_color(color_from_rgb(*color)),
            DetailViewHeaderTitle => ui.set_detail_view_header_title_text_color(color_from_rgb(*color)),
            DetailViewHeaderSubtitle => ui.set_detail_view_header_subtitle_text_color(color_from_rgb(*color)),
            LibraryListTitle => ui.set_library_list_title_text_color(color_from_rgb(*color)),
            LibraryListSubtitle => ui.set_library_list_subtitle_text_color(color_from_rgb(*color)),
            AlbumListTitle => ui.set_album_list_title_text_color(color_from_rgb(*color)),
            AlbumListSubtitle => ui.set_album_list_subtitle_text_color(color_from_rgb(*color)),
            other => eprintln!("{other} doesn't have a text_color property"),
        },

        TextSize(size) => match element {
            Sidebar => ui.set_sidebar_text_size(*size),
            NowPlayingSong => ui.set_now_playing_song_text_size(*size),
            NowPlayingArtist => ui.set_now_playing_artist_text_size(*size),
            DetailViewHeaderTitle => ui.set_detail_view_header_title_text_size(*size),
            DetailViewHeaderSubtitle => ui.set_detail_view_header_subtitle_text_size(*size),
            LibraryListTitle => ui.set_library_list_title_text_size(*size),
            LibraryListSubtitle => ui.set_library_list_subtitle_text_size(*size),
            AlbumListTitle => ui.set_album_list_title_text_size(*size),
            AlbumListSubtitle => ui.set_album_list_subtitle_text_size(*size),
            other => eprintln!("{other} doesn't have a text_size property"),
        },
    }
}
