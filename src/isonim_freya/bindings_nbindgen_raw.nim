
## Opaque handle to a Freya element. Wraps a NodeId.
# Allocated on the heap via Box so Nim holds a stable pointer.
type FreyaElement* = object
  node_id: uint64

## C function pointer type for event callbacks from Nim.
type EventCallback* = proc () {.cdecl.}

## Launch a Freya window.
#
# This creates a root element in the shadow tree and starts the Freya event loop.
# The `title` parameter sets the window title.
# The `width` and `height` parameters set the initial window size.
# The `root_builder` callback is called with the root element handle so the
# Nim side can build the initial tree before the event loop starts.
#
# **Note:** In M1 this is a placeholder that creates the root element and calls
# the builder callback but does NOT start an actual Freya window (that requires
# the full Freya dependency to be available at link time). The actual Freya
# integration will be completed in M2.
#
# With the `freya-backend` feature the call blocks in Freya's event loop
# until the window is closed or `freya_quit` is honoured. A request made
# before the call, or from inside `root_builder`, ends the loop once the
# window has presented its first frame; `freya_presented_frame_count`
# then reports how many frames it presented. A pending quit request is
# dropped when `freya_launch` returns, in either build.
#
# winit allows ONE event loop per process (`EventLoopError::RecreationAttempt`
# on a second `EventLoop` build), so with `freya-backend` `freya_launch`
# can run once per process; a second call aborts the process.
type RootBuilderCallback* = proc (root: ptr FreyaElement) {.cdecl.}

type CloseCallback* = proc (): uint8 {.cdecl.}

type FocusCallback* = proc (focused: uint8) {.cdecl.}

## C function pointer types for lifecycle callbacks.
type ResizeCallback* = proc (width: float64, height: float64) {.cdecl.}

## Global event dispatcher function pointer, registered by Nim via
# `freya_set_event_dispatcher`. When set, event listeners that have a
# `callback_id > 0` are dispatched through this function instead of
# calling a C function pointer directly.
type EventDispatcher* = proc (callback_id: int32) {.cdecl.}

## Register a callback for `event` on `node`.
# The `handler` is a C function pointer that Nim will pass in.
# Legacy API — uses direct function pointer dispatch (callback_id = 0).
proc freya_add_event_listener*(node: ptr FreyaElement,
                               event: ptr char,
                               handler: EventCallback) {.importc: "freya_add_event_listener".}

## Register a callback ID for `event` on `node`.
# The `callback_id` is dispatched via the global event dispatcher registered
# by `freya_set_event_dispatcher`.
proc freya_add_event_listener_id*(node: ptr FreyaElement,
                                  event: ptr char,
                                  callback_id: int32) {.importc: "freya_add_event_listener_id".}

## Append `child` as the last child of `parent`.
proc freya_append_child*(parent: ptr FreyaElement,
                         child: ptr FreyaElement) {.importc: "freya_append_child".}

## Get the number of children of a node.
# Returns 0 if the node is null or not found.
proc freya_child_count*(node: ptr FreyaElement): uint64 {.importc: "freya_child_count".}

## Request that a window be closed. If an on_close callback is registered
# and returns 0, the close is denied.
# Returns 1 if the window was closed, 0 if denied or not found.
proc freya_close_window*(window_id: uint32): uint8 {.importc: "freya_close_window".}

## Create a new element with the given tag name.
#
# Returns a heap-allocated handle that the caller (Nim) must hold.
# The element is added to the global shadow tree but not attached to any parent.
proc freya_create_element*(tag: ptr char): (ptr FreyaElement) {.importc: "freya_create_element".}

## Create a text node with the given content.
proc freya_create_text_node*(text: ptr char): (ptr FreyaElement) {.importc: "freya_create_text_node".}

## Create a new window with the given title and initial size.
# Returns a window ID (> 0) on success, 0 on failure.
proc freya_create_window*(title: ptr char,
                          width: float64,
                          height: float64): uint32 {.importc: "freya_create_window".}

## Free a FreyaElement handle.
# This deallocates the handle pointer but does NOT remove the node from the tree.
# Call freya_remove_child first to detach the node, then freya_destroy_element
# to free the handle memory.
proc freya_destroy_element*(handle: ptr FreyaElement) {.importc: "freya_destroy_element".}

## Remove a node and all its descendants from the shadow tree entirely.
# This is for cleanup — it removes the node from the tree store (not just
# from its parent's children list). The handle is also freed.
proc freya_destroy_tree*(handle: ptr FreyaElement) {.importc: "freya_destroy_tree".}

