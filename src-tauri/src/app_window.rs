use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Deserialize;
use tauri::{
    AppHandle, Listener, LogicalPosition, Manager, PhysicalPosition, PhysicalSize, WebviewWindow,
};

use crate::settings::InnerWindowSize;

const RESIZE_ATTACHMENT_VERIFY_DELAY: Duration = Duration::from_millis(150);

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

#[derive(Clone, Copy)]
struct WindowRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl WindowRect {
    fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }
}

struct DockingState {
    positions: HashMap<String, PhysicalPosition<i32>>,
    sizes: HashMap<String, PhysicalSize<u32>>,
    suppressed_positions: HashMap<String, PhysicalPosition<i32>>,
    attachment: Option<Attachment>,
    native_child_attached: bool,
}

impl DockingState {
    fn new(windows: [&WebviewWindow; 2]) -> Self {
        Self {
            positions: windows
                .iter()
                .filter_map(|window| {
                    Some((window.label().to_string(), window.outer_position().ok()?))
                })
                .collect(),
            sizes: windows
                .iter()
                .filter_map(|window| Some((window.label().to_string(), window.outer_size().ok()?)))
                .collect(),
            suppressed_positions: Default::default(),
            attachment: None,
            native_child_attached: false,
        }
    }

    fn refresh_window(&mut self, window: &WebviewWindow) -> Option<WindowRect> {
        self.positions
            .insert(window.label().to_string(), window.outer_position().ok()?);
        self.sizes
            .insert(window.label().to_string(), window.outer_size().ok()?);
        self.rect(window.label())
    }

    fn rect(&self, label: &str) -> Option<WindowRect> {
        let position = self.positions.get(label)?;
        let size = self.sizes.get(label)?;
        Some(WindowRect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        })
    }
}

#[derive(Clone, Copy)]
enum DockedEdge {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy)]
struct Attachment {
    edge: DockedEdge,
    offset_x: i32,
    offset_y: i32,
}

impl Attachment {
    fn new(anchor: WindowRect, follower: WindowRect) -> Option<Self> {
        Some(Self {
            edge: docked_edge(follower, anchor)?,
            offset_x: follower.x - anchor.x,
            offset_y: follower.y - anchor.y,
        })
    }

    fn follower_position(
        &self,
        anchor: WindowRect,
        follower_size: PhysicalSize<u32>,
    ) -> PhysicalPosition<i32> {
        match self.edge {
            DockedEdge::Left => PhysicalPosition::new(anchor.right(), anchor.y + self.offset_y),
            DockedEdge::Top => PhysicalPosition::new(anchor.x + self.offset_x, anchor.bottom()),
            DockedEdge::Right => PhysicalPosition::new(
                anchor.x - follower_size.width as i32,
                anchor.y + self.offset_y,
            ),
            DockedEdge::Bottom => PhysicalPosition::new(
                anchor.x + self.offset_x,
                anchor.y - follower_size.height as i32,
            ),
        }
    }
}

#[derive(Deserialize)]
enum DockingWindowEvent {
    DragStarted,
    DragEnded,
}

