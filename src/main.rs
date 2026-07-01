mod mpd_backend;
mod local_backend;
mod player_bridge;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = AppWindow::new()?;
    let tx = player_bridge::spawn_player_bridge(&ui);

    ui.run()
}
