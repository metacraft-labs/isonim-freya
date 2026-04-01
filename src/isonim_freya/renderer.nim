## FreyaRenderer — implements IsoNim's RendererBackend backed by the Freya
## native GUI framework via the Rust FFI shim.
##
## This module wraps the raw C bindings in bindings.nim into the
## high-level proc signatures that checkRendererBackend expects.

import std/tables
import isonim_freya/bindings

type
  FreyaRenderer* = object
    ## Renderer backend that delegates to Freya via FFI.

# Callback registry: maps an integer id to a Nim closure.
# The cdecl trampoline dispatches through this table.
var nextCallbackId: int
var callbackRegistry: Table[int, proc()]

proc createElement*(r: FreyaRenderer; tag: string): FreyaElement =
  freya_create_element(tag.cstring)

proc createTextNode*(r: FreyaRenderer; text: string): FreyaElement =
  freya_create_text_node(text.cstring)

proc appendChild*(r: FreyaRenderer; parent, child: FreyaElement) =
  freya_append_child(parent, child)

proc insertBefore*(r: FreyaRenderer; parent, child, reference: FreyaElement) =
  freya_insert_before(parent, child, reference)

proc removeChild*(r: FreyaRenderer; parent, child: FreyaElement) =
  freya_remove_child(parent, child)

proc setAttribute*(r: FreyaRenderer; node: FreyaElement; name, value: string) =
  freya_set_attribute(node, name.cstring, value.cstring)

proc removeAttribute*(r: FreyaRenderer; node: FreyaElement; name: string) =
  freya_remove_attribute(node, name.cstring)

proc setTextContent*(r: FreyaRenderer; node: FreyaElement; text: string) =
  freya_set_text_content(node, text.cstring)

proc setStyle*(r: FreyaRenderer; node: FreyaElement; prop, value: string) =
  freya_set_style(node, prop.cstring, value.cstring)

proc addEventListener*(r: FreyaRenderer; node: FreyaElement; event: string; handler: proc()) =
  ## Registers a Nim closure as an event handler. The closure is stored in a
  ## Nim-side registry keyed by an integer id. A cdecl trampoline is passed
  ## to the Rust shim which will call back with the id.
  ## TODO: implement the full trampoline mechanism in the Rust shim.
  let id = nextCallbackId
  inc nextCallbackId
  callbackRegistry[id] = handler
  # For now, pass a no-op cdecl callback; the real bridge will use the id.
  proc trampoline() {.cdecl.} = discard
  freya_add_event_listener(node, event.cstring, trampoline)

proc firstChild*(r: FreyaRenderer; node: FreyaElement): FreyaElement =
  freya_first_child(node)

proc nextSibling*(r: FreyaRenderer; node: FreyaElement): FreyaElement =
  freya_next_sibling(node)

proc parentNode*(r: FreyaRenderer; node: FreyaElement): FreyaElement =
  freya_parent_node(node)
