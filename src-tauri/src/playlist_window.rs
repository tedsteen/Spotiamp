use tauri::{AppHandle, LogicalPosition, WebviewWindow};

pub fn build_window(
    app: &AppHandle,
    zoom: f64,
    position: LogicalPosition<f64>,
) -> Result<WebviewWindow, tauri::Error> {
    tauri::WebviewWindowBuilder::new(app, "playlist", tauri::WebviewUrl::App("playlist".into()))
        .title("Playlist")
        .inner_size(275.0 * zoom, 116.0 * zoom)
        .decorations(false)
        .closable(false)
        .maximizable(false)
        .minimizable(false)
        .resizable(false)
        .position(position.x, position.y)
        .disable_drag_drop_handler()
        .accept_first_mouse(true)
        .visible(false)
        .build()
}