fn ranges_overlap(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> bool {
    a_start < b_end && b_start < a_end
}

fn ranges_touch(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> bool {
    a_start <= b_end && b_start <= a_end
}

fn docked_edge(window: WindowRect, other: WindowRect) -> Option<DockedEdge> {
    let vertically_touches = ranges_touch(window.y, window.bottom(), other.y, other.bottom());
    let horizontally_touches = ranges_touch(window.x, window.right(), other.x, other.right());

    if vertically_touches && window.right() == other.x {
        Some(DockedEdge::Right)
    } else if vertically_touches && window.x == other.right() {
        Some(DockedEdge::Left)
    } else if horizontally_touches && window.bottom() == other.y {
        Some(DockedEdge::Bottom)
    } else if horizontally_touches && window.y == other.bottom() {
        Some(DockedEdge::Top)
    } else {
        None
    }
}

fn snap_position(window: WindowRect, other: WindowRect) -> Option<PhysicalPosition<i32>> {
    const SNAP_DISTANCE: i32 = 10;

    let mut candidates = [
        (
            (window.right() - other.x).abs(),
            ranges_overlap(window.y, window.bottom(), other.y, other.bottom()),
            PhysicalPosition::new(other.x - window.width as i32, window.y),
        ),
        (
            (window.x - other.right()).abs(),
            ranges_overlap(window.y, window.bottom(), other.y, other.bottom()),
            PhysicalPosition::new(other.right(), window.y),
        ),
        (
            (window.bottom() - other.y).abs(),
            ranges_overlap(window.x, window.right(), other.x, other.right()),
            PhysicalPosition::new(window.x, other.y - window.height as i32),
        ),
        (
            (window.y - other.bottom()).abs(),
            ranges_overlap(window.x, window.right(), other.x, other.right()),
            PhysicalPosition::new(window.x, other.bottom()),
        ),
    ];

    candidates.sort_by_key(|candidate| candidate.0);
    let (distance, overlaps, position) = candidates[0];

    if overlaps && distance <= SNAP_DISTANCE {
        Some(position)
    } else {
        None
    }
}

fn move_window(state: &mut DockingState, window: &WebviewWindow, position: PhysicalPosition<i32>) {
    state.positions.insert(window.label().to_string(), position);
    state
        .suppressed_positions
        .insert(window.label().to_string(), position);
    let _ = window.set_position(position);
}

#[cfg(target_os = "macos")]
fn set_native_child_window(
    anchor_window: &WebviewWindow,
    follower_window: &WebviewWindow,
    attached: bool,
) {
    let anchor_window = anchor_window.clone();
    let follower_window = follower_window.clone();

    let _ = anchor_window.clone().run_on_main_thread(move || {
        use objc2_app_kit::{NSWindow, NSWindowOrderingMode};

        let (Ok(anchor_window), Ok(follower_window)) =
            (anchor_window.ns_window(), follower_window.ns_window())
        else {
            return;
        };

        // Tauri exposes AppKit handles as raw pointers. The windows are owned by Tauri;
        // this only changes their parent/child relationship on the main thread.
        let anchor_window = unsafe { &*(anchor_window as *mut NSWindow) };
        let follower_window = unsafe { &*(follower_window as *mut NSWindow) };

        if attached {
            unsafe {
                anchor_window.addChildWindow_ordered(follower_window, NSWindowOrderingMode::Above);
            }
        } else {
            anchor_window.removeChildWindow(follower_window);
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn set_native_child_window(
    _anchor_window: &WebviewWindow,
    _follower_window: &WebviewWindow,
    _attached: bool,
) {
}

#[cfg(target_os = "macos")]
fn native_docking_moves_follower() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
fn native_docking_moves_follower() -> bool {
    false
}

fn set_attachment(
    state: &mut DockingState,
    anchor_window: &WebviewWindow,
    follower_window: &WebviewWindow,
    attachment: Option<Attachment>,
) {
    state.attachment = attachment;
    let attached = attachment.is_some();
    if state.native_child_attached != attached {
        set_native_child_window(anchor_window, follower_window, attached);
        state.native_child_attached = attached;
    }
}

fn current_attachment(
    state: &DockingState,
    anchor_label: &str,
    follower_label: &str,
) -> Option<Attachment> {
    state
        .rect(anchor_label)
        .zip(state.rect(follower_label))
        .and_then(|(anchor, follower)| Attachment::new(anchor, follower))
}

fn commit_attachment_for_window(
    state: &mut DockingState,
    anchor_window: &WebviewWindow,
    follower_window: &WebviewWindow,
    moved_window: &WebviewWindow,
) {
    let Some(anchor) = state.rect(anchor_window.label()) else {
        return;
    };
    let Some(follower) = state.rect(follower_window.label()) else {
        return;
    };

    if let Some(attachment) =
        current_attachment(state, anchor_window.label(), follower_window.label())
    {
        set_attachment(state, anchor_window, follower_window, Some(attachment));
        return;
    }

    let (moving_rect, other_rect) = if moved_window.label() == anchor_window.label() {
        (anchor, follower)
    } else {
        (follower, anchor)
    };
    if let Some(position) = snap_position(moving_rect, other_rect) {
        let snapped_window = WindowRect {
            x: position.x,
            y: position.y,
            ..moving_rect
        };
        move_window(state, moved_window, position);

        let attachment = if moved_window.label() == anchor_window.label() {
            Attachment::new(snapped_window, follower)
        } else {
            Attachment::new(anchor, snapped_window)
        };
        set_attachment(state, anchor_window, follower_window, attachment);
    } else {
        set_attachment(state, anchor_window, follower_window, None);
    }
}

fn snap_window_without_attachment(
    state: &mut DockingState,
    window: &WebviewWindow,
    other_window: &WebviewWindow,
) {
    let Some(window_rect) = state.rect(window.label()) else {
        return;
    };
    let Some(other_rect) = state.rect(other_window.label()) else {
        return;
    };
    if let Some(position) = snap_position(window_rect, other_rect) {
        move_window(state, window, position);
    }
}

fn handle_window_moved(
    anchor_window: &WebviewWindow,
    follower_window: &WebviewWindow,
    moved_window: &WebviewWindow,
    position: PhysicalPosition<i32>,
    anchor_moved: bool,
    state: &mut DockingState,
) {
    let label = moved_window.label();
    let had_position = state
        .positions
        .insert(label.to_string(), position)
        .is_some();

    if let Some(suppressed_position) = state.suppressed_positions.remove(label)
        && suppressed_position == position
    {
        return;
    }

    if !had_position {
        return;
    }

    state.refresh_window(anchor_window);
    state.refresh_window(follower_window);
    state.positions.insert(label.to_string(), position);

    let Some(anchor) = state.rect(anchor_window.label()) else {
        return;
    };
    if anchor_moved {
        if let Some(attachment) = state.attachment {
            if native_docking_moves_follower() {
                return;
            }

            let Some(follower_size) = state.sizes.get(follower_window.label()).copied() else {
                return;
            };
            move_window(
                state,
                follower_window,
                attachment.follower_position(anchor, follower_size),
            );
            return;
        }
    } else {
        if let Some(attachment) = state.attachment {
            let Some(follower_size) = state.sizes.get(follower_window.label()).copied() else {
                return;
            };
            if position == attachment.follower_position(anchor, follower_size) {
                return;
            }
        }

        set_attachment(state, anchor_window, follower_window, None);
        snap_window_without_attachment(state, follower_window, anchor_window);
    }
}

fn handle_window_resized(
    anchor_window: &WebviewWindow,
    follower_window: &WebviewWindow,
    resized_window: &WebviewWindow,
    size: PhysicalSize<u32>,
    anchor_resized: bool,
    state: &mut DockingState,
) {
    state.sizes.insert(resized_window.label().to_string(), size);
    state.refresh_window(anchor_window);
    state.refresh_window(follower_window);

    if !anchor_resized {
        commit_attachment_for_window(state, anchor_window, follower_window, follower_window);
        return;
    }

    let Some(attachment) = state.attachment else {
        return;
    };

    let Some(anchor) = state.rect(anchor_window.label()) else {
        return;
    };
    let Some(follower_size) = state.sizes.get(follower_window.label()).copied() else {
        return;
    };

    move_window(
        state,
        follower_window,
        attachment.follower_position(anchor, follower_size),
    );
}

fn verify_attachment_after_resize(
    anchor_window: WebviewWindow,
    follower_window: WebviewWindow,
    state: Arc<Mutex<DockingState>>,
) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(RESIZE_ATTACHMENT_VERIFY_DELAY).await;

        let mut state = state.lock().expect("docking state lock");
        if state.attachment.is_none() {
            return;
        }

        state.refresh_window(&anchor_window);
        state.refresh_window(&follower_window);

        let attachment = current_attachment(&state, anchor_window.label(), follower_window.label());
        set_attachment(&mut state, &anchor_window, &follower_window, attachment);
    });
}

pub fn dock_windows(anchor_window: &WebviewWindow, follower_window: &WebviewWindow) {
    let state = Arc::new(Mutex::new(DockingState::new([
        anchor_window,
        follower_window,
    ])));

    {
        let mut state = state.lock().expect("docking state lock");
        state.refresh_window(anchor_window);
        state.refresh_window(follower_window);
        commit_attachment_for_window(&mut state, anchor_window, follower_window, follower_window);
    }

    for (window, anchor_moved) in [
        (anchor_window.clone(), true),
        (follower_window.clone(), false),
    ] {
        let anchor_window = anchor_window.clone();
        let follower_window = follower_window.clone();
        let state = state.clone();
        window.clone().on_window_event(move |window_event| {
            let state_handle = state.clone();
            let mut state = state.lock().expect("docking state lock");
            match window_event {
                tauri::WindowEvent::Moved(position) => {
                    handle_window_moved(
                        &anchor_window,
                        &follower_window,
                        &window,
                        *position,
                        anchor_moved,
                        &mut state,
                    );
                }
                tauri::WindowEvent::Resized(size) => {
                    handle_window_resized(
                        &anchor_window,
                        &follower_window,
                        &window,
                        *size,
                        anchor_moved,
                        &mut state,
                    );
                    if anchor_moved {
                        verify_attachment_after_resize(
                            anchor_window.clone(),
                            follower_window.clone(),
                            state_handle,
                        );
                    }
                }
                _ => {}
            }
        });
    }

    for (event_name, moved_window) in [
        ("playerWindow", anchor_window.clone()),
        ("playlistWindow", follower_window.clone()),
    ] {
        let anchor_window = anchor_window.clone();
        let follower_window = follower_window.clone();
        let state = state.clone();
        let app_handle = anchor_window.app_handle().clone();
        app_handle.listen(event_name, move |event| {
            let Ok(event) = serde_json::from_str::<DockingWindowEvent>(event.payload()) else {
                return;
            };

            let mut state = state.lock().expect("docking state lock");
            state.refresh_window(&anchor_window);
            state.refresh_window(&follower_window);

            match event {
                DockingWindowEvent::DragStarted => {
                    if moved_window.label() == follower_window.label() {
                        set_attachment(&mut state, &anchor_window, &follower_window, None);
                    }
                }
                DockingWindowEvent::DragEnded => {
                    commit_attachment_for_window(
                        &mut state,
                        &anchor_window,
                        &follower_window,
                        &moved_window,
                    );
                }
            }
        });
    }
}
