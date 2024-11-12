use librespot::{
    core::SpotifyId,
    metadata::{audio::AudioItem, Track},
};
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::player;

#[derive(Debug, Clone, Serialize)]
pub struct TrackData {
    uri: String,
    artist: String,
    name: String,
    duration: u32,
}
impl TrackData {
    pub fn new(track_id: SpotifyId, artist: &str, name: &str, duration: u32) -> Self {
        Self {
            uri: track_id.to_uri().expect("a valid uri"),
            artist: artist.to_string(),
            name: name.to_string(),
            duration,
        }
    }
}
impl From<&Track> for TrackData {
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
        )
    }
}
impl From<AudioItem> for TrackData {
    fn from(value: AudioItem) -> Self {
        let artist = match value.unique_fields {
            librespot::metadata::audio::UniqueFields::Track { artists, .. } => artists
                .iter()
                .find(|artist_with_role| {
                    matches!(
                        artist_with_role.role,
                        librespot::metadata::artist::ArtistRole::ARTIST_ROLE_MAIN_ARTIST
                    )
                })
                .map(|artist_with_role| artist_with_role.name.clone()),
            librespot::metadata::audio::UniqueFields::Episode { show_name, .. } => Some(show_name),
        };
        Self::new(
            value.track_id,
            &artist.unwrap_or("Unknown Artist".to_string()),
            &value.name,
            value.duration_ms,
        )
    }
}

#[tauri::command]
pub async fn get_volume() -> Result<u16, ()> {
    Ok(player().lock().await.get_volume())
}

#[tauri::command]
pub async fn set_volume(volume: u16) -> Result<(), ()> {
    player().lock().await.set_volume(volume);
    Ok(())
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
pub async fn get_track(uri: &str) -> Result<TrackData, String> {
    let spotify_player = &mut player().lock().await;

    Ok(TrackData::from(
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
pub async fn get_playlist_track_ids(uri: &str) -> Result<Vec<String>, String> {
    let spotify_player = &mut player().lock().await;
    Ok(spotify_player
        .get_playlist_track_ids(
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
pub fn set_playlist_window_visible(visible: bool, app: AppHandle) {
    if let Some(playlist_window) = app.get_webview_window("playlist") {
        if visible {
            playlist_window.show().expect("Playlist window to show");
        } else {
            playlist_window.hide().expect("Playlist window to hide");
        }
    } else {
        log::error!("Could not get hold of the playlist window");
    }
}

pub fn build_window(app_handle: &AppHandle, zoom: f64) -> Result<WebviewWindow, tauri::Error> {
    tauri::WebviewWindowBuilder::new(
        app_handle,
        "player",
        tauri::WebviewUrl::App("player".into()),
    )
    .title("Player")
    .inner_size(275.0 * zoom, 116.0 * zoom)
    .decorations(false)
    .closable(false)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .disable_drag_drop_handler()
    .accept_first_mouse(true)
    .build()
}
