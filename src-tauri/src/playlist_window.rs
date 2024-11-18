use tauri::{AppHandle, LogicalPosition, WebviewWindow};

use crate::settings::{InnerWindowSize, PlaylistSettings, Settings};

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

    let window = tauri::WebviewWindowBuilder::new(
        app,
        "playlist",
        tauri::WebviewUrl::App("playlist".into()),
    )
    .title("Playlist")
    .inner_size(inner_size.width as f64, inner_size.height as f64)
    .decorations(false)
    .shadow(false)
    .closable(false)
    .maximizable(false)
    .minimizable(false)
    .shadow(false)
    .resizable(false)
    .disable_drag_drop_handler()
    .accept_first_mouse(true)
    .build()?;

    let _ = window.set_position(
        Settings::current()
            .playlist
            .window_state
            .get_position()
            .unwrap_or(initial_position),
    );

    window.on_window_event({
        let window = window.clone();
        move |window_event| {
            if let tauri::WindowEvent::Moved(physical_position) = &window_event {
                Settings::current_mut().playlist.window_state.set_position(
                    physical_position.to_logical(
                        window
                            .scale_factor()
                            .expect("a scale factor for the playlist window"),
                    ),
                );
            }
        }
    });
    Ok(window)
}
