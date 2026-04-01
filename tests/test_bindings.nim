## Binding completeness test for isonim-freya.
##
## Verifies that all 19 Rust FFI exports have corresponding Nim bindings
## with correct type signatures. This is a compile-time check — it does not
## link against the Rust shim.

import isonim_freya/bindings

# Verify all 19 exported functions exist and have correct signatures.
# compiles() checks that the expression type-checks without executing it.
static:
  # --- 13 RendererBackend functions ---
  # 1. createElement
  assert compiles(freya_create_element("div".cstring))
  # 2. createTextNode
  assert compiles(freya_create_text_node("hello".cstring))
  # 3. appendChild
  var e: FreyaElement
  assert compiles(freya_append_child(e, e))
  # 4. insertBefore
  assert compiles(freya_insert_before(e, e, e))
  # 5. removeChild
  assert compiles(freya_remove_child(e, e))
  # 6. setAttribute
  assert compiles(freya_set_attribute(e, "k".cstring, "v".cstring))
  # 7. removeAttribute
  assert compiles(freya_remove_attribute(e, "k".cstring))
  # 8. setTextContent
  assert compiles(freya_set_text_content(e, "text".cstring))
  # 9. setStyle
  assert compiles(freya_set_style(e, "color".cstring, "red".cstring))
  # 10. addEventListener
  proc dummyCb() {.cdecl.} = discard
  assert compiles(freya_add_event_listener(e, "click".cstring, dummyCb))
  # 11. firstChild
  assert compiles(freya_first_child(e))
  # 12. nextSibling
  assert compiles(freya_next_sibling(e))
  # 13. parentNode
  assert compiles(freya_parent_node(e))

  # --- Window / event loop (3 functions) ---
  # 14. launch
  proc dummyBuilder(root: FreyaElement) {.cdecl.} = discard
  assert compiles(freya_launch("title".cstring, 800.0, 600.0, dummyBuilder))
  # 15. dispatchEvent
  assert compiles(freya_dispatch_event(e, "click".cstring))

  # --- Memory management (2 functions) ---
  # 16. destroyElement
  assert compiles(freya_destroy_element(e))
  # 17. destroyTree
  assert compiles(freya_destroy_tree(e))

  # --- Debug / testing (2 functions) ---
  # 18. resetTree
  assert compiles(freya_reset_tree())
  # 19. treeNodeCount
  assert compiles(freya_tree_node_count())

# Verify return types
static:
  var e: FreyaElement
  # Functions that return FreyaElement
  assert freya_create_element("".cstring) is FreyaElement
  assert freya_create_text_node("".cstring) is FreyaElement
  assert freya_first_child(e) is FreyaElement
  assert freya_next_sibling(e) is FreyaElement
  assert freya_parent_node(e) is FreyaElement
  # Function that returns uint64
  assert freya_tree_node_count() is uint64

# Verify callback types
static:
  assert EventCallback is proc() {.cdecl.}
  assert RootBuilderCallback is proc(root: FreyaElement) {.cdecl.}

echo "test_bindings: all 19 bindings verified (compile-time)"
