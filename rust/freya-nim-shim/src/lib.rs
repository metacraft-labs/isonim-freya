//! C FFI shim for Freya, exposing the 13 procs required by IsoNim's RendererBackend.
//!
//! Each function uses `extern "C"` with `#[no_mangle]` so Nim can link against the
//! compiled cdylib. Element handles are opaque pointers managed on the Rust side.
//!
//! ## Architecture
//!
//! Freya (built on Dioxus) is declarative — UI is expressed via RSX macros and
//! reactive state. IsoNim needs imperative tree manipulation. We bridge this by:
//!
//! 1. Maintaining a **shadow tree** of `Node` structs in Rust (`tree` module).
//! 2. Exposing the 13 RendererBackend operations as `extern "C"` functions that
//!    manipulate this tree imperatively.
//! 3. A separate render-sync step (M2+) will translate the shadow tree into
//!    Freya's declarative model for actual rendering.
//!
//! Element handles (`*mut FreyaElement`) are thin wrappers around `NodeId` values.
//! They are heap-allocated so the Nim side can hold them as opaque pointers.

pub mod tree;
pub mod window;
pub mod render_sync;
#[cfg(feature = "freya-backend")]
mod freya_app;
/// RS-M14 Phase 1: headless RGBA rendering via freya-testing's Skia
/// raster path. The module's `#[no_mangle]` entry points
/// (`freya_render_to_pixels`, `freya_free_pixels`) are exported as
/// part of the cdylib's C ABI when the `freya-headless` feature is
/// enabled.
#[cfg(feature = "freya-headless")]
pub mod freya_headless;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Mutex;

use tree::{EventListener, Node, NodeId, NodeKind, Tree};
use window::{CloseCallback, FocusCallback, ResizeCallback};

/// Global shadow tree protected by a mutex.
/// All extern "C" functions lock this to perform tree operations.
pub static TREE: std::sync::LazyLock<Mutex<Tree>> =
    std::sync::LazyLock::new(|| Mutex::new(Tree::new()));

/// Global root node ID for the render-sync bridge.
/// Set by `freya_launch()` so the Freya component knows which node is the root.
pub static ROOT_NODE_ID: std::sync::LazyLock<Mutex<NodeId>> =
    std::sync::LazyLock::new(|| Mutex::new(NodeId::NULL));

/// Global event dispatcher function pointer, registered by Nim via
/// `freya_set_event_dispatcher`. When set, event listeners that have a
/// `callback_id > 0` are dispatched through this function instead of
/// calling a C function pointer directly.
pub type EventDispatcher = extern "C" fn(callback_id: i32);

static EVENT_DISPATCHER: std::sync::LazyLock<Mutex<Option<EventDispatcher>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Lock the global tree, recovering from poison if needed.
/// Since the tree is always in a valid (if inconsistent) state after a panic,
/// we simply clear the poison and continue.
pub fn lock_tree() -> std::sync::MutexGuard<'static, Tree> {
    match TREE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Opaque handle to a Freya element. Wraps a NodeId.
/// Allocated on the heap via Box so Nim holds a stable pointer.
#[repr(C)]
pub struct FreyaElement {
    node_id: u64,
}

/// Helper: convert a raw C string pointer to a Rust &str.
/// Returns "" if the pointer is null or not valid UTF-8.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => s,
        Err(_) => "",
    }
}

/// Helper: allocate a FreyaElement handle on the heap for the given NodeId.
/// Returns null if the id is NULL.
fn node_id_to_handle(id: NodeId) -> *mut FreyaElement {
    if id.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(FreyaElement { node_id: id.0 }))
}

/// Helper: extract NodeId from a handle pointer. Returns NodeId::NULL if null.
unsafe fn handle_to_node_id(handle: *mut FreyaElement) -> NodeId {
    if handle.is_null() {
        NodeId::NULL
    } else {
        NodeId((*handle).node_id)
    }
}

// ---------------------------------------------------------------------------
// 1. createElement
// ---------------------------------------------------------------------------

/// Create a new element with the given tag name.
///
/// Returns a heap-allocated handle that the caller (Nim) must hold.
/// The element is added to the global shadow tree but not attached to any parent.
#[no_mangle]
pub extern "C" fn freya_create_element(tag: *const c_char) -> *mut FreyaElement {
    let tag_str = unsafe { cstr_to_str(tag) };
    let node = Node::new_element(tag_str);
    let mut tree = lock_tree();
    let id = tree.insert(node);
    node_id_to_handle(id)
}

// ---------------------------------------------------------------------------
// 2. createTextNode
// ---------------------------------------------------------------------------

/// Create a text node with the given content.
#[no_mangle]
pub extern "C" fn freya_create_text_node(text: *const c_char) -> *mut FreyaElement {
    let text_str = unsafe { cstr_to_str(text) };
    let node = Node::new_text(text_str);
    let mut tree = lock_tree();
    let id = tree.insert(node);
    node_id_to_handle(id)
}

// ---------------------------------------------------------------------------
// 3. appendChild
// ---------------------------------------------------------------------------

/// Append `child` as the last child of `parent`.
#[no_mangle]
pub extern "C" fn freya_append_child(parent: *mut FreyaElement, child: *mut FreyaElement) {
    let parent_id = unsafe { handle_to_node_id(parent) };
    let child_id = unsafe { handle_to_node_id(child) };
    if parent_id.is_null() || child_id.is_null() {
        return;
    }
    let mut tree = lock_tree();
    tree.append_child(parent_id, child_id);
    window::request_repaint();
}

// ---------------------------------------------------------------------------
// 4. insertBefore
// ---------------------------------------------------------------------------

/// Insert `child` before `reference` within `parent`.
/// If `reference` is null, appends child instead.
#[no_mangle]
pub extern "C" fn freya_insert_before(
    parent: *mut FreyaElement,
    child: *mut FreyaElement,
    reference: *mut FreyaElement,
) {
    let parent_id = unsafe { handle_to_node_id(parent) };
    let child_id = unsafe { handle_to_node_id(child) };
    let ref_id = unsafe { handle_to_node_id(reference) };
    if parent_id.is_null() || child_id.is_null() {
        return;
    }
    let mut tree = lock_tree();
    tree.insert_before(parent_id, child_id, ref_id);
    window::request_repaint();
}

// ---------------------------------------------------------------------------
// 5. removeChild
// ---------------------------------------------------------------------------

/// Remove `child` from `parent`.
#[no_mangle]
pub extern "C" fn freya_remove_child(parent: *mut FreyaElement, child: *mut FreyaElement) {
    let parent_id = unsafe { handle_to_node_id(parent) };
    let child_id = unsafe { handle_to_node_id(child) };
    if parent_id.is_null() || child_id.is_null() {
        return;
    }
    let mut tree = lock_tree();
    tree.remove_child(parent_id, child_id);
    window::request_repaint();
}

