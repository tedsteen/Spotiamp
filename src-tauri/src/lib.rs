use std::sync::OnceLock;

use spotify::SpotifyPlayer;
use tauri::{async_runtime::spawn, Listener};
use tokio::sync::Mutex;
mod oauth;
mod player_window;
mod playlist_window;
mod settings;
mod spotify;

pub fn player() -> &'static Mutex<SpotifyPlayer> {
    static MEM: OnceLock<Mutex<SpotifyPlayer>> = OnceLock::new();
    MEM.get_or_init(|| Mutex::new(SpotifyPlayer::new()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            player_window::get_track,
            player_window::play,
            player_window::pause,
            player_window::stop,
            player_window::get_volume
        ])
        .setup(|app| {
            app.listen("volume-change", move |event| {
                if let Ok(volume) = serde_json::from_str::<u16>(event.payload()) {
                    spawn(async move {
                        player().lock().await.set_volume(volume);
                    });
                }
            });
            let zoom = 2.0;

            let player_window = player_window::build_window(app, zoom)?;

            let mut playlist_position = player_window.outer_position()?;
            playlist_position.y += player_window.outer_size()?.height as i32;
            playlist_window::build_window(
                app,
                zoom,
                playlist_position.to_logical(player_window.scale_factor()?),
            )?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
