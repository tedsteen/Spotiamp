use std::sync::OnceLock;

use spotify::{SessionError, SpotifyPlayer};
use tauri::{async_runtime::spawn, AppHandle, Listener};
use thiserror::Error;
use tokio::sync::Mutex;
mod oauth;
mod player_window;
mod playlist_window;
mod settings;
mod sink;
mod spotify;
mod visualizer;

pub fn player() -> &'static Mutex<SpotifyPlayer> {
    static MEM: OnceLock<Mutex<SpotifyPlayer>> = OnceLock::new();
    MEM.get_or_init(|| Mutex::new(SpotifyPlayer::new()))
}

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
enum StartError {
    #[error("Failed to create {window_name} window ({e:?}")]
    WindowCreationFailed {
        window_name: String,
        e: tauri::Error,
    },

    #[error("Failed to login ({e:?}")]
    LoginFailed { e: SessionError },

    #[error("Failed to position windows ({e:?}")]
    WindowsPositioningFailed { e: tauri::Error },
}

async fn start_app(app_handle: &AppHandle) -> Result<(), StartError> {
    player()
        .lock()
        .await
        .login(app_handle)
        .await
        .map_err(|e| StartError::LoginFailed { e })?;

    let zoom = 2.0;

    let player_window = player_window::build_window(app_handle, zoom).map_err(|e| {
        StartError::WindowCreationFailed {
            window_name: "Player".to_string(),
            e,
        }
    })?;

    let mut playlist_position = player_window
        .outer_position()
        .map_err(|e| StartError::WindowsPositioningFailed { e })?;
    playlist_position.y += player_window
        .outer_size()
        .map_err(|e| StartError::WindowsPositioningFailed { e })?
        .height as i32;
    playlist_window::build_window(
        app_handle,
        zoom,
        playlist_position.to_logical(
            player_window
                .scale_factor()
                .map_err(|e| StartError::WindowsPositioningFailed { e })?,
        ),
    )
    .map_err(|e| StartError::WindowCreationFailed {
        window_name: "Playlist".to_string(),
        e,
    })?;
    Ok(())
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
            player_window::get_volume,
            player_window::take_latest_spectrum,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_app(&app_handle).await {
                    log::error!("Failed to start ({e:?})");
                    app_handle.exit(1);
                }
            });
            app.listen("volume-change", move |event| {
                if let Ok(volume) = serde_json::from_str::<u16>(event.payload()) {
                    spawn(async move {
                        player().lock().await.set_volume(volume);
                    });
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