## Destroy a window and remove it from the registry.
proc freya_destroy_window*(window_id: uint32) {.importc: "freya_destroy_window".}

## Trigger all event listeners for the given event on the given node.
# This is called by the Freya event loop (M2+) when an event occurs,
# or can be called directly for testing.
proc freya_dispatch_event*(node: ptr FreyaElement,
                           event: ptr char) {.importc: "freya_dispatch_event".}

## Return the first child of `node`, or null if it has no children.
proc freya_first_child*(node: ptr FreyaElement): (ptr FreyaElement) {.importc: "freya_first_child".}

## Free a buffer previously returned by [`freya_render_to_pixels`].
#
# # Safety
#
# `ptr` MUST be a pointer returned by `freya_render_to_pixels`,
# and `len` MUST be the byte count associated with that
# allocation. Calling with `ptr == null` or `len == 0` is a
# no-op. Calling twice on the same buffer is undefined behaviour
# (double-free).
proc freya_free_pixels*(ptrx: ptr uint8, len: uint) {.importc: "freya_free_pixels".}

## # Safety
#
# This arm never allocates, so the only correct argument is a null
# pointer / zero length. The signature is kept identical to the real
# arm so the ABI does not vary with features.
proc freya_free_pixels*(x_ptr: ptr uint8, x_len: uint) {.importc: "freya_free_pixels".}

## Free a string returned by `freya_render_plan_json`.
proc freya_free_string*(ptrx: ptr char) {.importc: "freya_free_string".}

## Get an attribute value from a node.
# Returns the number of bytes in the attribute value (excluding null),
# or 0 if the attribute is not found. Writes into `buf` if provided.
proc freya_get_attribute*(node: ptr FreyaElement,
                          name: ptr char,
                          buf: ptr char,
                          buf_len: uint64): uint64 {.importc: "freya_get_attribute".}

## Get the tag name of a node. Returns 0 for text nodes.
# Writes into `buf` if provided, returns the number of bytes needed.
#
# Mirrors `gpui_get_tag` in `isonim-gpui` — used by the RS-M4
# Freya streaming adapter (in `isonim-render-serve`) to derive a
# per-element fill colour when rasterizing the headless tree to
# RGBA pixels. The behaviour exactly matches the GPUI shim's
# `gpui_get_tag` so the two adapters can share assertions in
# cross-renderer parity tests.
proc freya_get_tag*(node: ptr FreyaElement,
                    buf: ptr char,
                    buf_len: uint64): uint64 {.importc: "freya_get_tag".}

## Get the text content of a node and all its descendants (recursive).
# For text nodes, returns the text. For element nodes, concatenates
# all descendant text. The result is written into the provided buffer.
# Returns the number of bytes written (excluding null terminator),
# or the required buffer size if the buffer is too small.
# If `buf` is null, returns the required size.
proc freya_get_text_content*(node: ptr FreyaElement,
                             buf: ptr char,
                             buf_len: uint64): uint64 {.importc: "freya_get_text_content".}

## Insert `child` before `reference` within `parent`.
# If `reference` is null, appends child instead.
proc freya_insert_before*(parent: ptr FreyaElement,
                          child: ptr FreyaElement,
                          reference: ptr FreyaElement) {.importc: "freya_insert_before".}

proc freya_launch*(title: ptr char,
                   width: float64,
                   height: float64,
                   root_builder: RootBuilderCallback) {.importc: "freya_launch".}

## Return the next sibling of `node`, or null.
proc freya_next_sibling*(node: ptr FreyaElement): (ptr FreyaElement) {.importc: "freya_next_sibling".}

## Stable identity of the shadow-tree node a handle refers to.
#
# **This is not redundant with the handle pointer, and assuming it was
# is a real bug this export exists to remove.** `freya_nth_child`,
# `freya_first_child`, `freya_parent_node` and friends each mint a FRESH
# `Box` (see `node_id_to_handle`), so two handles to the same node
# compare unequal as pointers. Anything that needs "is this the same
# node?" — the IsoNim reconciler's positional fallback, and every test
# that asserts a subtree survived a hot reload rather than being
# rebuilt — has to compare this instead.
#
# Returns 0 for a null handle and for the null node, which are the same
# answer on purpose: neither is a node.
proc freya_node_id*(handle: ptr FreyaElement): uint64 {.importc: "freya_node_id".}

