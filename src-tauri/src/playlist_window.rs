use tauri::{AppHandle, LogicalPosition, WebviewWindow, State};

use crate::{
    app_window,
    settings::{InnerWindowSize, PlaylistSettings, Settings},
    spotify::SharedPlayer,
};

#[tauri::command]
pub async fn get_spotify_access_token(
    player: State<'_, SharedPlayer>,
) -> Result<String, String> {
    let player = player.lock().await;
    match player
        .session
        .inner
        .token_provider()
        .get_token("playlist-read-private,playlist-read-collaborative,user-library-read")
        .await
    {
        Ok(token) => Ok(token.access_token),
        Err(e) => Err(format!("Failed to get Spotify token: {:?}", e)),
    }
}


#[tauri::command]
pub fn get_playlist_settings() -> PlaylistSettings {
    Settings::current().playlist.clone()
}

#[tauri::command]
pub fn set_uris(uris: Vec<String>) {
    Settings::current_mut().playlist.uris = uris;
}

#[tauri::command]
pub fn set_playlist_inner_size(width: u32, height: u32) {
    Settings::current_mut().playlist.window_state.inner_size =
        Some(InnerWindowSize { width, height });
}

pub fn build_window(
    app: &AppHandle,
    initial_position: LogicalPosition<i32>,
) -> Result<WebviewWindow, tauri::Error> {
    let inner_size = Settings::current()
        .playlist
        .window_state
        .inner_size
        .clone()
        .unwrap_or_default();

    let window =
        app_window::build_frameless_window(app, "playlist", "Playlist", "playlist", inner_size)?;

    app_window::apply_position(
        &window,
        Some(
            Settings::current()
                .playlist
                .window_state
                .get_position()
                .unwrap_or(initial_position),
        ),
    );
    app_window::remember_position(&window, "playlist window", |position| {
        Settings::current_mut()
            .playlist
            .window_state
            .set_position(position);
    });
    Ok(window)
}
