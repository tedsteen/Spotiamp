use librespot::{core::SpotifyId, metadata::Track};
use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::{
    playlist_window,
    settings::{PlayerSettings, Settings},
    spotify::SharedPlayer,
};

#[derive(Debug, Clone, Serialize)]
pub struct TrackMetadata {
    uri: String,
    artist: String,
    name: String,
    duration: u32,
    unavailable: bool,
}
impl TrackMetadata {
    pub fn new(
        track_id: SpotifyId,
        artist: &str,
        name: &str,
        duration: u32,
        unavailable: bool,
    ) -> Self {
        Self {
            unavailable,
            uri: track_id.to_uri().expect("a valid uri"),
            artist: artist.to_string(),
            name: name.to_string(),
            duration,
        }
    }
}
impl From<&Track> for TrackMetadata {
    fn from(track: &Track) -> Self {
        Self::new(
            track.id,
            &track
                .artists
                .first()
                .map(|artist| artist.name.clone())
                .unwrap_or("Unknown Artist".to_string()),
            &track.name,
            track.duration as u32,
            !track.restrictions.is_empty() && track.alternatives.is_empty(),
        )
    }
}

#[tauri::command]
pub fn get_player_settings() -> PlayerSettings {
    Settings::current().player.clone()
}

#[tauri::command]
pub async fn set_volume(volume: u16, player: State<'_, SharedPlayer>) -> Result<(), ()> {
    player.lock().await.set_volume(volume);
    Settings::current_mut().player.volume = volume;
    Ok(())
}

#[tauri::command]
pub fn set_double_size(active: bool) {
    Settings::current_mut().player.double_size_active = active;
}

#[tauri::command]
pub async fn take_latest_spectrum(player: State<'_, SharedPlayer>) -> Result<Vec<(f32, f32)>, ()> {
    Ok(player.lock().await.take_latest_spectrum())
}

#[tauri::command]
pub async fn load_track(uri: &str, player: State<'_, SharedPlayer>) -> Result<(), String> {
    player
        .lock()
        .await
        .load_track(uri)
        .await
        .map_err(|e| format!("TODO: Failed to load track ({e:?})"))
}

#[tauri::command]
pub async fn play(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.lock().await.play();

    Ok(())
}

#[tauri::command]
pub async fn pause(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player
        .lock()
        .await
        .pause()
        .await
        .map_err(|e| format!("TODO: Failed to pause ({e:?})"))?;

    Ok(())
}

#[tauri::command]
pub async fn stop(player: State<'_, SharedPlayer>) -> Result<(), String> {
    player
        .lock()
        .await
        .stop()
        .await
        .map_err(|e| format!("TODO: Failed to stop ({e:?})"))?;

    Ok(())
}

#[tauri::command]
pub async fn get_track_metadata(
    uri: &str,
    player: State<'_, SharedPlayer>,
) -> Result<TrackMetadata, String> {
    Ok(TrackMetadata::from(
        &player
            .lock()
            .await
            .get_track(
                SpotifyId::from_uri(uri)
                    .map_err(|e| format!("TODO: Failed to get track by uri '{uri}' ({e:?})"))?,
            )
            .await
            .map_err(|e| format!("Could not load track ({e:?})"))?,
    ))
}

#[tauri::command]
pub async fn get_track_ids(
    uri: &str,
    player: State<'_, SharedPlayer>,
) -> Result<Vec<String>, String> {
    Ok(player
        .lock()
        .await
        .get_track_ids(
            SpotifyId::from_uri(uri)
                .map_err(|e| format!("TODO: Failed to get playlist by uri '{uri}' ({e:?})"))?,
        )
        .await
        .map_err(|e| format!("Could not load playlist tracks ({e:?})"))?
        .iter()
        .map(|track_id| track_id.to_uri().expect("a valid uri"))
        .collect())
}

#[tauri::command]
pub async fn seek(position_ms: u32, player: State<'_, SharedPlayer>) -> Result<(), String> {
    player.lock().await.seek(position_ms);
    Ok(())
}

//NOTE: The command needs to be async for Windows to be able to create new windows in it.
//      See https://github.com/tauri-apps/tauri/issues/4121 for details
#[tauri::command]
pub async fn set_playlist_window_visible(visible: bool, app_handle: AppHandle) -> Result<(), ()> {
    let playlist_window = app_handle
        .get_webview_window("playlist")
        .unwrap_or_else(|| {
            let player_window = app_handle
                .get_webview_window("player")
                .expect("a player window");
            let mut initial_position = player_window
                .outer_position()
                .expect("a position for the player window");
            initial_position.y += player_window
                .outer_size()
                .expect("a player window position")
                .height as i32;

            playlist_window::build_window(
                &app_handle,
                initial_position.to_logical(
                    player_window
                        .scale_factor()
                        .expect("a scalefactor on the player window"),
                ),
            )
            .expect("a playlist window to be created")
        });
    Settings::current_mut().player.show_playlist = visible;
    if visible {
        playlist_window.show().expect("Playlist window to show");
    } else {
        playlist_window.hide().expect("Playlist window to hide");
    }
    Ok(())
}

pub fn build_window(app_handle: &AppHandle) -> Result<WebviewWindow, tauri::Error> {
    let inner_size = Settings::current()
        .player
        .window_state
        .inner_size
        .clone()
        .unwrap_or_default();
    let window = tauri::WebviewWindowBuilder::new(
        app_handle,
        "player",
        tauri::WebviewUrl::App("player".into()),
    )
    .title("Player")
    .inner_size(inner_size.width as f64, inner_size.height as f64)
    .decorations(false)
    .shadow(false)
    .closable(false)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .disable_drag_drop_handler()
    .accept_first_mouse(true)
    .build()?;

    if let Some(logical_position) = &Settings::current().player.window_state.get_position() {
        let _ = window.set_position(*logical_position);
    }

    window.on_window_event({
        let window = window.clone();
        move |window_event| {
            if let tauri::WindowEvent::Moved(physical_position) = &window_event {
                Settings::current_mut().player.window_state.set_position(
                    physical_position.to_logical(
                        window
                            .scale_factor()
                            .expect("a scale factor on the player window"),
                    ),
                );
            }
        }
    });

    Ok(window)
}
