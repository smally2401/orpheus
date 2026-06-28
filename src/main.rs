use mpd::Client;

slint::include_modules!();

struct TrackInfo {
    title: String,
    artist: String,
}

fn fetch_mpd_metadata() -> Result<TrackInfo, mpd::error::Error> {

    let mut client = Client::connect("127.0.0.1:6600")?;

    if let Some(song) = client.currentsong()? {

        let title = song.title.unwrap_or_else(|| song.file.clone());
        
        let artist = if let Some(native_artist) = song.artist {
            native_artist
        } else {
            song.tags.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("artist"))
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "unknown artist".to_string())
        };

        Ok(TrackInfo { title, artist })

    } else {
        Ok(TrackInfo { 
            title: "no song playing".to_string(), 
            artist: "---".to_string()
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = AppWindow::new()?;

    match fetch_mpd_metadata() {
        Ok(track) => {
            ui.set_current_track_title(track.title.into());
            ui.set_current_artist(track.artist.into());
        },
        Err(e) => println!("an error happened: {}", e),
    }

    ui.run()
}
