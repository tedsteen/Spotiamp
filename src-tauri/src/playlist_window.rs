use tauri::{AppHandle, LogicalPosition, WebviewWindow};

use crate::PLAYER_SIZE;

pub fn build_window(
    app: &AppHandle,
    zoom: f64,
    position: LogicalPosition<f64>,
) -> Result<WebviewWindow, tauri::Error> {
    tauri::WebviewWindowBuilder::new(app, "playlist", tauri::WebviewUrl::App("playlist".into()))
        .title("Playlist")
        .inner_size(PLAYER_SIZE.0 * zoom, PLAYER_SIZE.1 * zoom)
        .decorations(false)
        .shadow(false)
        .closable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        .resizable(false)
        .position(position.x, position.y)
        .disable_drag_drop_handler()
        .accept_first_mouse(true)
        .build()
}
