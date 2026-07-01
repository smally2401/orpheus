use mpd::Client;

pub struct TrackInfo {
    pub title: String,
    pub artist: String,
}

pub fn fetch_mpd_metadata() -> Result<TrackInfo, mpd::error::Error> {
    let mut client = Client::connect("127.0.0.1:6600")?;

    if let Some(song) = client.currentsong()? {

        let title = song.title.unwrap_or_else(|| song.file.clone());
        let artist = song.artist.unwrap_or_else(|| "unknown artist".to_string());

        Ok(TrackInfo { title, artist })

    } else {
        Ok(TrackInfo {
            title: "no song playing".to_string(),
            artist: "---".to_string(),
        })
    }
}

pub fn toggle_mpd_play() -> Result<(), mpd::error::Error> {
    let mut client = Client::connect("127.0.0.1:6600")?;
    let status = client.status()?;

    match status.state {
        mpd::State::Play => client.pause(true),
        mpd::State::Pause => client.pause(false),
        mpd::State::Stop => client.stop(), // not necessary for now, might even remove later
    }
}

pub fn next_mpd_track() -> Result<(), mpd::error::Error> {
    let mut client = Client::connect("127.0.0.1:6600")?;
    client.next()
}

pub fn prev_mpd_track() -> Result<(), mpd::error::Error> {
    let mut client = Client::connect("127.0.0.1:6600")?;
    client.prev()
}
