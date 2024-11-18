use tauri::{AppHandle, PhysicalPosition, WebviewWindow};

use crate::settings::{InnerWindowSize, OuterWindowPosition, PlaylistSettings, Settings};

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
    initial_position: PhysicalPosition<i32>,
) -> Result<WebviewWindow, tauri::Error> {
    let inner_size = Settings::current()
        .playlist
        .window_state
        .inner_size
        .clone()
        .unwrap_or_default();

    let mut window_builder = tauri::WebviewWindowBuilder::new(
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
    .accept_first_mouse(true);

    if let Some(outer_position) = &Settings::current().playlist.window_state.outer_position {
        window_builder = window_builder.position(outer_position.x as f64, outer_position.y as f64);
    } else {
        window_builder =
            window_builder.position(initial_position.x as f64, initial_position.y as f64);
    }

    let window = window_builder.build()?;
    window.on_window_event(move |window_event| {
        if let tauri::WindowEvent::Moved(physical_position) = &window_event {
            Settings::current_mut().playlist.window_state.outer_position =
                Some(OuterWindowPosition {
                    x: physical_position.x,
                    y: physical_position.y,
                });
        }
    });
    Ok(window)
}
