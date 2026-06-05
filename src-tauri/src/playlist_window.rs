use tauri::{AppHandle, LogicalPosition, WebviewWindow};

use crate::{
    app_window,
    settings::{InnerWindowSize, PlaylistSettings, Settings},
};

#[tauri::command]
pub fn get_playlist_settings() -> PlaylistSettings {
    Settings::current().playlist.clone()
}

#[tauri::command]
pub fn add_uri(uri: &str) {
    Settings::current_mut().playlist.uris.push(uri.to_string());
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
