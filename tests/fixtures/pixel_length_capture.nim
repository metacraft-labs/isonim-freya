## Exercise public FreyaRenderer.setStyle through the real default Rust shim.
## Shadow transport is the real owner FFI boundary; native parser acceptance is
## tested separately on these captured payloads, never inferred from this plan.
import std/[json, os, math]
import isonim_freya/[renderer, bindings]

var rows = newJArray()
let r = FreyaRenderer()
proc capture(name, prop, input, target, expected, parser, native: string;
             pair = ""; overwritePadding = false) =
  let node = r.createElement(if parser == "font": "span" else: "div")
  if overwritePadding: r.setStyle(node, "padding-top", "1px")
  r.setStyle(node, prop, input)
  let memory = freya_render_plan_json(node)
  doAssert memory != nil, "actual Rust shim produced no render plan"
  var plan: JsonNode
  try: plan = parseJson($cast[cstring](memory))
  finally: freya_free_string(memory)
  let present = plan["styles"].hasKey(target)
  let value = if present: plan["styles"][target].getStr else: ""
  rows.add %*{"name":name, "prop":prop, "input":input, "target":target,
    "expected":expected, "actual":value, "present":present,
    "expectedPresent":target != "unsupported", "parser":parser,
    "native":native, "pair":pair, "plan":plan}

proc pair(name, prop, value, target, parser: string) =
  capture(name & "-bare", prop, value, target, value, parser, "finite")
  capture(name, prop, value & "px", target, value, parser, "finite", name & "-bare")

for value in ["16", "24", "14", "20"]:
  pair("font-" & value, "font-size", value, "font_size", "font")
pair("font-native-alias", "font_size", "16", "font_size", "font")
for value in ["18", "24", "28", "48", "130", "300"]:
  pair("width-" & value, "width", value, "width", "size")
  pair("height-" & value, "height", value, "height", "size")
let textWidth = max("Completed".len * 9, 30)
pair("dynamic-filter-width", "width", $textWidth, "width", "size")
for prop in ["padding", "padding-top", "padding-bottom", "padding-left", "padding-right"]:
  pair("alias-" & prop, prop, "12", "padding", "gaps")
for prop in ["border-radius", "corner-radius", "border_radius", "corner_radius"]:
  pair("alias-" & prop, prop, "8", "corner_radius", "radius")
capture("padding-side-keeps-shared-field", "padding-right", "2px", "padding", "2", "gaps", "finite", overwritePadding=true)
for value in ["+.5", "-0", "+016.0", "1.6e1", "2E+1"]:
  capture("numeric-bare-" & value, "font-size", value, "font_size", value, "font", "finite")
  capture("numeric-px-" & value, "font-size", value & "px", "font_size", value, "font", "finite", "numeric-bare-" & value)
capture("unit-case", "font-size", "16PX", "font_size", "16", "font", "finite", "font-16-bare")
for value in ["px", "+px", ".px", "1.px", "1.e2px", "1e+px", "1pxpx", "1px 2px", " 16px", "16px ", "16em", "NaNpx", "infpx", "-Infpx", "1e39px", "1e309px"]:
  capture("preserved-" & value, "font-size", value, "font_size", value, "font", "reject")
for value in ["NaN", "inf", "1e39"]:
  capture("bare-nonfinite-" & value, "font-size", value, "font_size", value, "font", "nonfinite")
for value in ["0", "50%", "auto", "fill", "flex(2)", "calc(50% - 2)"]:
  capture("native-size-" & value, "width", value, "width", value, "size", "accept")
for item in [("min-width", "min_width"), ("max-width", "max_width"), ("min-height", "min_height"), ("max-height", "max_height"), ("margin", "margin"), ("gap", "spacing"), ("line-height", "line_height")]:
  capture("excluded-" & item[0], item[0], "12px", item[1], "12px", "transport-only", "transport-only")
for item in [("background-color", "background", "#123456", "#123456"), ("color", "color", "red", "red"), ("flex-direction", "direction", "row", "horizontal"), ("align-items", "cross_align", "flex-start", "start"), ("justify-content", "main_align", "flex-end", "end"), ("opacity", "opacity", "0.5", "0.5")]:
  capture("nonlength-" & item[0], item[0], item[2], item[1], item[3], "transport-only", "transport-only")
for prop in ["border-width", "position", "display", "flex-grow"]:
  capture("unsupported-preserved-" & prop, prop, "1", "unsupported", "", "transport-only", "transport-only")
let output = %*{"rows":rows, "diagnostics":{"float32High": $high(float32),
  "float32HighFinite":classify(high(float32)) notin {fcNan, fcInf, fcNegInf}}}
doAssert paramCount() == 1, "output path required"
writeFile(paramStr(1), $output & "\n")
echo "captured ", rows.len, " actual public FFI plan payloads"
