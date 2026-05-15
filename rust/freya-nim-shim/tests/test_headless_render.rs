//! RS-M14 Phase 1 — Rust-side integration test for headless RGBA rendering.
//!
//! Drives `freya_render_to_pixels` against a small composition built
//! through the shim's real `extern "C"` shadow-tree API (no mocks),
//! and asserts:
//!
//! - return code is 0 (success);
//! - buffer length matches `width * height * 4`;
//! - the buffer contains varied content (more than one unique colour),
//!   which proves we are rendering real Freya pixels rather than a
//!   uniform white canvas;
//! - `freya_free_pixels` releases the buffer without panicking.
//!
//! The test only runs when the shim is built with the new
//! `freya-headless` Cargo feature.

#![cfg(feature = "freya-headless")]

use std::collections::HashSet;
use std::ffi::CString;

use freya_nim_shim::{
    freya_append_child, freya_create_element, freya_create_text_node,
    freya_destroy_element, freya_reset_tree, freya_set_style,
    FreyaElement, ROOT_NODE_ID,
};
use freya_nim_shim::freya_headless::{freya_free_pixels, freya_render_to_pixels};
use freya_nim_shim::tree::{Node, NodeId};
use serial_test::serial;

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Reset the shadow tree and seed a root node, returning a heap
/// handle for it. Mirrors the shim's `freya_launch` setup so the
/// `shadow_tree_app` Dioxus component finds a root to render.
unsafe fn seed_root() -> *mut FreyaElement {
    freya_reset_tree();
    // Insert a root element directly via the public tree API so we
    // can capture its NodeId for ROOT_NODE_ID.
    let root_id: NodeId = {
        let mut tree = freya_nim_shim::lock_tree();
        let root_node = Node::new_element("root");
        tree.insert(root_node)
    };
    {
        let mut root = ROOT_NODE_ID.lock().unwrap_or_else(|p| p.into_inner());
        *root = root_id;
    }
    // Hand the caller a heap-allocated handle pointing at the root.
    // (The public FreyaElement layout is `#[repr(C)] { node_id: u64 }`.)
    #[repr(C)]
    struct Local { node_id: u64 }
    let boxed = Box::new(Local { node_id: root_id.0 });
    Box::into_raw(boxed) as *mut FreyaElement
}

#[test]
#[serial]
fn headless_render_produces_non_empty_buffer() {
    unsafe {
        let root = seed_root();
        // Build: root > rect(background=red, width=100%, height=100%) > label("Hello")
        let rect_tag = c("rect");
        let label_text = c("Hello RS-M14");

        let rect = freya_create_element(rect_tag.as_ptr());
        freya_set_style(rect, c("background").as_ptr(), c("rgb(220, 40, 90)").as_ptr());
        freya_set_style(rect, c("width").as_ptr(), c("100%").as_ptr());
        freya_set_style(rect, c("height").as_ptr(), c("100%").as_ptr());
        freya_append_child(root, rect);

        let label = freya_create_text_node(label_text.as_ptr());
        freya_append_child(rect, label);

        let mut out_ptr: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let width: u32 = 100;
        let height: u32 = 100;
        let rc = freya_render_to_pixels(width, height, 1.0, &mut out_ptr, &mut out_len);
        assert_eq!(rc, 0, "freya_render_to_pixels returned non-zero error code");
        assert!(!out_ptr.is_null(), "out_ptr is null on success");
        assert_eq!(
            out_len,
            (width as usize) * (height as usize) * 4,
            "buffer length must be width * height * 4"
        );

        // Inspect the pixel buffer. The renderer should produce more
        // than one unique RGBA value: a uniform white canvas would
        // mean the render pipeline failed to actually paint the
        // shadow tree.
        let slice = std::slice::from_raw_parts(out_ptr, out_len);
        let mut unique = HashSet::new();
        let mut i = 0;
        while i < slice.len() {
            let rgba = (slice[i], slice[i + 1], slice[i + 2], slice[i + 3]);
            unique.insert(rgba);
            if unique.len() > 8 {
                break;
            }
            i += 4;
        }
        assert!(
            unique.len() > 1,
            "expected the rendered buffer to contain more than one unique RGBA value, got {}",
            unique.len()
        );

        // Look for at least one pixel whose red channel is dominant —
        // we styled the rect background as `rgb(220, 40, 90)`, so
        // there should be a visibly red region somewhere in the
        // raster.
        let mut saw_red = false;
        let mut j = 0;
        while j < slice.len() {
            let r = slice[j];
            let g = slice[j + 1];
            let b = slice[j + 2];
            if r > 150 && g < 90 && b < 130 {
                saw_red = true;
                break;
            }
            j += 4;
        }
        assert!(
            saw_red,
            "expected at least one red pixel (rgb(220, 40, 90) styled rect) in the rendered buffer"
        );

        freya_free_pixels(out_ptr, out_len);

        // Cleanup handles we created. Tree state is reset on the next
        // test iteration via `freya_reset_tree`.
        freya_destroy_element(rect);
        freya_destroy_element(label);
        freya_destroy_element(root);
    }
}

#[test]
#[serial]
fn headless_render_rejects_zero_dimensions() {
    unsafe {
        let _root = seed_root();
        let mut out_ptr: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;
        let rc = freya_render_to_pixels(0, 100, 1.0, &mut out_ptr, &mut out_len);
        assert_ne!(rc, 0, "zero width should produce an error");
        assert!(out_ptr.is_null());
        assert_eq!(out_len, 0);
    }
}

#[test]
#[serial]
fn headless_free_pixels_is_null_safe() {
    unsafe {
        // Should not panic / segfault.
        freya_free_pixels(std::ptr::null_mut(), 0);
        freya_free_pixels(std::ptr::null_mut(), 1024);
    }
}
