## Cross-renderer tests for FreyaRenderer (M5).
##
## Ports the generic createCounter and createTaskList test components from
## isonim's test_native_renderer.nim and verifies that FreyaRenderer produces
## the same behavior as MockRenderer and TerminalRenderer.
##
## These tests link against the Rust shim and require it to be built first:
##   just rust-build
##
## Run with:
##   LD_LIBRARY_PATH=rust/target/debug nim c -r --path:../isonim/src tests/test_cross_renderer.nim

import unittest
import std/tables

# IsoNim renderers
import isonim/testing/mock_dom
import isonim/renderers/terminal_demo as terminal

# IsoNim reactive core
import isonim/core/[signals, computation, owner]

# FreyaRenderer
import isonim_freya/renderer
import isonim_freya/bindings

# ============================================================================
# Generic test components (work with any RendererBackend)
# ============================================================================
# These are the same components from isonim/tests/test_native_renderer.nim,
# duplicated here to avoid depending on isonim's test files at import time.

proc createCounter[R, N](renderer: R): N =
  ## A counter component that works with any RendererBackend.
  var count = createSignal(0)
  let container = renderer.createElement("div")
  let label = renderer.createTextNode("")
  let incBtn = renderer.createElement("button")
  let decBtn = renderer.createElement("button")

  renderer.appendChild(incBtn, renderer.createTextNode("+"))
  renderer.appendChild(decBtn, renderer.createTextNode("-"))
  renderer.appendChild(container, label)
  renderer.appendChild(container, incBtn)
  renderer.appendChild(container, decBtn)

  renderer.addEventListener(incBtn, "click", proc() =
    count.val = count.val + 1
  )
  renderer.addEventListener(decBtn, "click", proc() =
    count.val = count.val - 1
  )

  createRenderEffect proc() =
    renderer.setTextContent(label, "Count: " & $count.val)

  return container

proc createTaskList[R, N](renderer: R; items: seq[string]): N =
  ## A list component that works with any RendererBackend.
  let ul = renderer.createElement("ul")
  for item in items:
    let li = renderer.createElement("li")
    let span = renderer.createElement("span")
    renderer.setTextContent(span, item)
    renderer.appendChild(li, span)
    renderer.appendChild(ul, li)
  return ul

# ============================================================================
# FreyaRenderer-specific tree inspection helpers
# ============================================================================

proc freyaChildCount(node: FreyaElement): int =
  renderer.childCount(node)

proc freyaTextContent(node: FreyaElement): string =
  renderer.textContent(node)

# ============================================================================
# Test suites
# ============================================================================

suite "FreyaRenderer - Basic Operations":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "createElement returns non-null":
    let r = FreyaRenderer()
    let node = r.createElement("div")
    check node != nil

  test "createTextNode returns non-null":
    let r = FreyaRenderer()
    let node = r.createTextNode("hello")
    check node != nil

  test "appendChild builds tree":
    let r = FreyaRenderer()
    let parent = r.createElement("div")
    let child = r.createElement("span")
    r.appendChild(parent, child)
    check freyaChildCount(parent) == 1
    check r.firstChild(parent) != nil

  test "removeChild removes from tree":
    let r = FreyaRenderer()
    let parent = r.createElement("div")
    let child = r.createElement("span")
    r.appendChild(parent, child)
    r.removeChild(parent, child)
    check freyaChildCount(parent) == 0

  test "insertBefore inserts at correct position":
    let r = FreyaRenderer()
    let parent = r.createElement("div")
    let a = r.createElement("span")
    let b = r.createElement("span")
    let c = r.createElement("span")
    r.appendChild(parent, a)
    r.appendChild(parent, c)
    r.insertBefore(parent, b, c)
    check freyaChildCount(parent) == 3

  test "tree navigation works":
    let r = FreyaRenderer()
    let parent = r.createElement("div")
    let a = r.createElement("span")
    let b = r.createElement("span")
    r.appendChild(parent, a)
    r.appendChild(parent, b)
    check r.firstChild(parent) != nil
    check r.nextSibling(a) != nil
    check r.parentNode(a) != nil

  test "setText and getText round-trip on text node":
    let r = FreyaRenderer()
    let node = r.createTextNode("initial")
    check freyaTextContent(node) == "initial"
    r.setTextContent(node, "updated")
    check freyaTextContent(node) == "updated"

  test "setAttribute and getAttribute round-trip":
    let r = FreyaRenderer()
    let node = r.createElement("div")
    r.setAttribute(node, "class", "container")
    check renderer.getAttribute(node, "class") == "container"

