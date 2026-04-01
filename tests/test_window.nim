## Tests for FreyaWindow — M4 window management and event loop integration.
##
## These tests verify:
## - Window creation, show, close lifecycle
## - Lifecycle event callbacks (resize, focus, close)
## - Repaint flag integration with tree mutations
## - Window state machine transitions
## - Error handling for invalid operations
##
## To run:
##   nim c -r --nimcache:nimcache/test_window tests/test_window.nim
## (requires Rust shim: just rust-build)

import isonim_freya/bindings
import isonim_freya/window

# ===========================================================================
# 1. Compile-time type checks
# ===========================================================================

static:
  # Window creation
  assert compiles(createWindow("Test", 800.0, 600.0))

  # Window properties
  var w: FreyaWindow
  assert compiles(w.state)
  assert compiles(w.width)
  assert compiles(w.height)
  assert compiles(w.size)

  # Lifecycle
  assert compiles(w.show())
  assert compiles(w.close())
  assert compiles(w.destroy())

  # Callbacks
  assert compiles(w.onResize(proc(width, height: float) = discard))
  assert compiles(w.onFocus(proc(focused: bool) = discard))
  assert compiles(w.onClose(proc(): bool = true))

  # Repaint
  assert compiles(requestRepaint())
  assert compiles(repaintPending())

echo "test_window: compile-time type checks passed"

# ===========================================================================
# 2. Window creation and properties
# ===========================================================================

block windowCreation:
  resetWindows()
  freya_reset_tree()

  let win = createWindow("Test Window", 1024.0, 768.0)
  assert win.id > 0, "window ID should be positive"
  assert win.state == wsCreated, "new window should be in Created state"
  assert win.width == 1024.0, "width should match creation parameter"
  assert win.height == 768.0, "height should match creation parameter"

  let (w, h) = win.size
  assert w == 1024.0
  assert h == 768.0

  win.destroy()
  echo "test_window: window creation passed"

# ===========================================================================
# 3. Window lifecycle state machine
# ===========================================================================

block windowLifecycle:
  resetWindows()

  let win = createWindow("Lifecycle Test", 640.0, 480.0)
  assert win.state == wsCreated

  # Show
  assert win.show() == true, "show should succeed from Created"
  assert win.state == wsVisible

  # Cannot show again
  assert win.show() == false, "show should fail from Visible"
  assert win.state == wsVisible

  # Close
  assert win.close() == true, "close should succeed from Visible"
  assert win.state == wsClosed

  win.destroy()
  echo "test_window: window lifecycle passed"

# ===========================================================================
# 4. Resize callback
# ===========================================================================

block resizeCallback:
  resetWindows()

  let win = createWindow("Resize Test", 800.0, 600.0)

  var resizedWidth = 0.0
  var resizedHeight = 0.0

  win.onResize proc(w, h: float) =
    resizedWidth = w
    resizedHeight = h

  # Simulate resize via the low-level notify function
  freya_notify_resize(win.id, 1920.0, 1080.0)

  assert resizedWidth == 1920.0, "resize callback should receive new width"
  assert resizedHeight == 1080.0, "resize callback should receive new height"
  assert win.width == 1920.0, "window width should be updated"
  assert win.height == 1080.0, "window height should be updated"

  win.destroy()
  echo "test_window: resize callback passed"

# ===========================================================================
# 5. Focus callback
# ===========================================================================

block focusCallback:
  resetWindows()

  let win = createWindow("Focus Test", 800.0, 600.0)

  var lastFocusState = false

  win.onResize proc(w, h: float) = discard  # allocate slot first
  win.onFocus proc(focused: bool) =
    lastFocusState = focused

  # Simulate focus events
  freya_notify_focus(win.id, 1)
  assert lastFocusState == true, "focus callback should receive true"

  freya_notify_focus(win.id, 0)
  assert lastFocusState == false, "focus callback should receive false"

  win.destroy()
  echo "test_window: focus callback passed"

# ===========================================================================
# 6. Close callback (allow)
# ===========================================================================

