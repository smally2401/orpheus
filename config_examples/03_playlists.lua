music_dir = "~/my-stuff/music"

playlists = {
    {
        name = "Rock Playlist",
        songs = {
            "the-beatles/abbey-road/something.mp3",
            "metallica/black-album/enter-sandman.flac",
        },
        art = "covers/cool-ass-pic.jpg"
    },
    {
        name = "Joy Division Discography",
        songs = list_music_files("joy-division"),
        sort = true,
    },
}