suite "FreyaRenderer - Reactive Counter":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "reactive counter works with FreyaRenderer":
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      let counter = createCounter[FreyaRenderer, FreyaElement](r)

      check freyaTextContent(counter) == "Count: 0+-"

      # Click increment (child index 1 = incBtn)
      let incBtn = nthChild(counter, 1)
      check incBtn != nil
      fireEvent(incBtn, "click")
      check freyaTextContent(counter) == "Count: 1+-"

      fireEvent(incBtn, "click")
      fireEvent(incBtn, "click")
      check freyaTextContent(counter) == "Count: 3+-"

      # Click decrement (child index 2 = decBtn)
      let decBtn = nthChild(counter, 2)
      check decBtn != nil
      fireEvent(decBtn, "click")
      check freyaTextContent(counter) == "Count: 2+-"

      dispose()

suite "FreyaRenderer - Task List":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "task list renders with FreyaRenderer":
    let r = FreyaRenderer()
    let list = createTaskList[FreyaRenderer, FreyaElement](r,
      @["Buy groceries", "Write code", "Test app"])

    check freyaChildCount(list) == 3
    let child0 = nthChild(list, 0)
    let child1 = nthChild(list, 1)
    let child2 = nthChild(list, 2)
    check freyaTextContent(child0) == "Buy groceries"
    check freyaTextContent(child1) == "Write code"
    check freyaTextContent(child2) == "Test app"

suite "Cross-Renderer Compatibility":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "same counter component works across FreyaRenderer and MockRenderer":
    createRoot proc(dispose: proc()) =
      let fr = FreyaRenderer()
      let mr = MockRenderer()

      let freyaCounter = createCounter[FreyaRenderer, FreyaElement](fr)
      let mockCounter = createCounter[MockRenderer, MockNode](mr)

      # Both start at 0
      check freyaTextContent(freyaCounter) == "Count: 0+-"
      check mock_dom.textContent(mockCounter) == "Count: 0+-"

      # Same child count
      check freyaChildCount(freyaCounter) == mockCounter.children.len

      # Increment Freya counter
      let freyaIncBtn = nthChild(freyaCounter, 1)
      fireEvent(freyaIncBtn, "click")
      check freyaTextContent(freyaCounter) == "Count: 1+-"

      # Increment Mock counter
      mockCounter.children[1].fireEvent("click")
      check mock_dom.textContent(mockCounter) == "Count: 1+-"

      # Both produce same text after increment
      check freyaTextContent(freyaCounter) == mock_dom.textContent(mockCounter)

      dispose()

  test "same counter component works across FreyaRenderer and TerminalRenderer":
    createRoot proc(dispose: proc()) =
      let fr = FreyaRenderer()
      let tr = TerminalRenderer()

      let freyaCounter = createCounter[FreyaRenderer, FreyaElement](fr)
      let terminalCounter = createCounter[TerminalRenderer, TerminalNode](tr)

      # Both start at 0
      check freyaTextContent(freyaCounter) == "Count: 0+-"
      check terminal.textContent(terminalCounter) == "Count: 0+-"

      # Increment both
      let freyaIncBtn = nthChild(freyaCounter, 1)
      fireEvent(freyaIncBtn, "click")
      terminalCounter.children[1].fireEvent("click")

      # Both at 1
      check freyaTextContent(freyaCounter) == "Count: 1+-"
      check terminal.textContent(terminalCounter) == "Count: 1+-"
      check freyaTextContent(freyaCounter) == terminal.textContent(terminalCounter)

      # Same child count
      check freyaChildCount(freyaCounter) == terminalCounter.children.len

      dispose()

  test "same task list works across all renderers":
    let fr = FreyaRenderer()
    let tr = TerminalRenderer()
    let mr = MockRenderer()
    let items = @["Alpha", "Beta", "Gamma"]

    let fList = createTaskList[FreyaRenderer, FreyaElement](fr, items)
    let tList = createTaskList[TerminalRenderer, TerminalNode](tr, items)
    let mList = createTaskList[MockRenderer, MockNode](mr, items)

    # Same child count
    check freyaChildCount(fList) == 3
    check tList.children.len == 3
    check mList.children.len == 3

    # Same text content per item
    for i in 0..2:
      let fChild = nthChild(fList, i)
      check freyaTextContent(fChild) == items[i]
      check terminal.textContent(tList.children[i]) == items[i]
      check mock_dom.textContent(mList.children[i]) == items[i]

