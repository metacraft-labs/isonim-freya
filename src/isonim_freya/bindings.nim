## Raw C bindings to the freya-nim-shim Rust cdylib.
##
## These map 1:1 to the extern "C" functions exported by
## rust/freya-nim-shim/src/lib.rs.

type
  FreyaElementObj {.importc: "FreyaElement", header: "".} = object
  FreyaElement* = ptr FreyaElementObj
    ## Opaque handle to a Freya element managed by the Rust shim.

const shimLib = "libfreya_nim_shim.so"  # TODO: platform-specific (.dylib on macOS)

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

# --- Debugging / testing ---

proc freya_reset_tree*()
  {.importc: "freya_reset_tree".}

proc freya_tree_node_count*(): uint64
  {.importc: "freya_tree_node_count".}

{.pop.}