## Simulate a focus event on a window (for testing / event bridging).
# `focused`: 1 = gained focus, 0 = lost focus.
proc freya_notify_focus*(window_id: uint32, focused: uint8) {.importc: "freya_notify_focus".}

## Simulate a resize event on a window (for testing / event bridging).
proc freya_notify_resize*(window_id: uint32,
                          width: float64,
                          height: float64) {.importc: "freya_notify_resize".}

## Get the Nth child of a node (0-indexed).
# Returns null if out of bounds or node not found.
proc freya_nth_child*(node: ptr FreyaElement,
                      index: uint64): (ptr FreyaElement) {.importc: "freya_nth_child".}

## Register a callback for window close requests.
# The callback should return 1 to allow close, 0 to prevent it.
proc freya_on_close*(window_id: uint32, callback: CloseCallback) {.importc: "freya_on_close".}

## Register a callback for window focus events.
# The callback receives (focused: u8) where 1 = focused, 0 = unfocused.
proc freya_on_focus*(window_id: uint32, callback: FocusCallback) {.importc: "freya_on_focus".}

## Register a callback for window resize events.
# The callback receives (width: f64, height: f64).
proc freya_on_resize*(window_id: uint32, callback: ResizeCallback) {.importc: "freya_on_resize".}

## Return the parent of `node`, or null.
proc freya_parent_node*(node: ptr FreyaElement): (ptr FreyaElement) {.importc: "freya_parent_node".}

## Number of frames the Freya window presented in the current or most
# recent `freya_launch` (reset when a launch starts and by
# `freya_reset_windows`). Always 0 without `freya-backend`, where no
# window is shown; a caller uses it to tell a launch that really drew
# from one that returned without drawing.
proc freya_presented_frame_count*(): uint64 {.importc: "freya_presented_frame_count".}

## Ask the Freya event loop to end. Callable from any thread, and before
# `freya_launch` (or from its `root_builder`): the request is latched, and
# with `freya-backend` the loop honours it once the window has presented
# its first frame. `freya_launch` drops a request it did not honour when
# it returns, and `freya_reset_windows` drops a pending one.
#
# This is how a caller that is not a user at the keyboard -- a test
# harness, a scripted run -- ends the otherwise unbounded loop. Without
# `freya-backend` `freya_launch` does not block, and the request is
# simply consumed when it returns.
proc freya_quit*() {.importc: "freya_quit".}

## Remove attribute `name` from `node`.
proc freya_remove_attribute*(node: ptr FreyaElement,
                             name: ptr char) {.importc: "freya_remove_attribute".}

## Remove `child` from `parent`.
proc freya_remove_child*(parent: ptr FreyaElement,
                         child: ptr FreyaElement) {.importc: "freya_remove_child".}

## Return the total element count in the render plan rooted at `root`.
# Returns 0 if the handle is null or the node doesn't exist.
proc freya_render_plan_element_count*(root: ptr FreyaElement): uint32 {.importc: "freya_render_plan_element_count".}

## Build a render plan from the shadow tree rooted at `root` and return it as
# a JSON string. The caller must free the returned string with `freya_free_string`.
#
# Returns null if the handle is null or the node doesn't exist.
#
# JSON format:
# ```json
# {
#   "kind": "Rect",
#   "text": null,
#   "has_click_handler": false,
#   "has_input_handler": false,
#   "event_names": [],
#   "styles": { "width": "100%", ... },
#   "children": [ ... ]
# }
# ```
proc freya_render_plan_json*(root: ptr FreyaElement): (ptr char) {.importc: "freya_render_plan_json".}

## Render the current shadow tree to RGBA8888 pixels at the
# specified size.
#
# Returns 0 on success and writes the pixel-buffer pointer to
# `*out_ptr` + byte count to `*out_len`. The buffer is allocated
# by this function and MUST be released via
# [`freya_free_pixels`]; the caller owns the buffer for the
# duration between the two calls.
#
# Returns a non-zero error code on failure; on error the function
# writes a null pointer + zero length to the caller's out
# parameters so naive callers that always free the buffer remain
# safe.
#
# # Color space + byte order
#
# The output is RGBA8888 non-premultiplied sRGB, row-major, top
# row first — the canonical F-packet pixel format. We request this
# format directly from Skia via `read_pixels(..., RGBA_8888,
# Unpremul, ...)` so no manual BGRA<->RGBA conversion is needed
# regardless of the host Skia surface's native byte order.
#
# # Scale semantics
#
# `width` and `height` are the output pixel dimensions. `scale`
# is the logical-to-physical ratio: the layout pass runs at
# `(width / scale, height / scale)` logical pixels. Freya
# testing's internal `SCALE_FACTOR` is hard-coded to 1.0 in the
# 0.3.4 release, so we approximate by dividing the configured
# canvas size — adequate for the 1:1 case the Freya adapter uses
# today.
#
# # Safety
#
# `out_ptr` and `out_len` MUST be non-null and point to writable
# storage. The caller MUST NOT read or free the output buffer if
# the function returns a non-zero error code.
proc freya_render_to_pixels*(width: uint32,
                             height: uint32,
                             scale: float32,
                             out_ptr: ptr (ptr uint8),
                             out_len: ptr uint): int32 {.importc: "freya_render_to_pixels".}