block closeCallbackAllow:
  resetWindows()

  let win = createWindow("Close Allow Test", 800.0, 600.0)
  discard win.show()

  win.onResize proc(w, h: float) = discard  # allocate slot
  win.onClose proc(): bool = true  # allow close

  assert win.close() == true, "close should be allowed"
  assert win.state == wsClosed

  win.destroy()
  echo "test_window: close callback (allow) passed"

# ===========================================================================
# 7. Close callback (deny)
# ===========================================================================

block closeCallbackDeny:
  resetWindows()

  let win = createWindow("Close Deny Test", 800.0, 600.0)
  discard win.show()

  win.onResize proc(w, h: float) = discard
  win.onClose proc(): bool = false  # deny close

  assert win.close() == false, "close should be denied"
  assert win.state == wsVisible, "window should remain Visible after denied close"

  win.destroy()
  echo "test_window: close callback (deny) passed"

# ===========================================================================
# 8. Repaint integration with tree mutations
# ===========================================================================
#
# We test repaint integration using the raw bindings directly rather than
# the FreyaRenderer (which uses const tables that trigger a Nim compiler
# bug when bindings is imported alongside renderer).

block repaintIntegration:
  resetWindows()
  freya_reset_tree()
  discard repaintPending()  # clear

  # Create elements via raw bindings
  let parent = freya_create_element("rect".cstring)
  let child = freya_create_element("label".cstring)
  discard repaintPending()  # clear any residual

  # appendChild should trigger repaint
  freya_append_child(parent, child)
  assert repaintPending() == true, "appendChild should request repaint"

  # setAttribute should trigger repaint
  freya_set_attribute(parent, "width".cstring, "100".cstring)
  assert repaintPending() == true, "setAttribute should request repaint"

  # setStyle should trigger repaint
  freya_set_style(parent, "background".cstring, "red".cstring)
  assert repaintPending() == true, "setStyle should request repaint"

  # setTextContent should trigger repaint
  freya_set_text_content(child, "hello".cstring)
  assert repaintPending() == true, "setTextContent should request repaint"

  # removeChild should trigger repaint
  freya_remove_child(parent, child)
  assert repaintPending() == true, "removeChild should request repaint"

  # insertBefore should trigger repaint
  freya_append_child(parent, child)
  discard repaintPending()  # clear
  let child2 = freya_create_element("label".cstring)
  discard repaintPending()
  freya_insert_before(parent, child2, child)
  assert repaintPending() == true, "insertBefore should request repaint"

  # No more pending
  assert repaintPending() == false, "no repaint should be pending"

  echo "test_window: repaint integration passed"

# ===========================================================================
# 9. Manual repaint request
# ===========================================================================

block manualRepaint:
  resetWindows()
  discard repaintPending()  # clear

  requestRepaint()
  assert repaintPending() == true, "manual requestRepaint should set flag"
  assert repaintPending() == false, "flag should be cleared after take"

  echo "test_window: manual repaint passed"

# ===========================================================================
# 10. Multiple windows
# ===========================================================================

block multipleWindows:
  resetWindows()

  let win1 = createWindow("Window 1", 800.0, 600.0)
  let win2 = createWindow("Window 2", 1024.0, 768.0)

  assert win1.id != win2.id, "windows should have different IDs"
  assert win1.width == 800.0
  assert win2.width == 1024.0

  discard win1.show()
  assert win1.state == wsVisible
  assert win2.state == wsCreated

  discard win2.show()
  assert win2.state == wsVisible

  discard win1.close()
  assert win1.state == wsClosed
  assert win2.state == wsVisible

  win1.destroy()
  win2.destroy()
  echo "test_window: multiple windows passed"

# ===========================================================================
# 11. Window not found
# ===========================================================================

block windowNotFound:
  resetWindows()

  let fake = FreyaWindow(id: 999)
  assert fake.state == wsNotFound, "nonexistent window should be wsNotFound"
  assert fake.width == 0.0
  assert fake.height == 0.0
  assert fake.show() == false
  assert fake.close() == false
  # destroy should not crash
  fake.destroy()

  echo "test_window: window not found handling passed"

echo "test_window: all tests passed"
