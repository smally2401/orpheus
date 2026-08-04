//! The `playlists` block of `config.lua`: a Lua array of
//! `{ name, songs, sort, art }` tables, parsed into `PlaylistDef`s.

/// A single playlist as declared in `config.lua`, before its song paths
/// have been resolved against `music_dir` or matched against the library.
/// See `local_backend::build_playlists` for that resolution step.
pub(crate) struct PlaylistDef {
    pub(crate) name: String,
    pub(crate) songs: Vec<String>,
    /// If true, songs are re-sorted by artist/album/track number when
    /// resolved, rather than kept in the order listed. Intended for
    /// playlists generated with `list_music_files`, where listed order
    /// is just filesystem walk order.
    pub(crate) sort: bool,
    /// Optional path to a custom cover image for this playlist, relative
    /// to `music_dir`. If set, the image is loaded and displayed as the
    /// playlist's cover art. Falls back to the bundled placeholder if the
    /// path is missing or the image fails to load.
    pub(crate) art: Option<String>,
}

/// Reads the `playlists` global, a Lua array of `{ name, songs, sort, art }`
/// tables, into `PlaylistDef`s.
///
/// Missing or malformed entries are skipped individually rather than
/// failing the whole config load. Each song string that fails to
/// convert is also dropped silently.
pub(super) fn get_playlists(globals: &mlua::Table) -> Vec<PlaylistDef> {
    let playlists_table: mlua::Table = match globals.get("playlists") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut playlists = Vec::new();

    for entry in playlists_table.sequence_values::<mlua::Table>() {
        let entry_table: mlua::Table = match entry {
            Ok(t) => t,
            Err(_) => continue,
        };

        let name: String = match entry_table.get("name") {
            Ok(n) => n,
            Err(_) => continue,
        };

        let songs_table: mlua::Table = match entry_table.get("songs") {
            Ok(t) => t,
            Err(_) => continue,
        };

        let art: Option<String> = entry_table.get("art").ok();

        let songs: Vec<String> = songs_table
            .sequence_values::<String>()
            .filter_map(std::result::Result::ok)
            .collect();

        let sort: bool = entry_table.get("sort").unwrap_or(false);

        playlists.push(PlaylistDef {
            name,
            songs,
            sort,
            art,
        });
    }

    playlists
}