suite "FreyaRenderer - Task Manager Demo":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "task manager app structure":
    ## Simplified version of the demo app to verify FreyaRenderer can
    ## build a realistic component tree.
    let r = FreyaRenderer()

    # App root
    let app = r.createElement("div")
    r.setAttribute(app, "class", "app")

    # Header
    let header = r.createElement("header")
    let title = r.createElement("h1")
    r.setTextContent(title, "Task Manager")
    r.appendChild(header, title)
    r.appendChild(app, header)

    # Input area
    let inputArea = r.createElement("div")
    r.setAttribute(inputArea, "class", "input-area")
    let input = r.createElement("input")
    r.setAttribute(input, "placeholder", "New task...")
    let addBtn = r.createElement("button")
    r.setTextContent(addBtn, "Add")
    r.appendChild(inputArea, input)
    r.appendChild(inputArea, addBtn)
    r.appendChild(app, inputArea)

    # Task list
    let taskList = r.createElement("ul")
    r.setAttribute(taskList, "class", "task-list")

    var tasks = @["Design API", "Write tests", "Deploy"]
    for task in tasks:
      let li = r.createElement("li")
      let checkbox = r.createElement("input")
      r.setAttribute(checkbox, "type", "checkbox")
      let label = r.createElement("span")
      r.setTextContent(label, task)
      r.appendChild(li, checkbox)
      r.appendChild(li, label)
      r.appendChild(taskList, li)

    r.appendChild(app, taskList)

    # Footer
    let footer = r.createElement("footer")
    let count = r.createElement("span")
    r.setTextContent(count, "3 tasks")
    r.appendChild(footer, count)
    r.appendChild(app, footer)

    # Verify structure
    check freyaChildCount(app) == 4 # header, inputArea, taskList, footer
    check freyaChildCount(taskList) == 3 # 3 task items
    check renderer.getAttribute(app, "class") == "app"
    check renderer.getAttribute(taskList, "class") == "task-list"
    check freyaTextContent(footer) == "3 tasks"

    # Verify task text
    for i in 0..2:
      let li = nthChild(taskList, i)
      check freyaTextContent(li) == tasks[i]

  test "task manager dynamic updates":
    ## Verify that reactive updates work in a task-manager-like component.
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var taskCount = createSignal(0)

      let app = r.createElement("div")
      let counter = r.createElement("span")
      r.appendChild(app, counter)

      createRenderEffect proc() =
        r.setTextContent(counter, $taskCount.val & " tasks")

      check freyaTextContent(counter) == "0 tasks"

      taskCount.val = 1
      check freyaTextContent(counter) == "1 tasks"

      taskCount.val = 5
      check freyaTextContent(counter) == "5 tasks"

      dispose()
