## Raw C bindings to the freya-nim-shim Rust cdylib.
##
## These map 1:1 to the extern "C" functions exported by
## rust/freya-nim-shim/src/lib.rs.

type
  FreyaElement* = pointer
    ## Opaque handle to a Freya element managed by the Rust shim.
    ## The actual layout is a Rust struct; Nim only holds a pointer to it.

when defined(macosx):
  const shimLib = "libfreya_nim_shim.dylib"
elif defined(windows):
  const shimLib = "freya_nim_shim.dll"
else:
  const shimLib = "libfreya_nim_shim.so"

{.push cdecl, dynlib: shimLib.}

proc freya_create_element*(tag: cstring): FreyaElement
  {.importc: "freya_create_element".}

proc freya_create_text_node*(text: cstring): FreyaElement
  {.importc: "freya_create_text_node".}

proc freya_append_child*(parent, child: FreyaElement)
  {.importc: "freya_append_child".}

proc freya_insert_before*(parent, child, reference: FreyaElement)
  {.importc: "freya_insert_before".}

proc freya_remove_child*(parent, child: FreyaElement)
  {.importc: "freya_remove_child".}

proc freya_set_attribute*(node: FreyaElement; name, value: cstring)
  {.importc: "freya_set_attribute".}

proc freya_remove_attribute*(node: FreyaElement; name: cstring)
  {.importc: "freya_remove_attribute".}

proc freya_set_text_content*(node: FreyaElement; text: cstring)
  {.importc: "freya_set_text_content".}

proc freya_set_style*(node: FreyaElement; prop, value: cstring)
  {.importc: "freya_set_style".}

type EventCallback* = proc() {.cdecl.}

proc freya_add_event_listener*(node: FreyaElement; event: cstring; handler: EventCallback)
  {.importc: "freya_add_event_listener".}

proc freya_add_event_listener_id*(node: FreyaElement; event: cstring; callbackId: int32)
  {.importc: "freya_add_event_listener_id".}

type EventDispatcherCallback* = proc(callbackId: int32) {.cdecl.}

proc freya_set_event_dispatcher*(dispatcher: EventDispatcherCallback)
  {.importc: "freya_set_event_dispatcher".}

proc freya_first_child*(node: FreyaElement): FreyaElement
  {.importc: "freya_first_child".}

proc freya_next_sibling*(node: FreyaElement): FreyaElement
  {.importc: "freya_next_sibling".}

proc freya_parent_node*(node: FreyaElement): FreyaElement
  {.importc: "freya_parent_node".}

# --- Window / event loop management ---

type RootBuilderCallback* = proc(root: FreyaElement) {.cdecl.}

proc freya_launch*(title: cstring; width, height: cdouble;
                   root_builder: RootBuilderCallback)
  {.importc: "freya_launch".}

proc freya_dispatch_event*(node: FreyaElement; event: cstring)
  {.importc: "freya_dispatch_event".}

# --- Memory management ---

proc freya_destroy_element*(handle: FreyaElement)
  {.importc: "freya_destroy_element".}

proc freya_destroy_tree*(handle: FreyaElement)
  {.importc: "freya_destroy_tree".}

# --- Window management (M4) ---

type
  ResizeCallback* = proc(width, height: cdouble) {.cdecl.}
  FocusCallback* = proc(focused: uint8) {.cdecl.}
  CloseCallback* = proc(): uint8 {.cdecl.}

proc freya_create_window*(title: cstring; width, height: cdouble): uint32
  {.importc: "freya_create_window".}

proc freya_show_window*(window_id: uint32): uint8
  {.importc: "freya_show_window".}

proc freya_close_window*(window_id: uint32): uint8
  {.importc: "freya_close_window".}

proc freya_destroy_window*(window_id: uint32)
  {.importc: "freya_destroy_window".}

proc freya_window_state*(window_id: uint32): uint8
  {.importc: "freya_window_state".}

proc freya_window_width*(window_id: uint32): cdouble
  {.importc: "freya_window_width".}

proc freya_window_height*(window_id: uint32): cdouble
  {.importc: "freya_window_height".}

proc freya_request_repaint*()
  {.importc: "freya_request_repaint".}

proc freya_take_repaint_request*(): uint8
  {.importc: "freya_take_repaint_request".}

proc freya_on_resize*(window_id: uint32; callback: ResizeCallback)
  {.importc: "freya_on_resize".}

proc freya_on_focus*(window_id: uint32; callback: FocusCallback)
  {.importc: "freya_on_focus".}

proc freya_on_close*(window_id: uint32; callback: CloseCallback)
  {.importc: "freya_on_close".}

proc freya_notify_resize*(window_id: uint32; width, height: cdouble)
  {.importc: "freya_notify_resize".}

proc freya_notify_focus*(window_id: uint32; focused: uint8)
  {.importc: "freya_notify_focus".}

proc freya_reset_windows*()
  {.importc: "freya_reset_windows".}

# --- Debugging / testing ---

proc freya_reset_tree*()
  {.importc: "freya_reset_tree".}

proc freya_tree_node_count*(): uint64
  {.importc: "freya_tree_node_count".}

# --- Tree inspection (M5 — cross-renderer testing) ---

proc freya_child_count*(node: FreyaElement): uint64
  {.importc: "freya_child_count".}

proc freya_get_text_content*(node: FreyaElement; buf: pointer; bufLen: uint64): uint64
  {.importc: "freya_get_text_content".}

proc freya_get_attribute*(node: FreyaElement; name: cstring; buf: pointer; bufLen: uint64): uint64
  {.importc: "freya_get_attribute".}

proc freya_nth_child*(node: FreyaElement; index: uint64): FreyaElement
  {.importc: "freya_nth_child".}

# --- Render plan inspection (G3-F — integration testing) ---

proc freya_render_plan_json*(root: FreyaElement): pointer
  {.importc: "freya_render_plan_json".}

proc freya_free_string*(p: pointer)
  {.importc: "freya_free_string".}

proc freya_render_plan_element_count*(root: FreyaElement): uint32
  {.importc: "freya_render_plan_element_count".}

proc freya_verify_render_plan*(root: FreyaElement): uint8
  {.importc: "freya_verify_render_plan".}

{.pop.}
