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
/// # Arguments
/// * `title` - Window title
/// * `width` - Initial window width in pixels
/// * `height` - Initial window height in pixels
#[cfg(feature = "freya-backend")]
pub fn launch_freya_app(title: &str, width: f64, height: f64) {
    // LaunchConfig requires 'static lifetime for the title string.
    // Since launch_freya_app is called once and blocks (event loop), leaking
    // the title string is acceptable — it lives for the program's duration.
    let title_static: &'static str = Box::leak(title.to_string().into_boxed_str());
    launch_cfg(
        shadow_tree_app,
        LaunchConfig::<()>::new()
            .with_title(title_static)
            .with_size(width, height),
    );
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
        let _f: fn(&str, f64, f64) = launch_freya_app;
    }
}
