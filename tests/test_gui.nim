## GUI tests for isonim-freya.
## Run under headless display: just test-gui-x11
## Or without display (render plan only): nim c -r tests/test_gui.nim
##
## Build & run:
##   LD_LIBRARY_PATH=rust/target/debug nim c -r --path:../isonim/src tests/test_gui.nim

import unittest
import std/[json, strutils]
import isonim_freya/renderer
import isonim_freya/bindings

# ============================================================================
# Helpers
# ============================================================================

proc getPlan(node: FreyaElement): JsonNode =
  ## Build the render plan for a node and parse it as JSON.
  let jsonStr = renderPlanJson(node)
  check jsonStr.len > 0
  result = parseJson(jsonStr)

# ============================================================================
# Render Plan Smoke Tests (no display server required)
# ============================================================================

suite "GUI - Render Plan Smoke Tests":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "create_element_and_verify_render_plan":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let child = r.createElement("span")
    r.appendChild(root, child)
    r.setTextContent(child, "Hello Freya")

    check verifyRenderPlan(root)
    check renderPlanElementCount(root) > 0

    let plan = getPlan(root)
    check plan["kind"].getStr() == "Rect"  # div maps to Rect

  test "styled_element_in_render_plan":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    r.setStyle(root, "background-color", "#ff0000")
    r.setStyle(root, "width", "200px")
    r.setStyle(root, "height", "100px")

    let plan = getPlan(root)
    check plan.hasKey("styles")
    let styles = plan["styles"]
    check styles["background"].getStr() == "#ff0000"
    check styles["width"].getStr() == "200px"
    check styles["height"].getStr() == "100px"

  test "event_handler_in_render_plan":
    let r = FreyaRenderer()
    let btn = r.createElement("button")
    var clicked = false
    r.addEventListener(btn, "click", proc() = clicked = true)

    let plan = getPlan(btn)
    check plan["has_click_handler"].getBool() == true

    # Verify the event actually fires
    fireEvent(btn, "click")
    check clicked == true

  test "nested_tree_render_plan":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let header = r.createElement("h1")
    let nav = r.createElement("nav")
    let item1 = r.createElement("span")
    let item2 = r.createElement("span")

    r.appendChild(root, header)
    r.appendChild(root, nav)
    r.appendChild(nav, item1)
    r.appendChild(nav, item2)
    r.setTextContent(header, "Title")
    r.setTextContent(item1, "Item 1")
    r.setTextContent(item2, "Item 2")

    check verifyRenderPlan(root)
    check renderPlanElementCount(root) >= 5

    let plan = getPlan(root)
    check plan["children"].len == 2

  test "counter_app_render_plan":
    # Build a simple counter app and verify its render plan
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let countLabel = r.createElement("span")
    let incBtn = r.createElement("button")
    let decBtn = r.createElement("button")

    r.appendChild(root, countLabel)
    r.appendChild(root, incBtn)
    r.appendChild(root, decBtn)

    r.setTextContent(countLabel, "0")
    r.setTextContent(incBtn, "+")
    r.setTextContent(decBtn, "-")

    var count = 0
    r.addEventListener(incBtn, "click", proc() =
      count += 1
      r.setTextContent(countLabel, $count)
    )
    r.addEventListener(decBtn, "click", proc() =
      count -= 1
      r.setTextContent(countLabel, $count)
    )

    check verifyRenderPlan(root)
    check textContent(countLabel) == "0"

    fireEvent(incBtn, "click")
    check textContent(countLabel) == "1"
    check count == 1

    fireEvent(incBtn, "click")
    fireEvent(incBtn, "click")
    check textContent(countLabel) == "3"

    fireEvent(decBtn, "click")
    check textContent(countLabel) == "2"

    # Verify render plan still valid after mutations
    check verifyRenderPlan(root)

  test "nil_node_returns_empty_plan":
    let json = renderPlanJson(nil)
    check json == ""
    check not verifyRenderPlan(nil)
    check renderPlanElementCount(nil) == 0

  test "text_node_in_render_plan":
    let r = FreyaRenderer()
    let text = r.createTextNode("hello GUI")
    let plan = getPlan(text)
    check plan["kind"].getStr == "Label"
    check plan["text"].getStr == "hello GUI"

  test "multiple_styles_propagate":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "background-color", "blue")
    r.setStyle(el, "padding", "10")
    r.setStyle(el, "gap", "5")
    let plan = getPlan(el)
    check plan["styles"]["background"].getStr == "blue"
    check plan["styles"]["padding"].getStr == "10"
    check plan["styles"]["spacing"].getStr == "5"

  test "no_handlers_by_default":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    let plan = getPlan(el)
    check plan["has_click_handler"].getBool == false
    check plan["has_input_handler"].getBool == false
    check plan["event_names"].len == 0

# ============================================================================
# Launch Integration Tests (no display server required)
# ============================================================================
#
# These use freya_launch WITHOUT the freya-backend feature (default).
# The launch function calls the root_builder callback, sets up the shadow
# tree, and returns immediately. This tests the full Nim→FFI→Rust→callback→tree
# pipeline.

