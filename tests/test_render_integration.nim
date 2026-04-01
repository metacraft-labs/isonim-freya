## Integration tests for the full Nim → FFI → Rust → render-plan pipeline (G3-F).
##
## These tests verify that:
## 1. FreyaRenderer creates elements that produce valid render plans
## 2. The render plan has correct Freya element types (Rect, Label, Paragraph)
## 3. Styles are mapped correctly through the render plan
## 4. Event handlers are present in the render plan
## 5. Render plan updates after reactive changes
##
## Unlike test_cross_renderer.nim (which reads back from the shadow tree),
## these tests query the render plan — the intermediate representation that
## drives actual Freya rendering.
##
## Build & run:
##   LD_LIBRARY_PATH=rust/target/debug nim c -r --path:../isonim/src tests/test_render_integration.nim

import unittest
import std/[json, strutils]

# IsoNim reactive core
import isonim/core/[signals, computation, owner]

# FreyaRenderer
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
# Test suites
# ============================================================================

suite "Render Plan - Basic Elements":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "div maps to Rect in render plan":
    let r = FreyaRenderer()
    let divEl = r.createElement("div")
    let plan = getPlan(divEl)
    check plan["kind"].getStr == "Rect"
    check verifyRenderPlan(divEl)

  test "span maps to Label in render plan":
    let r = FreyaRenderer()
    let span = r.createElement("span")
    let plan = getPlan(span)
    check plan["kind"].getStr == "Label"

  test "p maps to Paragraph in render plan":
    let r = FreyaRenderer()
    let p = r.createElement("p")
    let plan = getPlan(p)
    check plan["kind"].getStr == "Paragraph"

  test "button maps to Rect in render plan":
    let r = FreyaRenderer()
    let btn = r.createElement("button")
    let plan = getPlan(btn)
    check plan["kind"].getStr == "Rect"

  test "text node maps to Label with text":
    let r = FreyaRenderer()
    let text = r.createTextNode("hello")
    let plan = getPlan(text)
    check plan["kind"].getStr == "Label"
    check plan["text"].getStr == "hello"

  test "HTML semantic tags map to Rect":
    let r = FreyaRenderer()
    for tag in ["section", "header", "footer", "nav", "main", "article",
                "form", "ul", "ol", "li"]:
      freya_reset_tree()
      let elem = r.createElement(tag)
      let plan = getPlan(elem)
      check plan["kind"].getStr == "Rect"

  test "HTML text tags map to Label":
    let r = FreyaRenderer()
    for tag in ["span", "h1", "h2", "h3", "h4", "h5", "h6", "label",
                "strong", "em", "code"]:
      freya_reset_tree()
      let elem = r.createElement(tag)
      let plan = getPlan(elem)
      check plan["kind"].getStr == "Label"

  test "nil node returns empty plan":
    let json = renderPlanJson(nil)
    check json == ""
    check not verifyRenderPlan(nil)
    check renderPlanElementCount(nil) == 0

suite "Render Plan - Tree Structure":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "parent-child structure in render plan":
    let r = FreyaRenderer()
    let parent = r.createElement("div")
    let child1 = r.createElement("span")
    let child2 = r.createElement("span")
    r.appendChild(parent, child1)
    r.appendChild(parent, child2)

    let plan = getPlan(parent)
    check plan["kind"].getStr == "Rect"
    check plan["children"].len == 2
    check plan["children"][0]["kind"].getStr == "Label"
    check plan["children"][1]["kind"].getStr == "Label"

  test "element count matches tree size":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let a = r.createElement("span")
    let b = r.createElement("span")
    let c = r.createTextNode("text")
    r.appendChild(a, c)
    r.appendChild(root, a)
    r.appendChild(root, b)

    check renderPlanElementCount(root) == 4

  test "deep nesting in render plan":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let inner = r.createElement("div")
    let span = r.createElement("span")
    let text = r.createTextNode("nested")
    r.appendChild(span, text)
    r.appendChild(inner, span)
    r.appendChild(root, inner)

    let plan = getPlan(root)
    check plan["children"][0]["children"][0]["children"][0]["text"].getStr == "nested"
    check renderPlanElementCount(root) == 4