// ---------------------------------------------------------------------------
// 6. setAttribute
// ---------------------------------------------------------------------------

/// Set attribute `name` to `value` on `node`.
#[no_mangle]
pub extern "C" fn freya_set_attribute(
    node: *mut FreyaElement,
    name: *const c_char,
    value: *const c_char,
) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let name_str = unsafe { cstr_to_str(name) };
    let value_str = unsafe { cstr_to_str(value) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        n.attributes
            .insert(name_str.to_string(), value_str.to_string());
        window::request_repaint();
    }
}

// ---------------------------------------------------------------------------
// 7. removeAttribute
// ---------------------------------------------------------------------------

/// Remove attribute `name` from `node`.
#[no_mangle]
pub extern "C" fn freya_remove_attribute(node: *mut FreyaElement, name: *const c_char) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let name_str = unsafe { cstr_to_str(name) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        n.attributes.remove(name_str);
        window::request_repaint();
    }
}

// ---------------------------------------------------------------------------
// 8. setTextContent
// ---------------------------------------------------------------------------

/// Set the text content of `node`.
#[no_mangle]
pub extern "C" fn freya_set_text_content(node: *mut FreyaElement, text: *const c_char) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let text_str = unsafe { cstr_to_str(text) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        n.set_text_content(text_str);
        window::request_repaint();
    }
}

// ---------------------------------------------------------------------------
// 9. setStyle
// ---------------------------------------------------------------------------

/// Set a style property on `node`.
#[no_mangle]
pub extern "C" fn freya_set_style(
    node: *mut FreyaElement,
    prop: *const c_char,
    value: *const c_char,
) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let prop_str = unsafe { cstr_to_str(prop) };
    let value_str = unsafe { cstr_to_str(value) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        n.styles
            .insert(prop_str.to_string(), value_str.to_string());
        window::request_repaint();
    }
}

// ---------------------------------------------------------------------------
// 10. addEventListener
// ---------------------------------------------------------------------------

/// C function pointer type for event callbacks from Nim.
pub type EventCallback = extern "C" fn();

/// Register a callback for `event` on `node`.
/// The `handler` is a C function pointer that Nim will pass in.
/// Legacy API — uses direct function pointer dispatch (callback_id = 0).
#[no_mangle]
pub extern "C" fn freya_add_event_listener(
    node: *mut FreyaElement,
    event: *const c_char,
    handler: EventCallback,
) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let event_str = unsafe { cstr_to_str(event) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        let listener = EventListener {
            callback: handler,
            callback_id: 0,
        };
        n.event_listeners
            .entry(event_str.to_string())
            .or_default()
            .push(listener);
    }
}

/// No-op function pointer used as placeholder for dispatcher-based listeners.
extern "C" fn _noop_callback() {}

/// Register a callback ID for `event` on `node`.
/// The `callback_id` is dispatched via the global event dispatcher registered
/// by `freya_set_event_dispatcher`.
#[no_mangle]
pub extern "C" fn freya_add_event_listener_id(
    node: *mut FreyaElement,
    event: *const c_char,
    callback_id: i32,
) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let event_str = unsafe { cstr_to_str(event) };
    let mut tree = lock_tree();
    if let Some(n) = tree.get_mut(node_id) {
        let listener = EventListener {
            callback: _noop_callback,
            callback_id,
        };
        n.event_listeners
            .entry(event_str.to_string())
            .or_default()
            .push(listener);
    }
}

/// Register the global event dispatcher function.
/// When set, event listeners with `callback_id > 0` are dispatched through
/// this function instead of calling their C function pointer directly.
#[no_mangle]
pub extern "C" fn freya_set_event_dispatcher(dispatcher: EventDispatcher) {
    let mut guard = EVENT_DISPATCHER.lock().unwrap_or_else(|p| p.into_inner());
    *guard = Some(dispatcher);
}

// ---------------------------------------------------------------------------
// 11. firstChild
// ---------------------------------------------------------------------------

/// Return the first child of `node`, or null if it has no children.
#[no_mangle]
pub extern "C" fn freya_first_child(node: *mut FreyaElement) -> *mut FreyaElement {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return std::ptr::null_mut();
    }
    let tree = lock_tree();
    let child_id = tree.first_child(node_id);
    node_id_to_handle(child_id)
}

// ---------------------------------------------------------------------------
// 12. nextSibling
// ---------------------------------------------------------------------------

/// Return the next sibling of `node`, or null.
#[no_mangle]
pub extern "C" fn freya_next_sibling(node: *mut FreyaElement) -> *mut FreyaElement {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return std::ptr::null_mut();
    }
    let tree = lock_tree();
    let sibling_id = tree.next_sibling(node_id);
    node_id_to_handle(sibling_id)
}

// ---------------------------------------------------------------------------
// 13. parentNode
// ---------------------------------------------------------------------------

/// Return the parent of `node`, or null.
#[no_mangle]
pub extern "C" fn freya_parent_node(node: *mut FreyaElement) -> *mut FreyaElement {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return std::ptr::null_mut();
    }
    let tree = lock_tree();
    let parent_id = tree.parent_node(node_id);
    node_id_to_handle(parent_id)
}

// ---------------------------------------------------------------------------
// Window / event loop management
// ---------------------------------------------------------------------------

/// Launch a Freya window.
///
/// This creates a root element in the shadow tree and starts the Freya event loop.
/// The `title` parameter sets the window title.
/// The `width` and `height` parameters set the initial window size.
/// The `root_builder` callback is called with the root element handle so the
/// Nim side can build the initial tree before the event loop starts.
///
/// **Note:** In M1 this is a placeholder that creates the root element and calls
/// the builder callback but does NOT start an actual Freya window (that requires
/// the full Freya dependency to be available at link time). The actual Freya
/// integration will be completed in M2.
pub type RootBuilderCallback = extern "C" fn(root: *mut FreyaElement);

#[no_mangle]
pub extern "C" fn freya_launch(
    title: *const c_char,
    width: f64,
    height: f64,
    root_builder: RootBuilderCallback,
) {
    let title_str = unsafe { cstr_to_str(title) };

    // Create a root element in the shadow tree
    let root_node = Node::new_element("root");
    let root_id = {
        let mut tree = lock_tree();
        tree.insert(root_node)
    };

    // Store the root ID globally so the Freya component can find it
    {
        let mut root = ROOT_NODE_ID.lock().unwrap_or_else(|p| p.into_inner());
        *root = root_id;
    }

    let root_handle = node_id_to_handle(root_id);

    // Call back to Nim so it can build the initial tree
    root_builder(root_handle);

    // When the freya-backend feature is enabled, launch the actual Freya
    // window with the shadow tree renderer as the root component.
    #[cfg(feature = "freya-backend")]
    {
        // Create a window in the registry so that the Freya component can
        // reference it for lifecycle events (resize, focus, close).
        let win_id = window::create_window(title_str, width, height);
        freya_app::launch_freya_app(title_str, width, height, win_id);
    }

    // Without the feature, the function returns after building the shadow tree.
    // This is the existing behavior used for testing and headless operation.
    #[cfg(not(feature = "freya-backend"))]
    {
        let _ = title_str;
    }
}

