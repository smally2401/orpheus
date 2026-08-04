/// Decodes raw embedded cover art bytes into a Slint `Image`, resizing to a
/// fixed 100x100 thumbnail. Falls back to a bundled placeholder image if
/// no art is present.
pub(crate) fn art_rust_to_slint(art: Option<&[u8]>) -> slint::Image {
    slint::Image::from(&decode_art(art))
}

/// Reads an image file from disk and decodes it into a Slint `Image`.
///
/// Returns the bundled placeholder if the path is `None`, the file doesn't
/// exist, or the bytes fail to decode as an image.
pub(crate) fn get_art_from_path(path: Option<PathBuf>) -> slint::Image {
    let contents = match path {
        Some(path) => std::fs::read(path).ok(),
        None => None,
    };
    art_rust_to_slint(contents.as_deref())
}

impl From<&Song> for SlintSong {
    /// Converts a single `Song` into its Slint-facing representation.
    fn from(song: &Song) -> Self {
        Self {
            title: song.title.clone().into(),
            artist: song.artist.clone().into(),
        }
    }
}

impl From<&Album> for SlintAlbum {
    /// Converts an `Album` into its Slint-facing representation, including
    /// a decoded, UI-ready cover image via `art_rust_to_slint`.
    fn from(album: &Album) -> Self {
        SlintAlbum {
            title: album.title.clone().into(),
            artist: album.artist.clone().into(),
            track_count: album.tracklist.len() as i32,
            tracks: tracklist_rust_to_slint(&album.tracklist),
            art: art_rust_to_slint(album.art.as_deref().map(Vec::as_slice)),
        }
    }
}

impl From<&DecodedArt> for slint::Image {
    /// The other half: wraps already-`decode_art`-ed bytes into a real
    /// `slint::Image`. Must run on the thread that will use the resulting
    /// image (in practice, the UI thread, e.g. inside
    /// `slint::invoke_from_event_loop`).
    fn from(art: &DecodedArt) -> Self {
        let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
            &art.rgb, art.width, art.height,
        );
        slint::Image::from_rgb8(buffer)
    }
}

/// Builds a `SlintPlaylist` without decoding any track art. Used for the
/// sidebar list at startup: art is loaded lazily when the playlist is opened.
pub(crate) fn playlist_rust_to_slint(preview: &PlaylistPreview) -> SlintPlaylist {
    SlintPlaylist {
        name: preview.name.clone().into(),
        track_count: preview.track_count as i32,
        tracks: ModelRc::new(VecModel::<SlintSongWithArt>::from(Vec::new())),
        art: get_art_from_path(preview.art_path.clone()),
    }
}

/// Converts a tracklist into the `ModelRc` Slint expects for list items.
pub(crate) fn tracklist_rust_to_slint(tracklist: &[Arc<Song>]) -> ModelRc<SlintSong> {
    let slint_tracklist: Vec<SlintSong> = tracklist
        .iter()
        .map(|s| SlintSong::from(s.as_ref()))
        .collect();

    ModelRc::new(VecModel::from(slint_tracklist))
}