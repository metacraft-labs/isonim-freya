//! C FFI shim for Freya, exposing the 13 procs required by IsoNim's RendererBackend.
//!
//! Each function uses `extern "C"` with `#[no_mangle]` so Nim can link against the
//! compiled cdylib. Element handles are opaque pointers managed on the Rust side.
//!
//! This is a placeholder implementation — the actual Freya integration will replace
//! the stub bodies with real Dioxus VirtualDom operations.

use std::ffi::CStr;
use std::os::raw::c_char;

/// Opaque handle to a Freya element (placeholder — will wrap Dioxus ElementId or similar).
#[repr(C)]
pub struct FreyaElement {
    _opaque: u64,
}

/// Create a new element with the given tag name.
#[no_mangle]
pub extern "C" fn freya_create_element(tag: *const c_char) -> *mut FreyaElement {
    let _tag = unsafe { CStr::from_ptr(tag) };
    // TODO: create Freya element from tag
    std::ptr::null_mut()
}

/// Create a text node with the given content.
#[no_mangle]
pub extern "C" fn freya_create_text_node(text: *const c_char) -> *mut FreyaElement {
    let _text = unsafe { CStr::from_ptr(text) };
    // TODO: create Freya text node
    std::ptr::null_mut()
}

/// Append `child` as the last child of `parent`.
#[no_mangle]
pub extern "C" fn freya_append_child(
    _parent: *mut FreyaElement,
    _child: *mut FreyaElement,
) {
    // TODO: implement
}

/// Insert `child` before `reference` within `parent`.
#[no_mangle]
pub extern "C" fn freya_insert_before(
    _parent: *mut FreyaElement,
    _child: *mut FreyaElement,
    _reference: *mut FreyaElement,
) {
    // TODO: implement
}

/// Remove `child` from `parent`.
#[no_mangle]
pub extern "C" fn freya_remove_child(
    _parent: *mut FreyaElement,
    _child: *mut FreyaElement,
) {
    // TODO: implement
}

/// Set attribute `name` to `value` on `node`.
#[no_mangle]
pub extern "C" fn freya_set_attribute(
    _node: *mut FreyaElement,
    _name: *const c_char,
    _value: *const c_char,
) {
    // TODO: implement
}

/// Remove attribute `name` from `node`.
#[no_mangle]
pub extern "C" fn freya_remove_attribute(
    _node: *mut FreyaElement,
    _name: *const c_char,
) {
    // TODO: implement
}

/// Set the text content of `node`.
#[no_mangle]
pub extern "C" fn freya_set_text_content(
    _node: *mut FreyaElement,
    _text: *const c_char,
) {
    // TODO: implement
}

/// Set a style property on `node`.
#[no_mangle]
pub extern "C" fn freya_set_style(
    _node: *mut FreyaElement,
    _prop: *const c_char,
    _value: *const c_char,
) {
    // TODO: implement
}

/// Register a callback for `event` on `node`.
/// The `handler` is a C function pointer that Nim will pass in.
pub type EventCallback = extern "C" fn();

#[no_mangle]
pub extern "C" fn freya_add_event_listener(
    _node: *mut FreyaElement,
    _event: *const c_char,
    _handler: EventCallback,
) {
    // TODO: implement
}

/// Return the first child of `node`, or null.
#[no_mangle]
pub extern "C" fn freya_first_child(
    _node: *mut FreyaElement,
) -> *mut FreyaElement {
    // TODO: implement
    std::ptr::null_mut()
}

/// Return the next sibling of `node`, or null.
#[no_mangle]
pub extern "C" fn freya_next_sibling(
    _node: *mut FreyaElement,
) -> *mut FreyaElement {
    // TODO: implement
    std::ptr::null_mut()
}

/// Return the parent of `node`, or null.
#[no_mangle]
pub extern "C" fn freya_parent_node(
    _node: *mut FreyaElement,
) -> *mut FreyaElement {
    // TODO: implement
    std::ptr::null_mut()
}
