## FreyaWindow — high-level window management for the IsoNim Freya backend.
##
## Provides:
## - Window creation with title and initial size
## - Show / close lifecycle
## - Lifecycle event callbacks (resize, focus, close)
## - Repaint request integration for reactive updates
##
## The window state machine:
##   Created -> Visible -> Closed
##
## Usage:
##   var win = createWindow("My App", 800, 600)
##   win.onResize proc(w, h: float) = echo "resized: ", w, "x", h
##   win.onFocus proc(focused: bool) = echo "focused: ", focused
##   win.onClose proc(): bool = true  # allow close
##   win.show()
##   # ... reactive updates happen, repaint is requested automatically ...
##   win.close()

import isonim_freya/bindings

type
  WindowState* = enum
    ## Window lifecycle states.
    wsNotFound = 0    ## Window ID not found in registry
    wsCreated = 1     ## Window created but not yet shown
    wsVisible = 2     ## Window is visible / event loop running
    wsCloseRequested = 3  ## Close has been requested (pending)
    wsClosed = 4      ## Window is closed

  FreyaWindow* = object
    ## Handle to a Freya window managed by the Rust shim.
    id*: uint32

# ===========================================================================
# Callback bridge
# ===========================================================================
#
# Similar to the event callback bridge in renderer.nim, we need cdecl
# trampolines for window lifecycle callbacks. We use a simpler approach
# here since there are at most a few windows and 3 callback types each.

var
  resizeCallbacks: array[4, proc(width, height: float)]
  focusCallbacks: array[4, proc(focused: bool)]
  closeCallbacks: array[4, proc(): bool]

# Trampolines for resize (up to 4 windows)
proc resizeTrampoline0(w, h: cdouble) {.cdecl.} =
  if resizeCallbacks[0] != nil: resizeCallbacks[0](w.float, h.float)
proc resizeTrampoline1(w, h: cdouble) {.cdecl.} =
  if resizeCallbacks[1] != nil: resizeCallbacks[1](w.float, h.float)
proc resizeTrampoline2(w, h: cdouble) {.cdecl.} =
  if resizeCallbacks[2] != nil: resizeCallbacks[2](w.float, h.float)
proc resizeTrampoline3(w, h: cdouble) {.cdecl.} =
  if resizeCallbacks[3] != nil: resizeCallbacks[3](w.float, h.float)

const resizeTrampolines: array[4, ResizeCallback] = [
  resizeTrampoline0, resizeTrampoline1,
  resizeTrampoline2, resizeTrampoline3,
]

# Trampolines for focus
proc focusTrampoline0(f: uint8) {.cdecl.} =
  if focusCallbacks[0] != nil: focusCallbacks[0](f != 0)
proc focusTrampoline1(f: uint8) {.cdecl.} =
  if focusCallbacks[1] != nil: focusCallbacks[1](f != 0)
proc focusTrampoline2(f: uint8) {.cdecl.} =
  if focusCallbacks[2] != nil: focusCallbacks[2](f != 0)
proc focusTrampoline3(f: uint8) {.cdecl.} =
  if focusCallbacks[3] != nil: focusCallbacks[3](f != 0)

const focusTrampolines: array[4, FocusCallback] = [
  focusTrampoline0, focusTrampoline1,
  focusTrampoline2, focusTrampoline3,
]

# Trampolines for close
proc closeTrampoline0(): uint8 {.cdecl.} =
  if closeCallbacks[0] != nil:
    if closeCallbacks[0](): 1'u8 else: 0'u8
  else: 1'u8
proc closeTrampoline1(): uint8 {.cdecl.} =
  if closeCallbacks[1] != nil:
    if closeCallbacks[1](): 1'u8 else: 0'u8
  else: 1'u8
proc closeTrampoline2(): uint8 {.cdecl.} =
  if closeCallbacks[2] != nil:
    if closeCallbacks[2](): 1'u8 else: 0'u8
  else: 1'u8
proc closeTrampoline3(): uint8 {.cdecl.} =
  if closeCallbacks[3] != nil:
    if closeCallbacks[3](): 1'u8 else: 0'u8
  else: 1'u8