suite "Render Plan - Styles":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "CSS background-color maps to background":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "background-color", "red")
    let plan = getPlan(el)
    check plan["styles"]["background"].getStr == "red"

  test "CSS flex-direction row maps to horizontal":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "flex-direction", "row")
    let plan = getPlan(el)
    check plan["styles"]["direction"].getStr == "horizontal"

  test "CSS flex-direction column maps to vertical":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "flex-direction", "column")
    let plan = getPlan(el)
    check plan["styles"]["direction"].getStr == "vertical"

  test "width and height propagate":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "width", "200px")
    r.setStyle(el, "height", "100px")
    let plan = getPlan(el)
    check plan["styles"]["width"].getStr == "200px"
    check plan["styles"]["height"].getStr == "100px"

  test "CSS border-radius maps to corner_radius":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "border-radius", "8")
    let plan = getPlan(el)
    check plan["styles"]["corner_radius"].getStr == "8"

  test "font-size maps to font_size":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "font-size", "16")
    let plan = getPlan(el)
    check plan["styles"]["font_size"].getStr == "16"

  test "multiple styles propagate together":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    r.setStyle(el, "background-color", "blue")
    r.setStyle(el, "padding", "10")
    r.setStyle(el, "gap", "5")
    let plan = getPlan(el)
    check plan["styles"]["background"].getStr == "blue"
    check plan["styles"]["padding"].getStr == "10"
    check plan["styles"]["spacing"].getStr == "5"

suite "Render Plan - Event Handlers":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "click handler shows in render plan":
    let r = FreyaRenderer()
    let btn = r.createElement("button")
    r.addEventListener(btn, "click", proc() = discard)
    let plan = getPlan(btn)
    check plan["has_click_handler"].getBool == true
    check plan["has_input_handler"].getBool == false

  test "no handlers by default":
    let r = FreyaRenderer()
    let el = r.createElement("div")
    let plan = getPlan(el)
    check plan["has_click_handler"].getBool == false
    check plan["has_input_handler"].getBool == false
    check plan["event_names"].len == 0

  test "event handlers preserved in children":
    let r = FreyaRenderer()
    let root = r.createElement("div")
    let btn = r.createElement("button")
    r.addEventListener(btn, "click", proc() = discard)
    r.appendChild(root, btn)

    let plan = getPlan(root)
    check plan["has_click_handler"].getBool == false
    check plan["children"][0]["has_click_handler"].getBool == true

  test "click dispatch works and handler fires":
    var clicked = false
    let r = FreyaRenderer()
    let btn = r.createElement("button")
    r.addEventListener(btn, "click", proc() = clicked = true)
    let plan = getPlan(btn)
    check plan["has_click_handler"].getBool == true

    fireEvent(btn, "click")
    check clicked == true