suite "GUI - Launch Integration Tests":
  setup:
    freya_reset_tree()
    freya_reset_windows()
    resetCallbacks()

  test "freya_launch_calls_root_builder":
    var builderCalled = false
    var rootElement: FreyaElement = nil

    proc builder(root: FreyaElement) {.cdecl.} =
      builderCalled = true
      rootElement = root

    freya_launch("Test App".cstring, 800.0, 600.0, builder)

    check builderCalled
    check rootElement != nil

  test "freya_launch_root_builder_can_build_tree":
    var rootEl: FreyaElement = nil

    proc builder(root: FreyaElement) {.cdecl.} =
      rootEl = root
      # Build a UI tree inside the callback
      let r = FreyaRenderer()
      let header = r.createElement("h1")
      let btn = r.createElement("button")
      r.appendChild(root, header)
      r.appendChild(root, btn)
      r.setTextContent(header, "Hello from Nim!")
      r.setTextContent(btn, "Click me")

    freya_launch("Builder Test".cstring, 640.0, 480.0, builder)

    # After launch returns, verify the tree was built
    check rootEl != nil
    check childCount(rootEl) == 2
    check textContent(rootEl).contains("Hello from Nim!")
    check textContent(rootEl).contains("Click me")

  test "freya_launch_render_plan_valid_after_build":
    var rootEl: FreyaElement = nil

    proc builder(root: FreyaElement) {.cdecl.} =
      rootEl = root
      let r = FreyaRenderer()
      let container = r.createElement("div")
      let label = r.createElement("span")
      r.appendChild(root, container)
      r.appendChild(container, label)
      r.setTextContent(label, "Render plan test")
      r.setStyle(container, "background-color", "blue")
      r.setStyle(container, "width", "300px")

    freya_launch("Plan Test".cstring, 800.0, 600.0, builder)

    check verifyRenderPlan(rootEl)
    check renderPlanElementCount(rootEl) >= 3

    let planJson = renderPlanJson(rootEl)
    check planJson.len > 0
    let plan = parseJson(planJson)
    check plan["kind"].getStr == "Rect"  # root tag maps to Rect
    check plan["children"].len >= 1

  test "freya_launch_event_handlers_work":
    var clickCount = 0
    var btnEl: FreyaElement = nil

    proc builder(root: FreyaElement) {.cdecl.} =
      let r = FreyaRenderer()
      btnEl = r.createElement("button")
      r.appendChild(root, btnEl)
      r.setTextContent(btnEl, "0")
      r.addEventListener(btnEl, "click", proc() =
        clickCount += 1
        r.setTextContent(btnEl, $clickCount)
      )

    freya_launch("Event Test".cstring, 400.0, 300.0, builder)

    check btnEl != nil
    check textContent(btnEl) == "0"

    fireEvent(btnEl, "click")
    check clickCount == 1
    check textContent(btnEl) == "1"

    fireEvent(btnEl, "click")
    fireEvent(btnEl, "click")
    check clickCount == 3
    check textContent(btnEl) == "3"

  test "freya_launch_counter_app_e2e":
    var rootEl, countLabel, incBtn, decBtn: FreyaElement
    var count = 0

    proc builder(root: FreyaElement) {.cdecl.} =
      rootEl = root
      let r = FreyaRenderer()
      countLabel = r.createElement("span")
      incBtn = r.createElement("button")
      decBtn = r.createElement("button")
      r.appendChild(root, countLabel)
      r.appendChild(root, incBtn)
      r.appendChild(root, decBtn)
      r.setTextContent(countLabel, "Count: 0")
      r.setTextContent(incBtn, "+")
      r.setTextContent(decBtn, "-")
      r.addEventListener(incBtn, "click", proc() =
        count += 1
        r.setTextContent(countLabel, "Count: " & $count)
      )
      r.addEventListener(decBtn, "click", proc() =
        count -= 1
        r.setTextContent(countLabel, "Count: " & $count)
      )

    freya_launch("Counter".cstring, 400.0, 300.0, builder)

    # Verify initial state
    check textContent(countLabel) == "Count: 0"
    check verifyRenderPlan(rootEl)

    # Simulate user interactions
    fireEvent(incBtn, "click")
    fireEvent(incBtn, "click")
    fireEvent(incBtn, "click")
    check textContent(countLabel) == "Count: 3"

    fireEvent(decBtn, "click")
    check textContent(countLabel) == "Count: 2"

    # Verify render plan still valid after mutations
    check verifyRenderPlan(rootEl)
    check renderPlanElementCount(rootEl) >= 4

# ============================================================================
# Freya Window Tests (require display server — Xvfb or Wayland)
# ============================================================================

when defined(freyaBackend):
  suite "GUI - Freya Backend Compile Check":
    test "freya_backend_feature_enabled":
      # When compiled with -d:freyaBackend, the Rust shim should be built
      # with --features freya-backend, which enables the actual Freya rendering.
      # We can't test the blocking event loop directly, but we can verify
      # that window management works.
      freya_reset_tree()
      freya_reset_windows()

      let winId = freya_create_window("Backend Test".cstring, 640.0, 480.0)
      check winId > 0
      check freya_window_state(winId) == 1  # Created state

      check freya_show_window(winId) == 1
      check freya_window_state(winId) == 2  # Visible state

      check freya_close_window(winId) == 1
      check freya_window_state(winId) == 4  # Closed state

      freya_destroy_window(winId)