## # Safety
#
# `out_ptr` and `out_len` MUST be non-null and point to writable
# storage, as in the real arm.
proc freya_render_to_pixels*(x_width: uint32,
                             x_height: uint32,
                             x_scale: float32,
                             out_ptr: ptr (ptr uint8),
                             out_len: ptr uint): int32 {.importc: "freya_render_to_pixels".}

## Request a repaint of the window. This signals that the shadow tree has
# changed and the window should re-render on the next frame.
proc freya_request_repaint*() {.importc: "freya_request_repaint".}

## Reset the global tree (useful for testing).
proc freya_reset_tree*() {.importc: "freya_reset_tree".}

## Reset all windows (for testing).
proc freya_reset_windows*() {.importc: "freya_reset_windows".}

## Set attribute `name` to `value` on `node`.
proc freya_set_attribute*(node: ptr FreyaElement,
                          name: ptr char,
                          value: ptr char) {.importc: "freya_set_attribute".}

## Register the global event dispatcher function.
# When set, event listeners with `callback_id > 0` are dispatched through
# this function instead of calling their C function pointer directly.
proc freya_set_event_dispatcher*(dispatcher: EventDispatcher) {.importc: "freya_set_event_dispatcher".}

## RS-M14 Phase 1: set the global `ROOT_NODE_ID` to the given element
# handle. Callers that build the shadow tree *without* going through
# `freya_launch` (e.g. the Freya streaming adapter in
# `isonim_render_serve`) must call this before invoking
# `freya_render_to_pixels` so the headless `shadow_tree_app` finds the
# correct tree root to render. Passing a null handle resets the root
# to `NodeId::NULL`, which causes the renderer to fall back to its
# "No shadow tree root found" placeholder element.
proc freya_set_root_element*(handle: ptr FreyaElement) {.importc: "freya_set_root_element".}

## Set a style property on `node`.
proc freya_set_style*(node: ptr FreyaElement,
                      prop: ptr char,
                      value: ptr char) {.importc: "freya_set_style".}

## Set the text content of `node`.
proc freya_set_text_content*(node: ptr FreyaElement,
                             text: ptr char) {.importc: "freya_set_text_content".}

## Show a window (transition from Created to Visible state).
#
# Without the `freya-backend` feature this just updates the internal state.
# With the feature enabled, this starts the Freya event loop for the window
# on a background thread.
#
# Returns 1 on success, 0 if the window was not in Created state or not found.
proc freya_show_window*(window_id: uint32): uint8 {.importc: "freya_show_window".}

## Check if a repaint has been requested (and clear the flag).
# Returns 1 if a repaint was pending, 0 otherwise.
proc freya_take_repaint_request*(): uint8 {.importc: "freya_take_repaint_request".}

## Get the number of nodes in the global tree (useful for debugging/testing).
proc freya_tree_node_count*(): uint64 {.importc: "freya_tree_node_count".}

## Verify that a render plan can be built from the shadow tree rooted at `root`.
# Returns 1 if valid, 0 if there are issues (null handle, missing node, etc.).
proc freya_verify_render_plan*(root: ptr FreyaElement): uint8 {.importc: "freya_verify_render_plan".}

## Get the current height of a window. Returns 0.0 if not found.
proc freya_window_height*(window_id: uint32): float64 {.importc: "freya_window_height".}

## Get the current state of a window.
# Returns: 0 = not found, 1 = Created, 2 = Visible, 3 = CloseRequested, 4 = Closed.
proc freya_window_state*(window_id: uint32): uint8 {.importc: "freya_window_state".}

## Get the current width of a window. Returns 0.0 if not found.
proc freya_window_width*(window_id: uint32): float64 {.importc: "freya_window_width".}
