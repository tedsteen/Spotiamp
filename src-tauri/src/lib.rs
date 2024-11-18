use std::sync::OnceLock;

use librespot::playback::player::PlayerEvent;
use serde::{Deserialize, Serialize};
use spotify::{SessionError, SpotifyPlayer};
use tauri::{AppHandle, Emitter, Listener};
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
}

#[derive(Clone, Serialize)]
enum SpotiampPlayerEvent {
    Stopped { uri: String },
    Paused { uri: String, position_ms: u32 },
    EndOfTrack { uri: String },
    PositionCorrection { uri: String, position_ms: u32 },
    Seeked { uri: String, position_ms: u32 },
    TrackChanged { uri: String },
    Playing { uri: String, position_ms: u32 },
}

#[derive(Clone, Deserialize)]
enum PlayerWindowEvent {
    CloseRequested,
}

#[derive(Clone, Deserialize)]
enum PlaylistWindowEvent {
    Ready,
}

async fn start_app(app_handle: &AppHandle) -> Result<(), StartError> {
    let p = player().lock().await;
    p.login(app_handle)
        .await
        .map_err(|e| StartError::LoginFailed { e })?;

    let mut channel = p.get_player_event_channel();
    let player_window =
        player_window::build_window(app_handle).map_err(|e| StartError::WindowCreationFailed {
            window_name: "Player".to_string(),
            e,
        })?;
    let player_window_ref = player_window.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(player_event) = channel.recv().await {
            if let Some(player_event) = match player_event {
                PlayerEvent::Playing {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::Playing {
                    uri: track_id.to_uri().expect("a valid uri"),
                    position_ms,
                }),
                PlayerEvent::Stopped { track_id, .. } => Some(SpotiampPlayerEvent::Stopped {
                    uri: track_id.to_uri().expect("a valid uri"),
                }),
                PlayerEvent::Paused {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::Paused {
                    uri: track_id.to_uri().expect("a valid uri"),
                    position_ms,
                }),
                PlayerEvent::EndOfTrack { track_id, .. } => Some(SpotiampPlayerEvent::EndOfTrack {
                    uri: track_id.to_uri().expect("a valid uri"),
                }),
                PlayerEvent::PositionCorrection {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::PositionCorrection {
                    uri: track_id.to_uri().expect("a valid uri"),
                    position_ms,
                }),
                PlayerEvent::Seeked {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::Seeked {
                    uri: track_id.to_uri().expect("a valid uri"),
                    position_ms,
                }),
                PlayerEvent::TrackChanged { audio_item } => {
                    Some(SpotiampPlayerEvent::TrackChanged {
                        uri: audio_item.track_id.to_uri().expect("a valid uri"),
                    })
                }
                _ => None,
            } {
                let _ = player_window_ref.emit("player", player_event);
            }
        }
    });

    let app_handle = app_handle.clone();
    app_handle.clone().listen("playerWindow", move |event| {
        match serde_json::from_str::<PlayerWindowEvent>(event.payload()) {
            Ok(e) => match e {
                PlayerWindowEvent::CloseRequested => {
                    std::process::exit(0);
                }
            },
            Err(e) => log::debug!(
                "Could not deserialize playlistWindow event: '{:?}' ({e:?}) - ignoring",
                event.payload()
            ),
        }
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            player_window::get_track_metadata,
            player_window::load_track,
            player_window::get_track_ids,
            player_window::play,
            player_window::pause,
            player_window::stop,
            player_window::get_player_settings,
            player_window::set_volume,
            player_window::set_double_size,
            player_window::take_latest_spectrum,
            player_window::seek,
            player_window::set_playlist_window_visible,
            playlist_window::get_playlist_settings,
            playlist_window::add_uri,
            playlist_window::set_playlist_inner_size,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_app(&app_handle).await {
                    log::error!("Failed to start ({e:?})");
                    app_handle.exit(1);
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
