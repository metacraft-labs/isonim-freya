## Basic smoke test for isonim-freya.
##
## Verifies that the Nim side compiles and the type signatures are correct.
## Does NOT link against the Rust shim (uses compile-only checks).

# The renderer module re-exports everything we need.
import isonim_freya/renderer
import isonim_freya/bindings

# Static type-level conformance check: verify that all 13 RendererBackend procs
# exist with the correct signatures. We cannot use checkRendererBackend here
# because its {.compileTime.} body tries to call the dynlib-imported procs,
# which the Nim VM cannot execute. Instead we verify each proc compiles
# individually.
static:
  # Check that all required procs are callable with the right types.
  # These compiles() checks verify type signatures without executing code.
  var r: FreyaRenderer
  assert compiles(r.createElement(""))
  assert compiles(r.createTextNode(""))
  var e: FreyaElement
  assert compiles(r.appendChild(e, e))
  assert compiles(r.insertBefore(e, e, e))
  assert compiles(r.removeChild(e, e))
  assert compiles(r.setAttribute(e, "", ""))
  assert compiles(r.removeAttribute(e, ""))
  assert compiles(r.setTextContent(e, ""))
  assert compiles(r.setStyle(e, "", ""))
  assert compiles(r.addEventListener(e, "", proc() = discard))
  assert compiles(r.firstChild(e))
  assert compiles(r.nextSibling(e))
  assert compiles(r.parentNode(e))

echo "isonim-freya: compile check passed"
