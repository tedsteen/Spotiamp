use std::sync::OnceLock;

use librespot::playback::player::PlayerEvent;
use player_window::TrackData;
use serde::{Deserialize, Serialize};
use spotify::{SessionError, SpotifyPlayer};
use tauri::{AppHandle, Emitter, Listener, Manager};
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
    Stopped { id: u128 },
    Paused { id: u128, position_ms: u32 },
    EndOfTrack { id: u128 },
    PositionCorrection { id: u128, position_ms: u32 },
    Seeked { id: u128, position_ms: u32 },
    TrackChanged(TrackData),
    Playing { id: u128, position_ms: u32 },
}

#[derive(Clone, Deserialize)]
enum PlayerWindowEvent {
    Ready,
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

    let zoom = 2.0;
    let mut channel = p.get_player_event_channel();
    let player_window = player_window::build_window(app_handle, zoom).map_err(|e| {
        StartError::WindowCreationFailed {
            window_name: "Player".to_string(),
            e,
        }
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
                    id: track_id.id,
                    position_ms,
                }),
                PlayerEvent::Stopped { track_id, .. } => {
                    Some(SpotiampPlayerEvent::Stopped { id: track_id.id })
                }
                PlayerEvent::Paused {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::Paused {
                    id: track_id.id,
                    position_ms,
                }),
                PlayerEvent::EndOfTrack { track_id, .. } => {
                    Some(SpotiampPlayerEvent::EndOfTrack { id: track_id.id })
                }
                PlayerEvent::PositionCorrection {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::PositionCorrection {
                    id: track_id.id,
                    position_ms,
                }),
                PlayerEvent::Seeked {
                    track_id,
                    position_ms,
                    ..
                } => Some(SpotiampPlayerEvent::Seeked {
                    id: track_id.id,
                    position_ms,
                }),
                PlayerEvent::TrackChanged { audio_item } => Some(
                    SpotiampPlayerEvent::TrackChanged(TrackData::from(*audio_item)),
                ),
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
                PlayerWindowEvent::Ready => {
                    if app_handle.get_webview_window("playlist").is_none() {
                        let mut playlist_position = player_window
                            .outer_position()
                            .expect("a position for the player window");
                        playlist_position.y += player_window
                            .outer_size()
                            .expect("a player windoow position")
                            .height as i32;
                        playlist_window::build_window(
                            &app_handle,
                            zoom,
                            playlist_position.to_logical(
                                player_window
                                    .scale_factor()
                                    .expect("a logical playlist position"),
                            ),
                        )
                        .expect("a playlist window to be created");
                    }
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
            player_window::get_track,
            player_window::load_track,
            player_window::play,
            player_window::pause,
            player_window::stop,
            player_window::get_volume,
            player_window::set_volume,
            player_window::take_latest_spectrum,
            player_window::seek,
            player_window::set_playlist_window_visible,
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
