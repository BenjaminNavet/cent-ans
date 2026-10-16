//! Exit deadlock guard (macOS, pipelined rendering).
//!
//! A winit window must be released on the main thread: on macOS `Window::drop` runs its close on
//! the main thread and waits for it (`run_on_main`). When the app exits, Bevy moves its own
//! `WindowWrapper`s into a list dropped on the main thread, but the render world still holds the
//! window through `ExtractedWindows` / its surface (`RawHandleWrapper` keeps an `Arc` to the
//! winit window). If the main thread lets go first, the last `Arc` dies on the render thread,
//! which then waits for the main thread, while the main thread waits for the render thread in
//! `RenderAppChannels::drop`: the process hangs at exit (1 run in 9, perf3).
//!
//! Fix: one clone of the primary window's `RawHandleWrapper` is kept in a static that is never
//! dropped, so neither thread ever drops the last reference; the OS reclaims the window when the
//! process ends.

use std::sync::{Mutex, OnceLock};

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapper};

static KEPT: OnceLock<Mutex<Vec<RawHandleWrapper>>> = OnceLock::new();

/// Number of window handles kept alive until the process ends.
pub fn kept_windows() -> usize {
    KEPT.get()
        .and_then(|kept| kept.lock().ok().map(|k| k.len()))
        .unwrap_or(0)
}

fn keep(handle: &RawHandleWrapper) {
    let kept = KEPT.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut kept) = kept.lock() {
        kept.push(handle.clone());
    }
}

/// Keeps the primary window's handle once it exists (winit creates it after the first frames).
pub fn keep_primary_window(
    windows: Query<&RawHandleWrapper, (With<PrimaryWindow>, Added<RawHandleWrapper>)>,
) {
    for handle in &windows {
        keep(handle);
    }
}
