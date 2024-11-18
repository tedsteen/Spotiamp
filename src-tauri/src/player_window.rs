use librespot::{core::SpotifyId, metadata::Track};
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::{
    player, playlist_window,
    settings::{OuterWindowPosition, PlayerSettings, Settings},
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
pub async fn set_volume(volume: u16) -> Result<(), ()> {
    player().lock().await.set_volume(volume);
    Settings::current_mut().player.volume = volume;
    Ok(())
}

#[tauri::command]
pub fn set_double_size(active: bool) {
    Settings::current_mut().player.double_size_active = active;
}

#[tauri::command]
pub async fn take_latest_spectrum() -> Result<Vec<(f32, f32)>, ()> {
    let spotify_player = &mut player().lock().await;
    Ok(spotify_player.take_latest_spectrum())
}

#[tauri::command]
pub async fn load_track(uri: &str) -> Result<(), String> {
    let spotify_player = &mut player().lock().await;
    spotify_player
        .load_track(uri)
        .await
        .map_err(|e| format!("TODO: Failed to load track ({e:?})"))
}

#[tauri::command]
pub async fn play() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;
    spotify_player.play();

    Ok(())
}

#[tauri::command]
pub async fn pause() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;

    spotify_player
        .pause()
        .await
        .map_err(|e| format!("TODO: Failed to pause ({e:?})"))?;

    Ok(())
}

#[tauri::command]
pub async fn stop() -> Result<(), String> {
    let spotify_player = &mut player().lock().await;

    spotify_player
        .stop()
        .await
        .map_err(|e| format!("TODO: Failed to stop ({e:?})"))?;

    Ok(())
}

#[tauri::command]
pub async fn get_track_metadata(uri: &str) -> Result<TrackMetadata, String> {
    let spotify_player = &mut player().lock().await;

    Ok(TrackMetadata::from(
        &spotify_player
            .get_track(
                SpotifyId::from_uri(uri)
                    .map_err(|e| format!("TODO: Failed to get track by uri '{uri}' ({e:?})"))?,
            )
            .await
            .map_err(|e| format!("Could not load track ({e:?})"))?,
    ))
}

#[tauri::command]
pub async fn get_track_ids(uri: &str) -> Result<Vec<String>, String> {
    let spotify_player = &mut player().lock().await;
    Ok(spotify_player
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
pub async fn seek(position_ms: u32) -> Result<(), String> {
    let spotify_player = &mut player().lock().await;

    spotify_player.seek(position_ms);

    Ok(())
}

#[tauri::command]
pub fn set_playlist_window_visible(visible: bool, app_handle: AppHandle) {
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
                .expect("a player windoow position")
                .height as i32;

            playlist_window::build_window(
                &app_handle,
                initial_position.to_logical(
                    player_window
                        .scale_factor()
                        .expect("A scale factor on the player window"),
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
}

pub fn build_window(app_handle: &AppHandle) -> Result<WebviewWindow, tauri::Error> {
    let inner_size = Settings::current()
        .player
        .window_state
        .inner_size
        .clone()
        .unwrap_or_default();
    let mut window_builder = tauri::WebviewWindowBuilder::new(
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
    .accept_first_mouse(true);

    if let Some(outer_position) = &Settings::current().player.window_state.outer_position {
        window_builder = window_builder.position(outer_position.x as f64, outer_position.y as f64);
    }

    let window = window_builder.build()?;
    let scale_factor = window.scale_factor().expect("a scale factor on the window");
    window.on_window_event(move |window_event| {
        if let tauri::WindowEvent::Moved(physical_position) = &window_event {
            let logical_position = physical_position.to_logical(scale_factor);
            Settings::current_mut().player.window_state.outer_position =
                Some(OuterWindowPosition {
                    x: logical_position.x,
                    y: logical_position.y,
                });
        }
    });

    Ok(window)
}
