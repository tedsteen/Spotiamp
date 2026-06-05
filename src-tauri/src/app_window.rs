use tauri::{AppHandle, LogicalPosition, WebviewWindow};

use crate::settings::InnerWindowSize;

pub fn build_frameless_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    route: &str,
    inner_size: InnerWindowSize,
) -> Result<WebviewWindow, tauri::Error> {
    tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(route.into()))
        .title(title)
        .inner_size(inner_size.width as f64, inner_size.height as f64)
        .decorations(false)
        .shadow(false)
        .closable(false)
        .maximizable(false)
        .minimizable(false)
        .resizable(false)
        .disable_drag_drop_handler()
        .accept_first_mouse(true)
        .build()
}

pub fn apply_position(window: &WebviewWindow, position: Option<LogicalPosition<i32>>) {
    if let Some(position) = position {
        let _ = window.set_position(position);
    }
}

pub fn remember_position(
    window: &WebviewWindow,
    scale_factor_context: &'static str,
    save_position: impl Fn(LogicalPosition<i32>) + Send + 'static,
) {
    let window = window.clone();
    window.clone().on_window_event(move |window_event| {
        if let tauri::WindowEvent::Moved(physical_position) = window_event {
            save_position(
                physical_position.to_logical(
                    window.scale_factor().unwrap_or_else(|_| {
                        panic!("a scale factor for the {scale_factor_context}")
                    }),
                ),
            );
        }
    });
}
