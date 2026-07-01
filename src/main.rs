mod mpd_backend;
mod local_backend;
mod player_bridge;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = AppWindow::new()?;

    match mpd_backend::fetch_mpd_metadata() {
        Ok(track) => {
            ui.set_current_track_title(track.title.into());
            ui.set_current_artist(track.artist.into());
        },
        Err(e) => println!("an error happened: {}", e),
    }

    ui.run()
}