/// RS-M14 Phase 1: set the global `ROOT_NODE_ID` to the given element
/// handle. Callers that build the shadow tree *without* going through
/// `freya_launch` (e.g. the Freya streaming adapter in
/// `isonim_render_serve`) must call this before invoking
/// `freya_render_to_pixels` so the headless `shadow_tree_app` finds the
/// correct tree root to render. Passing a null handle resets the root
/// to `NodeId::NULL`, which causes the renderer to fall back to its
/// "No shadow tree root found" placeholder element.
#[no_mangle]
pub extern "C" fn freya_set_root_element(handle: *mut FreyaElement) {
    let node_id = unsafe { handle_to_node_id(handle) };
    let mut root = ROOT_NODE_ID.lock().unwrap_or_else(|p| p.into_inner());
    *root = node_id;
}

/// Trigger all event listeners for the given event on the given node.
/// This is called by the Freya event loop (M2+) when an event occurs,
/// or can be called directly for testing.
#[no_mangle]
pub extern "C" fn freya_dispatch_event(node: *mut FreyaElement, event: *const c_char) {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return;
    }
    let event_str = unsafe { cstr_to_str(event) };

    // Collect listeners while holding the tree lock, then dispatch after releasing.
    // This avoids deadlock when callbacks modify the tree.
    let listeners: Vec<(extern "C" fn(), i32)> = {
        let tree = lock_tree();
        if let Some(n) = tree.get(node_id) {
            n.event_listeners
                .get(event_str)
                .map(|ls| ls.iter().map(|l| (l.callback, l.callback_id)).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    // Read the dispatcher once (outside the tree lock)
    let dispatcher = {
        EVENT_DISPATCHER
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    };

    for (cb, id) in listeners {
        if id > 0 {
            if let Some(dispatch) = dispatcher {
                dispatch(id);
            }
        } else {
            cb();
        }
    }
}

/// Free a FreyaElement handle.
/// This deallocates the handle pointer but does NOT remove the node from the tree.
/// Call freya_remove_child first to detach the node, then freya_destroy_element
/// to free the handle memory.
#[no_mangle]
pub extern "C" fn freya_destroy_element(handle: *mut FreyaElement) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Remove a node and all its descendants from the shadow tree entirely.
/// This is for cleanup — it removes the node from the tree store (not just
/// from its parent's children list). The handle is also freed.
#[no_mangle]
pub extern "C" fn freya_destroy_tree(handle: *mut FreyaElement) {
    if handle.is_null() {
        return;
    }
    let node_id = unsafe { handle_to_node_id(handle) };

    // Collect all descendant IDs via BFS
    let ids_to_remove = {
        let tree = lock_tree();
        let mut to_visit = vec![node_id];
        let mut to_remove = Vec::new();
        while let Some(id) = to_visit.pop() {
            to_remove.push(id);
            if let Some(n) = tree.get(id) {
                to_visit.extend_from_slice(&n.children);
            }
        }
        to_remove
    };

    {
        let mut tree = lock_tree();
        for id in &ids_to_remove {
            tree.remove(*id);
        }
    }

    // Free the handle
    unsafe {
        drop(Box::from_raw(handle));
    }
}

/// Reset the global tree (useful for testing).
#[no_mangle]
pub extern "C" fn freya_reset_tree() {
    let mut tree = lock_tree();
    *tree = Tree::new();
    // Also reset the root node ID
    let mut root = ROOT_NODE_ID.lock().unwrap_or_else(|p| p.into_inner());
    *root = NodeId::NULL;
}

/// Get the number of nodes in the global tree (useful for debugging/testing).
#[no_mangle]
pub extern "C" fn freya_tree_node_count() -> u64 {
    let tree = lock_tree();
    tree.len() as u64
}

// ---------------------------------------------------------------------------
// Tree inspection functions (M5 — cross-renderer testing)
// ---------------------------------------------------------------------------

/// Get the number of children of a node.
/// Returns 0 if the node is null or not found.
#[no_mangle]
pub extern "C" fn freya_child_count(node: *mut FreyaElement) -> u64 {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return 0;
    }
    let tree = lock_tree();
    tree.get(node_id)
        .map(|n| n.children.len() as u64)
        .unwrap_or(0)
}

/// Get the tag name of a node. Returns 0 for text nodes.
/// Writes into `buf` if provided, returns the number of bytes needed.
///
/// Mirrors `gpui_get_tag` in `isonim-gpui` — used by the RS-M4
/// Freya streaming adapter (in `isonim-render-serve`) to derive a
/// per-element fill colour when rasterizing the headless tree to
/// RGBA pixels. The behaviour exactly matches the GPUI shim's
/// `gpui_get_tag` so the two adapters can share assertions in
/// cross-renderer parity tests.
#[no_mangle]
pub extern "C" fn freya_get_tag(
    node: *mut FreyaElement,
    buf: *mut c_char,
    buf_len: u64,
) -> u64 {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return 0;
    }
    let tree = lock_tree();
    let tag = match tree.get(node_id) {
        Some(n) => match n.tag() {
            Some(t) => t.to_string(),
            None => return 0,
        },
        None => return 0,
    };

    let bytes = tag.as_bytes();
    let needed = bytes.len() as u64;

    if buf.is_null() || buf_len == 0 {
        return needed;
    }

    let to_copy = std::cmp::min(bytes.len(), (buf_len - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, to_copy);
        *buf.add(to_copy) = 0; // null terminator
    }
    needed
}

/// Get the text content of a node and all its descendants (recursive).
/// For text nodes, returns the text. For element nodes, concatenates
/// all descendant text. The result is written into the provided buffer.
/// Returns the number of bytes written (excluding null terminator),
/// or the required buffer size if the buffer is too small.
/// If `buf` is null, returns the required size.
#[no_mangle]
pub extern "C" fn freya_get_text_content(
    node: *mut FreyaElement,
    buf: *mut c_char,
    buf_len: u64,
) -> u64 {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return 0;
    }
    let tree = lock_tree();
    let text = collect_text_content(&tree, node_id);
    let bytes = text.as_bytes();
    let needed = bytes.len() as u64;

    if buf.is_null() || buf_len == 0 {
        return needed;
    }

    let to_copy = std::cmp::min(bytes.len(), (buf_len - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, to_copy);
        *buf.add(to_copy) = 0; // null terminator
    }
    needed
}

