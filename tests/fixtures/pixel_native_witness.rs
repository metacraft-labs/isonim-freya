// Executed with original checksum-bound native parser modules assembled by the
// owner test driver. No replacement renderer/parser or full GUI/state claim.
use serde_json::{json, Value};
use std::{collections::HashMap, fs};
use parsing::Parse;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input: Value = serde_json::from_str(&fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut outputs = Vec::new();
    let mut values: HashMap<String, Vec<f32>> = HashMap::new();
    let mut failures = Vec::new();
    for row in input["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let actual = row["actual"].as_str().unwrap();
        let kind = row["parser"].as_str().unwrap();
        if kind == "transport-only" {
            outputs.push(json!({"name":name,"scope":"exact FFI preservation only; no native property support claim"}));
            continue;
        }
        let parsed: Option<Vec<f32>> = match kind {
            "font" => {
                let mut state = FontNumericArm {font_size:99.0};
                state.apply(actual);
                actual.parse::<f32>().ok().map(|_| vec![state.font_size])
            }
            "size" => torin::size::Size::parse(actual).ok().map(|size| match size {
                torin::size::Size::Pixels(value) | torin::size::Size::Percentage(value) |
                torin::size::Size::Flex(value) => vec![value.get()],
                _ => vec![],
            }),
            "gaps" => torin::gaps::Gaps::parse(actual).ok().map(|gaps| vec![gaps.top(),gaps.right(),gaps.bottom(),gaps.left()]),
            "radius" => radius_parser::CornerRadius::parse(actual).ok().map(|radius| vec![radius.top_left,radius.top_right,radius.bottom_right,radius.bottom_left]),
            _ => panic!("unknown required native parser"),
        };
        let accepted = parsed.is_some();
        let finite = parsed.as_ref().map(|v|v.iter().all(|n|n.is_finite())).unwrap_or(false);
        let expected = row["native"].as_str().unwrap();
        let ok = match expected {
            "finite" => accepted && finite,
            "accept" => accepted,
            "reject" => !accepted,
            "nonfinite" => accepted && !finite,
            _ => panic!("unknown native assertion"),
        };
        if !ok { failures.push(format!("native-compatibility:{name}")); }
        let pair = row["pair"].as_str().unwrap();
        if !pair.is_empty() && (parsed.is_none() || parsed.as_ref() != values.get(pair)) {
            failures.push(format!("native-companion:{name}"));
        }
        if let Some(ref numbers) = parsed { values.insert(name.to_owned(), numbers.clone()); }
        outputs.push(json!({"name":name,"payload":actual,"accepted":accepted,"finite":finite,"values":parsed,"ok":ok}));
    }
    fs::write(&args[2], serde_json::to_string_pretty(&json!({"outputs":outputs,"failures":failures})).unwrap()).unwrap();
    assert!(failures.is_empty(), "semantic: native-compatibility failed: {failures:?}");
    println!("PASS actual captured payloads in original native parsers");
}
