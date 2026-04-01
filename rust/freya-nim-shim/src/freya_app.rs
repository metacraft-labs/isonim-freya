//! Freya application lifecycle: launching the real Freya window with the
//! shadow tree renderer as the root component.
//!
//! This module is only compiled when the `freya-backend` feature is enabled.
//! It provides `launch_freya_app()` which creates a Freya window, registers
//! the shadow tree renderer component, and starts the event loop.

#[cfg(feature = "freya-backend")]
use freya::prelude::*;

#[cfg(feature = "freya-backend")]
use crate::render_sync::freya_render::shadow_tree_app;

#[cfg(feature = "freya-backend")]
use crate::window;

/// The ID of the window that is currently being displayed by Freya.
/// Set before launching the event loop so that the root component and
/// event handlers can reference the correct window in the registry.
#[cfg(feature = "freya-backend")]
static ACTIVE_WINDOW_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// Get the active Freya window ID (0 if none).
#[cfg(feature = "freya-backend")]
pub fn active_window_id() -> u32 {
    ACTIVE_WINDOW_ID.load(std::sync::atomic::Ordering::Acquire)
}

/// Launch a Freya application window that renders the shadow tree.
///
/// This function:
/// 1. Creates a `LaunchConfig` with the specified title and size
/// 2. Sets `shadow_tree_app` as the root component
/// 3. Starts the Freya event loop (blocking)
///
/// The shadow tree should already be populated before calling this function
/// (typically via the `root_builder` callback in `freya_launch`).
///
/// When a `window_id` is provided (non-zero), the window state machine is
/// updated: the window transitions to Visible before the event loop starts
/// and to Closed after the event loop returns (i.e. the user closed the window).
///
/// # Arguments
/// * `title` - Window title
/// * `width` - Initial window width in pixels
/// * `height` - Initial window height in pixels
/// * `window_id` - The window registry ID (from `freya_create_window`), or 0
#[cfg(feature = "freya-backend")]
pub fn launch_freya_app(title: &str, width: f64, height: f64, window_id: u32) {
    // Record the active window so components can reference it.
    ACTIVE_WINDOW_ID.store(window_id, std::sync::atomic::Ordering::Release);

    // Transition window to Visible before entering the event loop.
    if window_id != 0 {
        window::show_window(window_id);
    }

    // LaunchConfig requires 'static lifetime for the title string.
    // Since launch_freya_app is called once and blocks (event loop), leaking
    // the title string is acceptable -- it lives for the program's duration.
    let title_static: &'static str = Box::leak(title.to_string().into_boxed_str());
    launch_cfg(
        shadow_tree_app,
        LaunchConfig::<()>::new()
            .with_title(title_static)
            .with_size(width, height),
    );

    // The event loop has returned -- the user closed the window.
    if window_id != 0 {
        window::close_window(window_id);
    }

    ACTIVE_WINDOW_ID.store(0, std::sync::atomic::Ordering::Release);
}

#[cfg(test)]
#[cfg(feature = "freya-backend")]
mod tests {
    // Integration tests for the Freya app would require a display server
    // or freya-testing. For now, we verify the module compiles correctly
    // and the public API is accessible.

    use super::*;

    #[test]
    fn test_launch_freya_app_exists() {
        // Verify the function exists and has the right signature.
        // We can't actually call it in tests because it starts an event loop.
        let _f: fn(&str, f64, f64, u32) = launch_freya_app;
    }

    #[test]
    fn test_active_window_id_default() {
        assert_eq!(active_window_id(), 0);
    }
}