/// Recursively collect text content from a node and its descendants.
fn collect_text_content(tree: &Tree, node_id: NodeId) -> String {
    if let Some(node) = tree.get(node_id) {
        match &node.kind {
            NodeKind::Text(text) => text.clone(),
            NodeKind::Element(_) => {
                // Check for __text_content attribute (set by setTextContent on elements)
                if let Some(tc) = node.attributes.get("__text_content") {
                    return tc.clone();
                }
                let mut result = String::new();
                for &child_id in &node.children {
                    result.push_str(&collect_text_content(tree, child_id));
                }
                result
            }
        }
    } else {
        String::new()
    }
}

/// Get an attribute value from a node.
/// Returns the number of bytes in the attribute value (excluding null),
/// or 0 if the attribute is not found. Writes into `buf` if provided.
#[no_mangle]
pub extern "C" fn freya_get_attribute(
    node: *mut FreyaElement,
    name: *const c_char,
    buf: *mut c_char,
    buf_len: u64,
) -> u64 {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return 0;
    }
    let name_str = unsafe { cstr_to_str(name) };
    let tree = lock_tree();
    let value = match tree.get(node_id) {
        Some(n) => match n.attributes.get(name_str) {
            Some(v) => v.clone(),
            None => return 0,
        },
        None => return 0,
    };

    let bytes = value.as_bytes();
    let needed = bytes.len() as u64;

    if buf.is_null() || buf_len == 0 {
        return needed;
    }

    let to_copy = std::cmp::min(bytes.len(), (buf_len - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, to_copy);
        *buf.add(to_copy) = 0;
    }
    needed
}

/// Get the Nth child of a node (0-indexed).
/// Returns null if out of bounds or node not found.
#[no_mangle]
pub extern "C" fn freya_nth_child(node: *mut FreyaElement, index: u64) -> *mut FreyaElement {
    let node_id = unsafe { handle_to_node_id(node) };
    if node_id.is_null() {
        return std::ptr::null_mut();
    }
    let tree = lock_tree();
    if let Some(n) = tree.get(node_id) {
        if (index as usize) < n.children.len() {
            return node_id_to_handle(n.children[index as usize]);
        }
    }
    std::ptr::null_mut()
}

// ---------------------------------------------------------------------------
// Window management (M4)
// ---------------------------------------------------------------------------

/// Create a new window with the given title and initial size.
/// Returns a window ID (> 0) on success, 0 on failure.
#[no_mangle]
pub extern "C" fn freya_create_window(
    title: *const c_char,
    width: f64,
    height: f64,
) -> u32 {
    let title_str = unsafe { cstr_to_str(title) };
    window::create_window(title_str, width, height)
}

/// Show a window (transition from Created to Visible state).
///
/// Without the `freya-backend` feature this just updates the internal state.
/// With the feature enabled, this starts the Freya event loop for the window
/// on a background thread.
///
/// Returns 1 on success, 0 if the window was not in Created state or not found.
#[no_mangle]
pub extern "C" fn freya_show_window(window_id: u32) -> u8 {
    if window::show_window(window_id) {
        1
    } else {
        0
    }
}

/// Request that a window be closed. If an on_close callback is registered
/// and returns 0, the close is denied.
/// Returns 1 if the window was closed, 0 if denied or not found.
#[no_mangle]
pub extern "C" fn freya_close_window(window_id: u32) -> u8 {
    if window::close_window(window_id) {
        1
    } else {
        0
    }
}

/// Destroy a window and remove it from the registry.
#[no_mangle]
pub extern "C" fn freya_destroy_window(window_id: u32) {
    window::destroy_window(window_id);
}

/// Get the current state of a window.
/// Returns: 0 = not found, 1 = Created, 2 = Visible, 3 = CloseRequested, 4 = Closed.
#[no_mangle]
pub extern "C" fn freya_window_state(window_id: u32) -> u8 {
    match window::window_state(window_id) {
        None => 0,
        Some(window::WindowState::Created) => 1,
        Some(window::WindowState::Visible) => 2,
        Some(window::WindowState::CloseRequested) => 3,
        Some(window::WindowState::Closed) => 4,
    }
}

/// Get the current width of a window. Returns 0.0 if not found.
#[no_mangle]
pub extern "C" fn freya_window_width(window_id: u32) -> f64 {
    window::window_size(window_id)
        .map(|(w, _)| w)
        .unwrap_or(0.0)
}

/// Get the current height of a window. Returns 0.0 if not found.
#[no_mangle]
pub extern "C" fn freya_window_height(window_id: u32) -> f64 {
    window::window_size(window_id)
        .map(|(_, h)| h)
        .unwrap_or(0.0)
}

/// Request a repaint of the window. This signals that the shadow tree has
/// changed and the window should re-render on the next frame.
#[no_mangle]
pub extern "C" fn freya_request_repaint() {
    window::request_repaint();
}

/// Check if a repaint has been requested (and clear the flag).
/// Returns 1 if a repaint was pending, 0 otherwise.
#[no_mangle]
pub extern "C" fn freya_take_repaint_request() -> u8 {
    if window::take_repaint_request() {
        1
    } else {
        0
    }
}

/// Register a callback for window resize events.
/// The callback receives (width: f64, height: f64).
#[no_mangle]
pub extern "C" fn freya_on_resize(
    window_id: u32,
    callback: ResizeCallback,
) {
    window::with_window_mut(window_id, |w| {
        w.on_resize = Some(callback);
    });
}

/// Register a callback for window focus events.
/// The callback receives (focused: u8) where 1 = focused, 0 = unfocused.
#[no_mangle]
pub extern "C" fn freya_on_focus(
    window_id: u32,
    callback: FocusCallback,
) {
    window::with_window_mut(window_id, |w| {
        w.on_focus = Some(callback);
    });
}

/// Register a callback for window close requests.
/// The callback should return 1 to allow close, 0 to prevent it.
#[no_mangle]
pub extern "C" fn freya_on_close(
    window_id: u32,
    callback: CloseCallback,
) {
    window::with_window_mut(window_id, |w| {
        w.on_close = Some(callback);
    });
}

/// Simulate a resize event on a window (for testing / event bridging).
#[no_mangle]
pub extern "C" fn freya_notify_resize(window_id: u32, width: f64, height: f64) {
    window::notify_resize(window_id, width, height);
}

/// Simulate a focus event on a window (for testing / event bridging).
/// `focused`: 1 = gained focus, 0 = lost focus.
#[no_mangle]
pub extern "C" fn freya_notify_focus(window_id: u32, focused: u8) {
    window::notify_focus(window_id, focused != 0);
}

/// Reset all windows (for testing).
#[no_mangle]
pub extern "C" fn freya_reset_windows() {
    window::reset_windows();
}

// ---------------------------------------------------------------------------
// Render plan inspection FFI (G3-F)
// ---------------------------------------------------------------------------

