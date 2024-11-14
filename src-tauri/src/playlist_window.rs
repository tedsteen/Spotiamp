use tauri::{AppHandle, LogicalPosition, WebviewWindow};

pub fn build_window(
    app: &AppHandle,
    zoom: f64,
    position: LogicalPosition<f64>,
) -> Result<WebviewWindow, tauri::Error> {
    let height = 116.0 * zoom;
    let width = 275.0 * zoom;
    #[cfg(target_os = "windows")]
    let (width, height) = {
        // Compensate for missing titlebar and something on the width. See https://github.com/tauri-apps/tauri/issues/6333
        // TODO: Figure out actual compensation, this is probably going to differ between users
        (width - 12.0, height - 35.0)
    };

    let window_builder = tauri::WebviewWindowBuilder::new(
        app,
        "playlist",
        tauri::WebviewUrl::App("playlist".into()),
    )
    .title("Playlist")
    .inner_size(width, height)
    .decorations(false)
    .closable(false)
    .maximizable(false)
    .minimizable(false)
    .resizable(false);

    #[cfg(target_os = "windows")]
    let window_builder = { window_builder.transparent(true) };

    window_builder
        .position(position.x, position.y)
        .disable_drag_drop_handler()
        .accept_first_mouse(true)
        .build()
}