suite "Render Plan - Counter App":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "counter app render plan structure":
    ## Build a counter app with FreyaRenderer and verify the render plan.
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var count = createSignal(0)

      let container = r.createElement("div")
      let label = r.createTextNode("")
      let incBtn = r.createElement("button")
      let decBtn = r.createElement("button")

      r.appendChild(incBtn, r.createTextNode("+"))
      r.appendChild(decBtn, r.createTextNode("-"))
      r.appendChild(container, label)
      r.appendChild(container, incBtn)
      r.appendChild(container, decBtn)

      r.addEventListener(incBtn, "click", proc() =
        count.val = count.val + 1
      )
      r.addEventListener(decBtn, "click", proc() =
        count.val = count.val - 1
      )

      createRenderEffect proc() =
        r.setTextContent(label, "Count: " & $count.val)

      # Verify render plan structure
      let plan = getPlan(container)
      check plan["kind"].getStr == "Rect"  # div → Rect
      check plan["children"].len == 3       # label + incBtn + decBtn
      check verifyRenderPlan(container)

      # label (text node) → Label
      check plan["children"][0]["kind"].getStr == "Label"
      check plan["children"][0]["text"].getStr == "Count: 0"

      # inc button → Rect with click handler
      check plan["children"][1]["kind"].getStr == "Rect"
      check plan["children"][1]["has_click_handler"].getBool == true

      # dec button → Rect with click handler
      check plan["children"][2]["kind"].getStr == "Rect"
      check plan["children"][2]["has_click_handler"].getBool == true

      dispose()

  test "counter app render plan updates after click":
    ## Verify that reactive changes update the shadow tree, and the rebuilt
    ## render plan reflects those changes.
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var count = createSignal(0)

      let container = r.createElement("div")
      let label = r.createTextNode("")
      let incBtn = r.createElement("button")
      r.appendChild(incBtn, r.createTextNode("+"))
      r.appendChild(container, label)
      r.appendChild(container, incBtn)

      r.addEventListener(incBtn, "click", proc() =
        count.val = count.val + 1
      )

      createRenderEffect proc() =
        r.setTextContent(label, "Count: " & $count.val)

      # Initial state
      var plan = getPlan(container)
      check plan["children"][0]["text"].getStr == "Count: 0"

      # Click increment
      fireEvent(incBtn, "click")
      plan = getPlan(container)
      check plan["children"][0]["text"].getStr == "Count: 1"

      # Click again
      fireEvent(incBtn, "click")
      fireEvent(incBtn, "click")
      plan = getPlan(container)
      check plan["children"][0]["text"].getStr == "Count: 3"

      dispose()

suite "Render Plan - Task Manager Demo":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "task manager render plan structure":
    ## Build a simplified task manager and verify its render plan has the
    ## correct Freya element types.
    let r = FreyaRenderer()

    let app = r.createElement("div")
    let header = r.createElement("header")
    let title = r.createElement("h1")
    r.setTextContent(title, "Task Manager")
    r.appendChild(header, title)
    r.appendChild(app, header)

    let inputArea = r.createElement("div")
    let addBtn = r.createElement("button")
    r.setTextContent(addBtn, "Add")
    r.addEventListener(addBtn, "click", proc() = discard)
    r.appendChild(inputArea, addBtn)
    r.appendChild(app, inputArea)

    let taskList = r.createElement("ul")
    for task in ["Design API", "Write tests", "Deploy"]:
      let li = r.createElement("li")
      let span = r.createElement("span")
      r.setTextContent(span, task)
      r.appendChild(li, span)
      r.appendChild(taskList, li)
    r.appendChild(app, taskList)

    let footer = r.createElement("footer")
    let countSpan = r.createElement("span")
    r.setTextContent(countSpan, "3 tasks")
    r.appendChild(footer, countSpan)
    r.appendChild(app, footer)

    # Verify the render plan
    let plan = getPlan(app)
    check plan["kind"].getStr == "Rect"
    check plan["children"].len == 4  # header, inputArea, taskList, footer

    # header → Rect (mapped from "header")
    check plan["children"][0]["kind"].getStr == "Rect"
    # h1 → Label (mapped from "h1")
    check plan["children"][0]["children"][0]["kind"].getStr == "Label"

    # input area → Rect
    check plan["children"][1]["kind"].getStr == "Rect"
    # add button → Rect with click handler
    check plan["children"][1]["children"][0]["has_click_handler"].getBool == true

    # task list (ul) → Rect with 3 children
    let taskListPlan = plan["children"][2]
    check taskListPlan["kind"].getStr == "Rect"
    check taskListPlan["children"].len == 3

    # Each li → Rect, each span → Label
    for i in 0 ..< 3:
      check taskListPlan["children"][i]["kind"].getStr == "Rect"
      check taskListPlan["children"][i]["children"][0]["kind"].getStr == "Label"

    # footer → Rect, span → Label
    check plan["children"][3]["kind"].getStr == "Rect"
    check plan["children"][3]["children"][0]["kind"].getStr == "Label"

    # Total element count
    check renderPlanElementCount(app) > 10
    check verifyRenderPlan(app)

  test "task manager render plan updates after adding task":
    ## Verify that adding a task to the tree is reflected in the render plan.
    let r = FreyaRenderer()

    let taskList = r.createElement("ul")
    let li1 = r.createElement("li")
    let span1 = r.createElement("span")
    r.setTextContent(span1, "Task 1")
    r.appendChild(li1, span1)
    r.appendChild(taskList, li1)

    var plan = getPlan(taskList)
    check plan["children"].len == 1

    # Add another task
    let li2 = r.createElement("li")
    let span2 = r.createElement("span")
    r.setTextContent(span2, "Task 2")
    r.appendChild(li2, span2)
    r.appendChild(taskList, li2)

    plan = getPlan(taskList)
    check plan["children"].len == 2

  test "render plan with styled task items":
    ## Verify styles propagate through task items in the render plan.
    let r = FreyaRenderer()

    let li = r.createElement("li")
    r.setStyle(li, "background-color", "#f5f5f5")
    r.setStyle(li, "padding", "8")
    r.setStyle(li, "border-radius", "4")

    let span = r.createElement("span")
    r.setStyle(span, "color", "#333")
    r.setStyle(span, "font-size", "14")
    r.setTextContent(span, "Styled task")
    r.appendChild(li, span)

    let plan = getPlan(li)
    check plan["styles"]["background"].getStr == "#f5f5f5"
    check plan["styles"]["padding"].getStr == "8"
    check plan["styles"]["corner_radius"].getStr == "4"

    let spanPlan = plan["children"][0]
    check spanPlan["styles"]["color"].getStr == "#333"
    check spanPlan["styles"]["font_size"].getStr == "14"