/// Build a render plan from the shadow tree rooted at `root` and return it as
/// a JSON string. The caller must free the returned string with `freya_free_string`.
///
/// Returns null if the handle is null or the node doesn't exist.
///
/// JSON format:
/// ```json
/// {
///   "kind": "Rect",
///   "text": null,
///   "has_click_handler": false,
///   "has_input_handler": false,
///   "event_names": [],
///   "styles": { "width": "100%", ... },
///   "children": [ ... ]
/// }
/// ```
#[no_mangle]
pub extern "C" fn freya_render_plan_json(root: *mut FreyaElement) -> *mut c_char {
    let node_id = unsafe { handle_to_node_id(root) };
    if node_id.is_null() {
        return std::ptr::null_mut();
    }
    let tree = lock_tree();
    let plan = match render_sync::build_render_plan(&tree, node_id) {
        Some(p) => p,
        None => return std::ptr::null_mut(),
    };
    drop(tree);

    let json = render_plan_to_json(&plan);
    match std::ffi::CString::new(json) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string returned by `freya_render_plan_json`.
#[no_mangle]
pub extern "C" fn freya_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(std::ffi::CString::from_raw(ptr));
        }
    }
}

/// Return the total element count in the render plan rooted at `root`.
/// Returns 0 if the handle is null or the node doesn't exist.
#[no_mangle]
pub extern "C" fn freya_render_plan_element_count(root: *mut FreyaElement) -> u32 {
    let node_id = unsafe { handle_to_node_id(root) };
    if node_id.is_null() {
        return 0;
    }
    let tree = lock_tree();
    match render_sync::build_render_plan(&tree, node_id) {
        Some(plan) => render_sync::count_render_nodes(&plan) as u32,
        None => 0,
    }
}

/// Verify that a render plan can be built from the shadow tree rooted at `root`.
/// Returns 1 if valid, 0 if there are issues (null handle, missing node, etc.).
#[no_mangle]
pub extern "C" fn freya_verify_render_plan(root: *mut FreyaElement) -> u8 {
    let node_id = unsafe { handle_to_node_id(root) };
    if node_id.is_null() {
        return 0;
    }
    let tree = lock_tree();
    match render_sync::build_render_plan(&tree, node_id) {
        Some(_) => 1,
        None => 0,
    }
}

/// Internal helper: serialize a RenderNode to a JSON string.
fn render_plan_to_json(plan: &render_sync::RenderNode) -> String {
    fn node_to_string(plan: &render_sync::RenderNode) -> String {
        let kind = format!("{:?}", plan.element_kind);
        let text = match &plan.text {
            Some(t) => format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\"")),
            None => "null".to_string(),
        };

        let mut style_entries = Vec::new();
        let s = &plan.styles;
        macro_rules! push_style {
            ($field:ident, $name:expr) => {
                if let Some(ref v) = s.$field {
                    style_entries.push(format!("\"{}\":\"{}\"", $name, v.replace('"', "\\\"")));
                }
            };
        }
        push_style!(width, "width");
        push_style!(height, "height");
        push_style!(min_width, "min_width");
        push_style!(min_height, "min_height");
        push_style!(max_width, "max_width");
        push_style!(max_height, "max_height");
        push_style!(padding, "padding");
        push_style!(margin, "margin");
        push_style!(background, "background");
        push_style!(color, "color");
        push_style!(font_size, "font_size");
        push_style!(font_family, "font_family");
        push_style!(font_weight, "font_weight");
        push_style!(font_style, "font_style");
        push_style!(corner_radius, "corner_radius");
        push_style!(border, "border");
        push_style!(shadow, "shadow");
        push_style!(direction, "direction");
        push_style!(main_align, "main_align");
        push_style!(cross_align, "cross_align");
        push_style!(overflow, "overflow");
        push_style!(opacity, "opacity");
        push_style!(spacing, "spacing");
        push_style!(text_align, "text_align");
        push_style!(line_height, "line_height");
        push_style!(letter_spacing, "letter_spacing");
        let styles_json = format!("{{{}}}", style_entries.join(","));

        let event_names: Vec<String> = plan
            .event_names
            .iter()
            .map(|e| format!("\"{}\"", e))
            .collect();

        let children: Vec<String> = plan.children.iter().map(node_to_string).collect();

        let attr_entries: Vec<String> = plan
            .attributes
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        let attributes_json = format!("{{{}}}", attr_entries.join(","));

        format!(
            "{{\"kind\":\"{}\",\"text\":{},\"has_click_handler\":{},\"has_input_handler\":{},\"event_names\":[{}],\"styles\":{},\"attributes\":{},\"children\":[{}]}}",
            kind,
            text,
            plan.has_click_handler,
            plan.has_input_handler,
            event_names.join(","),
            styles_json,
            attributes_json,
            children.join(","),
        )
    }
    node_to_string(plan)
}

