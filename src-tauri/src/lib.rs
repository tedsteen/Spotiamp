use std::sync::OnceLock;

use librespot::{core::SpotifyId, metadata::Track};
use serde::Serialize;
use spotify::SpotifyPlayer;
use tauri::{async_runtime::spawn, Listener, Manager, PhysicalSize, Size};
use tokio::sync::Mutex;
mod oauth;
mod settings;
mod spotify;

#[tauri::command]
async fn get_volume() -> Result<u16, ()> {
    let spotify_player = &mut player().lock().await;
    Ok(spotify_player.get_volume())
}

#[derive(Debug, Serialize)]
struct TrackData {
    artist: String,
    name: String,
    duration: String,
}

impl From<Track> for TrackData {
    fn from(track: Track) -> Self {
        Self {
            artist: track
                .artists
                .first()
                .map(|artist| artist.name.clone())
                .unwrap_or("Unknown Artist".to_string()),
            name: track.name,
            duration: format!("{}", track.duration),
        }
    }
}

#[tauri::command]
async fn play() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;
    spotify_player
        .play()
        .await
        .map_err(|e| format!("TODO: Failed to play ({e:?})"))?;

    Ok(())
}

#[tauri::command]
async fn pause() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;

    spotify_player
        .pause()
        .await
        .map_err(|e| format!("TODO: Failed to pause ({e:?})"))?;

    Ok(())
}

#[tauri::command]
async fn stop() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;

    spotify_player
        .stop()
        .await
        .map_err(|e| format!("TODO: Failed to stop ({e:?})"))?;

    Ok(())
}

#[tauri::command]
async fn load(uri: &str) -> Result<TrackData, String> {
    let spotify_player = &mut player().lock().await;

    let track = spotify_player
        .load(
            SpotifyId::from_uri(uri)
                .map_err(|e| format!("TODO: Failed to load spotify uri '{uri}' ({e:?})"))?,
        )
        .await
        .map_err(|e| format!("Could not load track ({e:?})"))?;
    let track_data = TrackData::from(track);
    log::trace!("Loaded track: {track_data:?}");
    Ok(track_data)
}

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
            load, play, pause, stop, get_volume
        ])
        .setup(|app| {
            app.listen("volume-change", move |event| {
                if let Ok(volume) = serde_json::from_str::<u16>(event.payload()) {
                    spawn(async move {
                        player().lock().await.set_volume(volume);
                    });
                }
            });

            for (_, w) in app.webview_windows() {
                if let Ok(size) = w.outer_size() {
                    let zoom = 2.0;
                    w.set_size(Size::Physical(PhysicalSize {
                        width: (size.width as f32 * zoom) as u32,
                        height: (size.height as f32 * zoom) as u32,
                    }))
                    .unwrap();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