suite "Render Plan - Reactive Updates":
  setup:
    freya_reset_tree()
    resetCallbacks()

  test "render plan reflects signal-driven text change":
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var text = createSignal("initial")
      let label = r.createTextNode("")

      createRenderEffect proc() =
        r.setTextContent(label, text.val)

      var plan = getPlan(label)
      check plan["text"].getStr == "initial"

      text.val = "updated"
      plan = getPlan(label)
      check plan["text"].getStr == "updated"

      text.val = "final"
      plan = getPlan(label)
      check plan["text"].getStr == "final"

      dispose()

  test "render plan reflects dynamic child addition":
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var items = createSignal(newSeq[string]())
      let container = r.createElement("div")

      proc rebuildList() =
        # Clear children (simplified — in real code use reconciliation)
        while childCount(container) > 0:
          let child = r.firstChild(container)
          if child != nil:
            r.removeChild(container, child)
          else:
            break
        for item in items.val:
          let span = r.createElement("span")
          r.setTextContent(span, item)
          r.appendChild(container, span)

      createRenderEffect proc() =
        rebuildList()

      var plan = getPlan(container)
      check plan["children"].len == 0

      items.val = @["alpha", "beta"]
      plan = getPlan(container)
      check plan["children"].len == 2

      items.val = @["alpha", "beta", "gamma"]
      plan = getPlan(container)
      check plan["children"].len == 3

      dispose()

  test "render plan reflects style change via signal":
    createRoot proc(dispose: proc()) =
      let r = FreyaRenderer()
      var bg = createSignal("red")
      let el = r.createElement("div")

      createRenderEffect proc() =
        r.setStyle(el, "background-color", bg.val)

      var plan = getPlan(el)
      check plan["styles"]["background"].getStr == "red"

      bg.val = "blue"
      plan = getPlan(el)
      check plan["styles"]["background"].getStr == "blue"

      dispose()