// ---------------------------------------------------------------------------
// Rust-side tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use serial_test::serial;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Helper to create a CString and return its pointer.
    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    #[serial]
    fn test_create_element_returns_non_null() {
        freya_reset_tree();
        let tag = c("rect");
        let handle = freya_create_element(tag.as_ptr());
        assert!(!handle.is_null());
        freya_destroy_element(handle);
    }

    #[test]
    #[serial]
    fn test_create_text_node_returns_non_null() {
        freya_reset_tree();
        let text = c("hello");
        let handle = freya_create_text_node(text.as_ptr());
        assert!(!handle.is_null());
        freya_destroy_element(handle);
    }

    #[test]
    #[serial]
    fn test_append_and_traverse() {
        freya_reset_tree();
        let tag_rect = c("rect");
        let tag_label = c("label");

        let parent = freya_create_element(tag_rect.as_ptr());
        let child1 = freya_create_element(tag_label.as_ptr());
        let child2 = freya_create_element(tag_label.as_ptr());

        freya_append_child(parent, child1);
        freya_append_child(parent, child2);

        // first_child of parent should be child1
        let fc = freya_first_child(parent);
        assert!(!fc.is_null());
        assert_eq!(unsafe { (*fc).node_id }, unsafe { (*child1).node_id });
        freya_destroy_element(fc);

        // next_sibling of child1 should be child2
        let ns = freya_next_sibling(child1);
        assert!(!ns.is_null());
        assert_eq!(unsafe { (*ns).node_id }, unsafe { (*child2).node_id });
        freya_destroy_element(ns);

        // parent_node of child1 should be parent
        let pn = freya_parent_node(child1);
        assert!(!pn.is_null());
        assert_eq!(unsafe { (*pn).node_id }, unsafe { (*parent).node_id });
        freya_destroy_element(pn);

        freya_destroy_element(parent);
        freya_destroy_element(child1);
        freya_destroy_element(child2);
    }

    #[test]
    #[serial]
    fn test_insert_before() {
        freya_reset_tree();
        let tag = c("rect");

        let parent = freya_create_element(tag.as_ptr());
        let c1 = freya_create_element(tag.as_ptr());
        let c2 = freya_create_element(tag.as_ptr());
        let c3 = freya_create_element(tag.as_ptr());

        freya_append_child(parent, c1);
        freya_append_child(parent, c2);
        freya_insert_before(parent, c3, c2); // c3 before c2

        // Order should be: c1, c3, c2
        let fc = freya_first_child(parent);
        assert_eq!(unsafe { (*fc).node_id }, unsafe { (*c1).node_id });

        let ns1 = freya_next_sibling(c1);
        assert_eq!(unsafe { (*ns1).node_id }, unsafe { (*c3).node_id });

        let ns2 = freya_next_sibling(c3);
        assert_eq!(unsafe { (*ns2).node_id }, unsafe { (*c2).node_id });

        freya_destroy_element(fc);
        freya_destroy_element(ns1);
        freya_destroy_element(ns2);
        freya_destroy_element(parent);
        freya_destroy_element(c1);
        freya_destroy_element(c2);
        freya_destroy_element(c3);
    }

    #[test]
    #[serial]
    fn test_remove_child() {
        freya_reset_tree();
        let tag = c("rect");

        let parent = freya_create_element(tag.as_ptr());
        let child = freya_create_element(tag.as_ptr());

        freya_append_child(parent, child);
        freya_remove_child(parent, child);

        let fc = freya_first_child(parent);
        assert!(fc.is_null());

        let pn = freya_parent_node(child);
        assert!(pn.is_null());

        freya_destroy_element(parent);
        freya_destroy_element(child);
    }

    #[test]
    #[serial]
    fn test_set_and_remove_attribute() {
        freya_reset_tree();
        let tag = c("rect");
        let name = c("width");
        let value = c("100%");

        let node = freya_create_element(tag.as_ptr());
        freya_set_attribute(node, name.as_ptr(), value.as_ptr());

        {
            let tree = lock_tree();
            let nid = NodeId(unsafe { (*node).node_id });
            let n = tree.get(nid).unwrap();
            assert_eq!(n.attributes.get("width").map(|s| s.as_str()), Some("100%"));
        }

        freya_remove_attribute(node, name.as_ptr());

        {
            let tree = lock_tree();
            let nid = NodeId(unsafe { (*node).node_id });
            let n = tree.get(nid).unwrap();
            assert!(n.attributes.get("width").is_none());
        }

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_set_text_content() {
        freya_reset_tree();
        let text1 = c("hello");
        let text2 = c("world");

        let node = freya_create_text_node(text1.as_ptr());
        freya_set_text_content(node, text2.as_ptr());

        {
            let tree = lock_tree();
            let nid = NodeId(unsafe { (*node).node_id });
            let n = tree.get(nid).unwrap();
            assert_eq!(n.text_content(), Some("world"));
        }

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_set_style() {
        freya_reset_tree();
        let tag = c("rect");
        let prop = c("background");
        let value = c("rgb(255, 0, 0)");

        let node = freya_create_element(tag.as_ptr());
        freya_set_style(node, prop.as_ptr(), value.as_ptr());

        {
            let tree = lock_tree();
            let nid = NodeId(unsafe { (*node).node_id });
            let n = tree.get(nid).unwrap();
            assert_eq!(
                n.styles.get("background").map(|s| s.as_str()),
                Some("rgb(255, 0, 0)")
            );
        }

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_add_event_listener_and_dispatch() {
        freya_reset_tree();
        let tag = c("rect");
        let event = c("click");

        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

        extern "C" fn test_handler() {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        CALL_COUNT.store(0, Ordering::SeqCst);

        let node = freya_create_element(tag.as_ptr());
        freya_add_event_listener(node, event.as_ptr(), test_handler);

        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);

        freya_dispatch_event(node, event.as_ptr());
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

        freya_dispatch_event(node, event.as_ptr());
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_null_safety() {
        // All functions should handle null pointers gracefully
        let tag = c("rect");
        let name = c("width");
        let value = c("100");

        freya_append_child(std::ptr::null_mut(), std::ptr::null_mut());
        freya_insert_before(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        freya_remove_child(std::ptr::null_mut(), std::ptr::null_mut());
        freya_set_attribute(std::ptr::null_mut(), name.as_ptr(), value.as_ptr());
        freya_remove_attribute(std::ptr::null_mut(), name.as_ptr());
        freya_set_text_content(std::ptr::null_mut(), tag.as_ptr());
        freya_set_style(std::ptr::null_mut(), name.as_ptr(), value.as_ptr());

        extern "C" fn noop() {}
        freya_add_event_listener(std::ptr::null_mut(), name.as_ptr(), noop);

        let fc = freya_first_child(std::ptr::null_mut());
        assert!(fc.is_null());
        let ns = freya_next_sibling(std::ptr::null_mut());
        assert!(ns.is_null());
        let pn = freya_parent_node(std::ptr::null_mut());
        assert!(pn.is_null());

        freya_destroy_element(std::ptr::null_mut());
        freya_destroy_tree(std::ptr::null_mut());
    }

    #[test]
    #[serial]
    fn test_tree_node_count() {
        freya_reset_tree();
        assert_eq!(freya_tree_node_count(), 0);

        let tag = c("rect");
        let n1 = freya_create_element(tag.as_ptr());
        assert_eq!(freya_tree_node_count(), 1);

        let n2 = freya_create_element(tag.as_ptr());
        assert_eq!(freya_tree_node_count(), 2);

        freya_destroy_element(n1);
        freya_destroy_element(n2);
    }

    #[test]
    #[serial]
    fn test_create_window() {
        freya_reset_windows();
        let title = c("My Window");
        let id = freya_create_window(title.as_ptr(), 800.0, 600.0);
        assert!(id > 0);
        assert_eq!(freya_window_state(id), 1); // Created
        assert_eq!(freya_window_width(id), 800.0);
        assert_eq!(freya_window_height(id), 600.0);
        freya_destroy_window(id);
    }

    #[test]
    #[serial]
    fn test_show_and_close_window() {
        freya_reset_windows();
        let title = c("Test Window");
        let id = freya_create_window(title.as_ptr(), 640.0, 480.0);
        assert_eq!(freya_show_window(id), 1);
        assert_eq!(freya_window_state(id), 2); // Visible
        // Cannot show again
        assert_eq!(freya_show_window(id), 0);
        // Close
        assert_eq!(freya_close_window(id), 1);
        assert_eq!(freya_window_state(id), 4); // Closed
        freya_destroy_window(id);
    }

    #[test]
    #[serial]
    fn test_window_lifecycle_callbacks() {
        freya_reset_windows();
        let title = c("Callback Test");
        let id = freya_create_window(title.as_ptr(), 800.0, 600.0);

        static RESIZE_CALLED: AtomicU32 = AtomicU32::new(0);
        static FOCUS_CALLED: AtomicU32 = AtomicU32::new(0);

        extern "C" fn on_resize(_w: f64, _h: f64) {
            RESIZE_CALLED.fetch_add(1, Ordering::SeqCst);
        }
        extern "C" fn on_focus(_f: u8) {
            FOCUS_CALLED.fetch_add(1, Ordering::SeqCst);
        }
        extern "C" fn deny_close() -> u8 {
            0
        }

        RESIZE_CALLED.store(0, Ordering::SeqCst);
        FOCUS_CALLED.store(0, Ordering::SeqCst);

        freya_on_resize(id, on_resize);
        freya_on_focus(id, on_focus);
        freya_on_close(id, deny_close);

        freya_show_window(id);

        // Trigger events
        freya_notify_resize(id, 1024.0, 768.0);
        assert_eq!(RESIZE_CALLED.load(Ordering::SeqCst), 1);
        assert_eq!(freya_window_width(id), 1024.0);
        assert_eq!(freya_window_height(id), 768.0);

        freya_notify_focus(id, 1);
        assert_eq!(FOCUS_CALLED.load(Ordering::SeqCst), 1);

        // Close should be denied
        assert_eq!(freya_close_window(id), 0);
        assert_eq!(freya_window_state(id), 2); // Still Visible

        freya_destroy_window(id);
    }

    #[test]
    #[serial]
    fn test_repaint_on_tree_mutation() {
        freya_reset_tree();
        freya_reset_windows();
        // Clear any pending repaint
        freya_take_repaint_request();

        let tag = c("rect");
        let parent = freya_create_element(tag.as_ptr());
        // create_element does not request repaint (element isn't visible yet)
        // but append_child does
        let child = freya_create_element(tag.as_ptr());
        freya_take_repaint_request(); // clear

        freya_append_child(parent, child);
        assert_eq!(freya_take_repaint_request(), 1); // repaint requested

        let name = c("width");
        let value = c("100");
        freya_set_attribute(parent, name.as_ptr(), value.as_ptr());
        assert_eq!(freya_take_repaint_request(), 1);

        let prop = c("background");
        let val = c("red");
        freya_set_style(parent, prop.as_ptr(), val.as_ptr());
        assert_eq!(freya_take_repaint_request(), 1);

        let text = c("hello");
        freya_set_text_content(parent, text.as_ptr());
        assert_eq!(freya_take_repaint_request(), 1);

        freya_remove_child(parent, child);
        assert_eq!(freya_take_repaint_request(), 1);

        // No more pending
        assert_eq!(freya_take_repaint_request(), 0);

        freya_destroy_element(parent);
        freya_destroy_element(child);
    }

    #[test]
    #[serial]
    fn test_window_not_found() {
        freya_reset_windows();
        assert_eq!(freya_window_state(999), 0); // not found
        assert_eq!(freya_window_width(999), 0.0);
        assert_eq!(freya_window_height(999), 0.0);
        assert_eq!(freya_show_window(999), 0);
        assert_eq!(freya_close_window(999), 0);
        // These should not crash
        freya_destroy_window(999);
        freya_on_resize(999, { extern "C" fn noop(_: f64, _: f64) {} noop });
        freya_on_focus(999, { extern "C" fn noop(_: u8) {} noop });
        freya_on_close(999, { extern "C" fn noop() -> u8 { 1 } noop });
        freya_notify_resize(999, 100.0, 100.0);
        freya_notify_focus(999, 1);
    }

    #[test]
    #[serial]
    #[cfg(not(feature = "freya-backend"))]
    fn test_launch_callback() {
        freya_reset_tree();

        static BUILDER_CALLED: AtomicU32 = AtomicU32::new(0);

        extern "C" fn test_builder(root: *mut FreyaElement) {
            assert!(!root.is_null());
            BUILDER_CALLED.fetch_add(1, Ordering::SeqCst);

            // Build a small tree inside the callback
            let tag = CString::new("label").unwrap();
            let child = freya_create_element(tag.as_ptr());
            freya_append_child(root, child);
            // Don't destroy child handle here — it's still in the tree
            freya_destroy_element(child);
        }

        BUILDER_CALLED.store(0, Ordering::SeqCst);

        let title = c("Test Window");
        freya_launch(title.as_ptr(), 800.0, 600.0, test_builder);

        assert_eq!(BUILDER_CALLED.load(Ordering::SeqCst), 1);
        // Root + label child = 2 nodes
        assert_eq!(freya_tree_node_count(), 2);
    }

    #[test]
    #[serial]
    fn test_dispatch_event_multiple_listeners() {
        freya_reset_tree();

        static CALL_A: AtomicU32 = AtomicU32::new(0);
        static CALL_B: AtomicU32 = AtomicU32::new(0);

        extern "C" fn handler_a() {
            CALL_A.fetch_add(1, Ordering::SeqCst);
        }
        extern "C" fn handler_b() {
            CALL_B.fetch_add(1, Ordering::SeqCst);
        }

        CALL_A.store(0, Ordering::SeqCst);
        CALL_B.store(0, Ordering::SeqCst);

        let tag = c("button");
        let click = c("click");
        let node = freya_create_element(tag.as_ptr());

        freya_add_event_listener(node, click.as_ptr(), handler_a);
        freya_add_event_listener(node, click.as_ptr(), handler_b);

        freya_dispatch_event(node, click.as_ptr());
        assert_eq!(CALL_A.load(Ordering::SeqCst), 1);
        assert_eq!(CALL_B.load(Ordering::SeqCst), 1);

        freya_dispatch_event(node, click.as_ptr());
        assert_eq!(CALL_A.load(Ordering::SeqCst), 2);
        assert_eq!(CALL_B.load(Ordering::SeqCst), 2);

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_dispatch_event_different_events() {
        freya_reset_tree();

        static CLICK_COUNT: AtomicU32 = AtomicU32::new(0);
        static INPUT_COUNT: AtomicU32 = AtomicU32::new(0);

        extern "C" fn on_click() {
            CLICK_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        extern "C" fn on_input() {
            INPUT_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        CLICK_COUNT.store(0, Ordering::SeqCst);
        INPUT_COUNT.store(0, Ordering::SeqCst);

        let tag = c("div");
        let click = c("click");
        let input = c("input");
        let node = freya_create_element(tag.as_ptr());

        freya_add_event_listener(node, click.as_ptr(), on_click);
        freya_add_event_listener(node, input.as_ptr(), on_input);

        // Dispatch click — only click handler fires
        freya_dispatch_event(node, click.as_ptr());
        assert_eq!(CLICK_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(INPUT_COUNT.load(Ordering::SeqCst), 0);

        // Dispatch input — only input handler fires
        freya_dispatch_event(node, input.as_ptr());
        assert_eq!(CLICK_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(INPUT_COUNT.load(Ordering::SeqCst), 1);

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_dispatch_event_nonexistent_event() {
        freya_reset_tree();
        let tag = c("div");
        let node = freya_create_element(tag.as_ptr());
        let event = c("nonexistent");

        // Should not crash — just does nothing
        freya_dispatch_event(node, event.as_ptr());
        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_dispatch_event_null_node() {
        let event = c("click");
        // Should not crash
        freya_dispatch_event(std::ptr::null_mut(), event.as_ptr());
    }

    #[test]
    #[serial]
    fn test_event_dispatch_releases_lock_before_callback() {
        // Verify that the callback can access the tree (proving the lock
        // was released before the callback was invoked).
        freya_reset_tree();

        static SUCCESS: AtomicU32 = AtomicU32::new(0);

        extern "C" fn tree_accessing_handler() {
            // This should not deadlock — the tree lock must be released
            // before callbacks are invoked.
            let count = freya_tree_node_count();
            if count > 0 {
                SUCCESS.store(1, Ordering::SeqCst);
            }
        }

        SUCCESS.store(0, Ordering::SeqCst);

        let tag = c("div");
        let click = c("click");
        let node = freya_create_element(tag.as_ptr());
        freya_add_event_listener(node, click.as_ptr(), tree_accessing_handler);

        freya_dispatch_event(node, click.as_ptr());
        assert_eq!(SUCCESS.load(Ordering::SeqCst), 1);

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    #[cfg(not(feature = "freya-backend"))]
    fn test_window_integration_with_launch() {
        // Verify that freya_launch (without freya-backend) creates the root
        // and the builder callback can attach event listeners that dispatch.
        freya_reset_tree();
        freya_reset_windows();

        static CLICK_FIRED: AtomicU32 = AtomicU32::new(0);

        extern "C" fn on_click() {
            CLICK_FIRED.fetch_add(1, Ordering::SeqCst);
        }

        extern "C" fn builder(root: *mut FreyaElement) {
            let tag = CString::new("button").unwrap();
            let click = CString::new("click").unwrap();
            let btn = freya_create_element(tag.as_ptr());
            freya_add_event_listener(btn, click.as_ptr(), on_click);
            freya_append_child(root, btn);
            freya_destroy_element(btn);
        }

        CLICK_FIRED.store(0, Ordering::SeqCst);

        let title = c("Test");
        freya_launch(title.as_ptr(), 800.0, 600.0, builder);

        // The root should now exist with 1 child (the button)
        assert_eq!(freya_tree_node_count(), 2);

        // Find the button (first child of root) and dispatch a click
        let root_id = {
            let root = ROOT_NODE_ID.lock().unwrap();
            *root
        };
        let root_handle = node_id_to_handle(root_id);
        let btn_handle = freya_first_child(root_handle);
        assert!(!btn_handle.is_null());

        let click = c("click");
        freya_dispatch_event(btn_handle, click.as_ptr());
        assert_eq!(CLICK_FIRED.load(Ordering::SeqCst), 1);

        freya_destroy_element(root_handle);
        freya_destroy_element(btn_handle);
    }

    #[test]
    #[serial]
    fn test_child_count() {
        freya_reset_tree();
        let tag = c("rect");
        let parent = freya_create_element(tag.as_ptr());
        assert_eq!(freya_child_count(parent), 0);

        let c1 = freya_create_element(tag.as_ptr());
        let c2 = freya_create_element(tag.as_ptr());
        freya_append_child(parent, c1);
        assert_eq!(freya_child_count(parent), 1);
        freya_append_child(parent, c2);
        assert_eq!(freya_child_count(parent), 2);

        freya_remove_child(parent, c1);
        assert_eq!(freya_child_count(parent), 1);

        freya_destroy_element(parent);
        freya_destroy_element(c1);
        freya_destroy_element(c2);
    }

    #[test]
    #[serial]
    fn test_get_text_content() {
        freya_reset_tree();
        let text = c("hello world");
        let node = freya_create_text_node(text.as_ptr());

        // Query size first
        let needed = freya_get_text_content(node, std::ptr::null_mut(), 0);
        assert_eq!(needed, 11); // "hello world".len()

        // Read into buffer
        let mut buf = vec![0u8; 64];
        let written = freya_get_text_content(node, buf.as_mut_ptr() as *mut c_char, 64);
        assert_eq!(written, 11);
        let result = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
        assert_eq!(result.to_str().unwrap(), "hello world");

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_get_text_content_recursive() {
        freya_reset_tree();
        let tag = c("rect");
        let parent = freya_create_element(tag.as_ptr());
        let t1 = c("hello ");
        let t2 = c("world");
        let child1 = freya_create_text_node(t1.as_ptr());
        let child2 = freya_create_text_node(t2.as_ptr());
        freya_append_child(parent, child1);
        freya_append_child(parent, child2);

        let mut buf = vec![0u8; 64];
        let written = freya_get_text_content(parent, buf.as_mut_ptr() as *mut c_char, 64);
        assert_eq!(written, 11);
        let result = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
        assert_eq!(result.to_str().unwrap(), "hello world");

        freya_destroy_element(parent);
        freya_destroy_element(child1);
        freya_destroy_element(child2);
    }

    #[test]
    #[serial]
    fn test_get_attribute() {
        freya_reset_tree();
        let tag = c("rect");
        let node = freya_create_element(tag.as_ptr());
        let name = c("class");
        let value = c("container");
        freya_set_attribute(node, name.as_ptr(), value.as_ptr());

        let mut buf = vec![0u8; 64];
        let written = freya_get_attribute(
            node,
            name.as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            64,
        );
        assert_eq!(written, 9); // "container".len()
        let result = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
        assert_eq!(result.to_str().unwrap(), "container");

        // Non-existent attribute
        let missing = c("nonexistent");
        let written = freya_get_attribute(
            node,
            missing.as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            64,
        );
        assert_eq!(written, 0);

        freya_destroy_element(node);
    }

    #[test]
    #[serial]
    fn test_nth_child() {
        freya_reset_tree();
        let tag = c("rect");
        let parent = freya_create_element(tag.as_ptr());
        let c1 = freya_create_element(tag.as_ptr());
        let c2 = freya_create_element(tag.as_ptr());
        freya_append_child(parent, c1);
        freya_append_child(parent, c2);

        let nth0 = freya_nth_child(parent, 0);
        assert!(!nth0.is_null());
        assert_eq!(unsafe { (*nth0).node_id }, unsafe { (*c1).node_id });

        let nth1 = freya_nth_child(parent, 1);
        assert!(!nth1.is_null());
        assert_eq!(unsafe { (*nth1).node_id }, unsafe { (*c2).node_id });

        let nth2 = freya_nth_child(parent, 2);
        assert!(nth2.is_null());

        freya_destroy_element(nth0);
        freya_destroy_element(nth1);
        freya_destroy_element(parent);
        freya_destroy_element(c1);
        freya_destroy_element(c2);
    }
}
