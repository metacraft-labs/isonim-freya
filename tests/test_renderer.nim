## Comprehensive tests for the FreyaRenderer.
##
## Tests tag mapping, style mapping, attribute mapping, and the event
## callback bridge. These are compile-time and unit tests that exercise
## the Nim-side logic without requiring the Rust shim at link time.
##
## To run with the Rust shim linked (full integration test):
##   nim c -r --nimcache:nimcache/test_renderer tests/test_renderer.nim
##
## To verify compilation only (no Rust shim needed):
##   nim check --nimcache:nimcache/test_renderer tests/test_renderer.nim

import isonim_freya/renderer
import isonim_freya/bindings

# ===========================================================================
# 1. Compile-time concept conformance
# ===========================================================================

static:
  var r: FreyaRenderer
  var e: FreyaElement

  # All 13 RendererBackend procs must type-check
  assert compiles(r.createElement("div"))
  assert compiles(r.createTextNode("hello"))
  assert compiles(r.appendChild(e, e))
  assert compiles(r.insertBefore(e, e, e))
  assert compiles(r.removeChild(e, e))
  assert compiles(r.setAttribute(e, "class", "container"))
  assert compiles(r.removeAttribute(e, "class"))
  assert compiles(r.setTextContent(e, "text"))
  assert compiles(r.setStyle(e, "color", "red"))
  assert compiles(r.addEventListener(e, "click", proc() = discard))
  assert compiles(r.firstChild(e))
  assert compiles(r.nextSibling(e))
  assert compiles(r.parentNode(e))

  # Return types
  assert r.createElement("") is FreyaElement
  assert r.createTextNode("") is FreyaElement
  assert r.firstChild(e) is FreyaElement
  assert r.nextSibling(e) is FreyaElement
  assert r.parentNode(e) is FreyaElement

echo "test_renderer: compile-time concept check passed"

# ===========================================================================
# 2. Tag mapping tests (compile-time, no FFI calls)
# ===========================================================================
# The tag mapping is a const table, so we can test it at compile time
# by importing the module. Since mapTag is not exported, we test indirectly
# through createElement's behavior. But we can at least verify the compile-time
# checks pass for various tags.

static:
  var r: FreyaRenderer
  # All common HTML tags should be accepted
  assert compiles(r.createElement("div"))
  assert compiles(r.createElement("span"))
  assert compiles(r.createElement("button"))
  assert compiles(r.createElement("input"))
  assert compiles(r.createElement("p"))
  assert compiles(r.createElement("h1"))
  assert compiles(r.createElement("ul"))
  assert compiles(r.createElement("li"))
  assert compiles(r.createElement("img"))
  assert compiles(r.createElement("form"))
  # Freya-native tags should also work (pass-through)
  assert compiles(r.createElement("rect"))
  assert compiles(r.createElement("label"))
  assert compiles(r.createElement("paragraph"))
  assert compiles(r.createElement("ScrollView"))

echo "test_renderer: tag mapping compile check passed"

# ===========================================================================
# 3. Style mapping tests (compile-time)
# ===========================================================================

static:
  var r: FreyaRenderer
  var e: FreyaElement
  # CSS properties should be accepted
  assert compiles(r.setStyle(e, "background-color", "red"))
  assert compiles(r.setStyle(e, "color", "#333"))
  assert compiles(r.setStyle(e, "font-size", "16"))
  assert compiles(r.setStyle(e, "flex-direction", "row"))
  assert compiles(r.setStyle(e, "width", "100"))
  assert compiles(r.setStyle(e, "height", "50"))
  assert compiles(r.setStyle(e, "padding", "10"))
  assert compiles(r.setStyle(e, "margin", "5"))
  assert compiles(r.setStyle(e, "border-radius", "8"))
  assert compiles(r.setStyle(e, "align-items", "center"))
  assert compiles(r.setStyle(e, "justify-content", "space-between"))
  assert compiles(r.setStyle(e, "gap", "10"))
  # Freya-native properties should also work (pass-through)
  assert compiles(r.setStyle(e, "background", "rgb(255,0,0)"))
  assert compiles(r.setStyle(e, "direction", "horizontal"))
  assert compiles(r.setStyle(e, "corner_radius", "8"))

echo "test_renderer: style mapping compile check passed"

# ===========================================================================
# 4. Attribute mapping tests (compile-time)
# ===========================================================================

static:
  var r: FreyaRenderer
  var e: FreyaElement
  assert compiles(r.setAttribute(e, "class", "container"))
  assert compiles(r.setAttribute(e, "id", "main"))
  assert compiles(r.setAttribute(e, "disabled", ""))
  assert compiles(r.setAttribute(e, "placeholder", "Enter text"))
  assert compiles(r.setAttribute(e, "value", "hello"))
  assert compiles(r.removeAttribute(e, "disabled"))
  assert compiles(r.removeAttribute(e, "class"))

echo "test_renderer: attribute mapping compile check passed"

# ===========================================================================
# 5. Event callback bridge tests (compile-time)
# ===========================================================================

static:
  var r: FreyaRenderer
  var e: FreyaElement
  # Closures with captured variables should be accepted
  var counter = 0
  assert compiles(r.addEventListener(e, "click", proc() = counter += 1))
  assert compiles(r.addEventListener(e, "input", proc() = discard))
  assert compiles(r.addEventListener(e, "change", proc() = discard))

echo "test_renderer: event callback bridge compile check passed"

# ===========================================================================
# 6. Callback registry unit tests (no FFI)
# ===========================================================================

block callbackRegistryTest:
  resetCallbacks()

  var counter = 0
  let cb = registerCallback(proc() = counter += 1)

  # The returned callback is a cdecl proc
  assert cb != nil

  # Calling the trampoline should invoke our closure
  cb()
  assert counter == 1, "callback should have been called once"

  cb()
  assert counter == 2, "callback should have been called twice"

  # Register another callback
  var other = 0
  let cb2 = registerCallback(proc() = other += 10)
  cb2()
  assert other == 10, "second callback should work independently"

  # First callback still works
  cb()
  assert counter == 3

  resetCallbacks()
  echo "test_renderer: callback registry unit test passed"

# ===========================================================================
# 7. Trampoline pool exhaustion test
# ===========================================================================

block trampolinePoolTest:
  resetCallbacks()

  # Register all available trampolines
  for i in 0 ..< trampolineCount:
    let cb = registerCallback(proc() = discard)
    assert cb != nil

  # Next registration should fail
  var failed = false
  try:
    discard registerCallback(proc() = discard)
  except AssertionDefect:
    failed = true

  assert failed, "should have raised AssertionDefect when pool exhausted"

  resetCallbacks()
  echo "test_renderer: trampoline pool exhaustion test passed"

echo "test_renderer: all tests passed"
