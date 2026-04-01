
## Globally unique node identifier.
type NodeId* {.incompleteStruct.} = object

## Opaque handle to a Freya element. Wraps a NodeId.
# Allocated on the heap via Box so Nim holds a stable pointer.
type FreyaElement* = object
  xnode_id*: uint64

## C function pointer type for event callbacks from Nim.
type EventCallback* = proc(): void

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
type RootBuilderCallback* = proc(root: ptr FreyaElement): void



## Create a new element with the given tag name.
#
# Returns a heap-allocated handle that the caller (Nim) must hold.
# The element is added to the global shadow tree but not attached to any parent.
proc freya_create_element*(tag: pointer): ptr FreyaElement {.importc: "freya_create_element".}

## Create a text node with the given content.
proc freya_create_text_node*(text: pointer): ptr FreyaElement {.importc: "freya_create_text_node".}

## Append `child` as the last child of `parent`.
proc freya_append_child*(parent: ptr FreyaElement,
                         child: ptr FreyaElement): void {.importc: "freya_append_child".}

## Insert `child` before `reference` within `parent`.
# If `reference` is null, appends child instead.
proc freya_insert_before*(parent: ptr FreyaElement,
                          child: ptr FreyaElement,
                          reference: ptr FreyaElement): void {.importc: "freya_insert_before".}

## Remove `child` from `parent`.
proc freya_remove_child*(parent: ptr FreyaElement,
                         child: ptr FreyaElement): void {.importc: "freya_remove_child".}

## Set attribute `name` to `value` on `node`.
proc freya_set_attribute*(node: ptr FreyaElement,
                          name: pointer,
                          value: pointer): void {.importc: "freya_set_attribute".}

## Remove attribute `name` from `node`.
proc freya_remove_attribute*(node: ptr FreyaElement,
                             name: pointer): void {.importc: "freya_remove_attribute".}

## Set the text content of `node`.
proc freya_set_text_content*(node: ptr FreyaElement,
                             text: pointer): void {.importc: "freya_set_text_content".}

## Set a style property on `node`.
proc freya_set_style*(node: ptr FreyaElement,
                      prop: pointer,
                      value: pointer): void {.importc: "freya_set_style".}

## Register a callback for `event` on `node`.
# The `handler` is a C function pointer that Nim will pass in.
proc freya_add_event_listener*(node: ptr FreyaElement,
                               event: pointer,
                               handler: EventCallback): void {.importc: "freya_add_event_listener".}

## Return the first child of `node`, or null if it has no children.
proc freya_first_child*(node: ptr FreyaElement): ptr FreyaElement {.importc: "freya_first_child".}

## Return the next sibling of `node`, or null.
proc freya_next_sibling*(node: ptr FreyaElement): ptr FreyaElement {.importc: "freya_next_sibling".}

## Return the parent of `node`, or null.
proc freya_parent_node*(node: ptr FreyaElement): ptr FreyaElement {.importc: "freya_parent_node".}

proc freya_launch*(title: pointer,
                   width: float64,
                   height: float64,
                   root_builder: RootBuilderCallback): void {.importc: "freya_launch".}

## Trigger all event listeners for the given event on the given node.
# This is called by the Freya event loop (M2+) when an event occurs,
# or can be called directly for testing.
proc freya_dispatch_event*(node: ptr FreyaElement,
                           event: pointer): void {.importc: "freya_dispatch_event".}

## Free a FreyaElement handle.
# This deallocates the handle pointer but does NOT remove the node from the tree.
# Call freya_remove_child first to detach the node, then freya_destroy_element
# to free the handle memory.
proc freya_destroy_element*(handle: ptr FreyaElement): void {.importc: "freya_destroy_element".}

## Remove a node and all its descendants from the shadow tree entirely.
# This is for cleanup — it removes the node from the tree store (not just
# from its parent's children list). The handle is also freed.
proc freya_destroy_tree*(handle: ptr FreyaElement): void {.importc: "freya_destroy_tree".}

## Reset the global tree (useful for testing).
proc freya_reset_tree*(): void {.importc: "freya_reset_tree".}

## Get the number of nodes in the global tree (useful for debugging/testing).
proc freya_tree_node_count*(): uint64 {.importc: "freya_tree_node_count".}
