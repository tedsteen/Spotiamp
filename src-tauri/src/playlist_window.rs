use tauri::{AppHandle, LogicalPosition, WebviewWindow};

pub fn build_window(
    app: &AppHandle,
    zoom: f64,
    position: LogicalPosition<f64>,
) -> Result<WebviewWindow, tauri::Error> {
    let height = 116.0 * zoom;
    let width = 275.0 * zoom;

    #[cfg(target_os = "windows")]
    let (width, height) = crate::player_window::fix_window_size(width, height);

    tauri::WebviewWindowBuilder::new(app, "playlist", tauri::WebviewUrl::App("playlist".into()))
        .title("Playlist")
        .inner_size(width, height)
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