const closeTrampolines: array[4, CloseCallback] = [
  closeTrampoline0, closeTrampoline1,
  closeTrampoline2, closeTrampoline3,
]

var nextWindowSlot: int

proc allocWindowSlot(): int =
  ## Allocate a trampoline slot for a window. Returns the slot index.
  assert nextWindowSlot < 4,
    "FreyaWindow: maximum number of concurrent windows (4) exceeded"
  result = nextWindowSlot
  inc nextWindowSlot

# ===========================================================================
# Window API
# ===========================================================================

proc createWindow*(title: string; width, height: float): FreyaWindow =
  ## Create a new window with the given title and initial size.
  let id = freya_create_window(title.cstring, width.cdouble, height.cdouble)
  assert id > 0, "FreyaWindow: failed to create window"
  FreyaWindow(id: id)

proc state*(win: FreyaWindow): WindowState =
  ## Get the current lifecycle state of the window.
  WindowState(freya_window_state(win.id))

proc width*(win: FreyaWindow): float =
  ## Get the current window width.
  freya_window_width(win.id).float

proc height*(win: FreyaWindow): float =
  ## Get the current window height.
  freya_window_height(win.id).float

proc size*(win: FreyaWindow): tuple[width, height: float] =
  ## Get the current window size.
  (win.width, win.height)

proc show*(win: FreyaWindow): bool =
  ## Show the window (transition from Created to Visible).
  ## Returns true if the transition was successful.
  freya_show_window(win.id) != 0

proc close*(win: FreyaWindow): bool =
  ## Request window close. If an onClose callback is registered and
  ## returns false, the close is denied.
  ## Returns true if the window was closed.
  freya_close_window(win.id) != 0

proc destroy*(win: FreyaWindow) =
  ## Destroy the window and free its resources.
  freya_destroy_window(win.id)

proc onResize*(win: FreyaWindow; callback: proc(width, height: float)) =
  ## Register a callback for window resize events.
  let slot = allocWindowSlot()
  resizeCallbacks[slot] = callback
  freya_on_resize(win.id, resizeTrampolines[slot])

proc onFocus*(win: FreyaWindow; callback: proc(focused: bool)) =
  ## Register a callback for window focus events.
  # Reuse the same slot logic — for simplicity, we just use the
  # next available slot. In practice a window would register all its
  # callbacks at once.
  let slot = nextWindowSlot - 1  # use same slot as last alloc
  if slot < 0 or slot >= 4:
    let newSlot = allocWindowSlot()
    focusCallbacks[newSlot] = callback
    freya_on_focus(win.id, focusTrampolines[newSlot])
  else:
    focusCallbacks[slot] = callback
    freya_on_focus(win.id, focusTrampolines[slot])

proc onClose*(win: FreyaWindow; callback: proc(): bool) =
  ## Register a callback for window close requests.
  ## Return true from the callback to allow closing, false to prevent it.
  let slot = nextWindowSlot - 1
  if slot < 0 or slot >= 4:
    let newSlot = allocWindowSlot()
    closeCallbacks[newSlot] = callback
    freya_on_close(win.id, closeTrampolines[newSlot])
  else:
    closeCallbacks[slot] = callback
    freya_on_close(win.id, closeTrampolines[slot])

proc requestRepaint*() =
  ## Request a repaint of the active window. Call this after modifying
  ## the shadow tree to trigger a re-render on the next frame.
  ## Note: Tree mutation functions (appendChild, setAttribute, etc.)
  ## automatically request repaint, so this is only needed for
  ## manual/explicit repaint triggers.
  freya_request_repaint()

proc repaintPending*(): bool =
  ## Check if a repaint has been requested (and clear the flag).
  ## This is primarily useful for testing and custom render loops.
  freya_take_repaint_request() != 0

proc resetWindows*() =
  ## Reset all window state (for testing).
  freya_reset_windows()
  for i in 0 ..< 4:
    resizeCallbacks[i] = nil
    focusCallbacks[i] = nil
    closeCallbacks[i] = nil
  nextWindowSlot = 0
