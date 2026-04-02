//! Render-sync bridge: translates the shadow element tree into Freya's declarative model.
//!
//! This module is the core of the imperative→declarative bridge. It walks the
//! shadow tree maintained by the C FFI layer and produces Freya elements via
//! Dioxus RSX for each node.
//!
//! ## Mapping
//!
//! | Shadow tag      | Freya element |
//! |-----------------|---------------|
//! | `div`, `rect`   | `rect`        |
//! | `span`, `label` | `label`       |
//! | `p`, `paragraph`| `paragraph`   |
//! | `button`        | `rect` (with click handler) |
//! | `img`, `image`  | `image`       |
//! | `svg`           | `svg`         |
//! | `root`          | `rect` (full-size container) |
//! | text node       | `label` with the text content |
//! | unknown tag     | `rect` (fallback container) |
//!
//! ## Style mapping
//!
//! Shadow style properties use CSS-like names. This module maps them to the
//! corresponding Freya element attributes. For example:
//! - `background` / `background-color` → `background`
//! - `width` / `height` → `width` / `height`
//! - `padding` / `margin` → `padding` / `margin`
//! - `color` → `color` (on text elements)
//! - `font-size` → `font_size`
//! - `border-radius` / `corner-radius` → `corner_radius`

use crate::tree::{Node, NodeId, NodeKind, Tree};

/// The Freya element type that a shadow node maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreyaElementKind {
    /// Container element (`rect` in Freya).
    Rect,
    /// Single-line text (`label` in Freya).
    Label,
    /// Rich text container (`paragraph` in Freya).
    Paragraph,
    /// Image element (`image` in Freya).
    Image,
    /// SVG element (`svg` in Freya).
    Svg,
}

/// Determines what Freya element a shadow node should map to.
pub fn classify_node(node: &Node) -> FreyaElementKind {
    match &node.kind {
        NodeKind::Text(_) => FreyaElementKind::Label,
        NodeKind::Element(tag) => classify_tag(tag),
    }
}

/// Maps a shadow tag name to a Freya element kind.
pub fn classify_tag(tag: &str) -> FreyaElementKind {
    match tag {
        "div" | "rect" | "root" | "button" | "section" | "header" | "footer" | "nav" | "main"
        | "article" | "aside" | "form" | "fieldset" => FreyaElementKind::Rect,
        "span" | "label" | "a" | "strong" | "em" | "code" | "h1" | "h2" | "h3" | "h4" | "h5"
        | "h6" => FreyaElementKind::Label,
        "p" | "paragraph" | "pre" | "blockquote" => FreyaElementKind::Paragraph,
        "img" | "image" => FreyaElementKind::Image,
        "svg" => FreyaElementKind::Svg,
        // Unknown tags default to rect (container)
        _ => FreyaElementKind::Rect,
    }
}

/// A collected style ready to be applied to a Freya element.
/// Freya uses its own attribute names; this struct normalizes CSS-like
/// property names to Freya equivalents.
#[derive(Debug, Clone, Default)]
pub struct FreyaStyles {
    pub width: Option<String>,
    pub height: Option<String>,
    pub min_width: Option<String>,
    pub min_height: Option<String>,
    pub max_width: Option<String>,
    pub max_height: Option<String>,
    pub padding: Option<String>,
    pub margin: Option<String>,
    pub background: Option<String>,
    pub color: Option<String>,
    pub font_size: Option<String>,
    pub font_family: Option<String>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub corner_radius: Option<String>,
    pub border: Option<String>,
    pub shadow: Option<String>,
    pub direction: Option<String>,
    pub main_align: Option<String>,
    pub cross_align: Option<String>,
    pub overflow: Option<String>,
    pub opacity: Option<String>,
    pub spacing: Option<String>,
    pub text_align: Option<String>,
    pub line_height: Option<String>,
    pub letter_spacing: Option<String>,
}

impl FreyaStyles {
    /// Collect styles from a shadow node, normalizing CSS-like names to Freya names.
    pub fn from_node(node: &Node) -> Self {
        let mut s = FreyaStyles::default();
        for (key, value) in &node.styles {
            s.apply(key, value);
        }
        // Also check attributes that might be styling (e.g. width/height set as attributes)
        for (key, value) in &node.attributes {
            // Skip internal attributes
            if key.starts_with("__") {
                continue;
            }
            // Only apply attribute-based styles if no explicit style was set
            s.apply_if_absent(key, value);
        }
        s
    }

    /// Apply a single CSS-like property.
    fn apply(&mut self, key: &str, value: &str) {
        let v = value.to_string();
        match normalize_property_name(key) {
            "width" => self.width = Some(v),
            "height" => self.height = Some(v),
            "min_width" | "min-width" => self.min_width = Some(v),
            "min_height" | "min-height" => self.min_height = Some(v),
            "max_width" | "max-width" => self.max_width = Some(v),
            "max_height" | "max-height" => self.max_height = Some(v),
            "padding" => self.padding = Some(v),
            "margin" => self.margin = Some(v),
            "background" | "background_color" | "background-color" => self.background = Some(v),
            "color" => self.color = Some(v),
            "font_size" | "font-size" => self.font_size = Some(v),
            "font_family" | "font-family" => self.font_family = Some(v),
            "font_weight" | "font-weight" => self.font_weight = Some(v),
            "font_style" | "font-style" => self.font_style = Some(v),
            "corner_radius" | "corner-radius" | "border_radius" | "border-radius" => {
                self.corner_radius = Some(v)
            }
            "border" => self.border = Some(v),
            "shadow" | "box_shadow" | "box-shadow" => self.shadow = Some(v),
            "direction" | "flex_direction" | "flex-direction" => {
                self.direction = Some(normalize_direction(value))
            }
            "main_align" | "justify_content" | "justify-content" => self.main_align = Some(v),
            "cross_align" | "align_items" | "align-items" => self.cross_align = Some(v),
            "overflow" => self.overflow = Some(v),
            "opacity" => self.opacity = Some(v),
            "spacing" | "gap" => self.spacing = Some(v),
            "text_align" | "text-align" => self.text_align = Some(v),
            "line_height" | "line-height" => self.line_height = Some(v),
            "letter_spacing" | "letter-spacing" => self.letter_spacing = Some(v),
            _ => {} // Ignore unknown properties
        }
    }

    /// Apply a property only if the corresponding field is None.
    fn apply_if_absent(&mut self, key: &str, value: &str) {
        let normalized = normalize_property_name(key);
        let already_set = match normalized {
            "width" => self.width.is_some(),
            "height" => self.height.is_some(),
            "padding" => self.padding.is_some(),
            "margin" => self.margin.is_some(),
            "background" | "background_color" | "background-color" => self.background.is_some(),
            "color" => self.color.is_some(),
            _ => false,
        };
        if !already_set {
            self.apply(key, value);
        }
    }
}

/// Normalize a CSS-like property name. This is a pass-through for now;
/// the matching in `apply()` handles both kebab-case and snake_case.
fn normalize_property_name(name: &str) -> &str {
    name
}

/// Normalize CSS direction values to Freya direction values.
fn normalize_direction(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "row" | "horizontal" => "horizontal".to_string(),
        "column" | "vertical" => "vertical".to_string(),
        other => other.to_string(),
    }
}

/// Describes a single node in the render plan — an intermediate representation
/// between the shadow tree and Freya RSX. This allows testing the mapping
/// logic without requiring actual Freya rendering.
#[derive(Debug, Clone)]
pub struct RenderNode {
    /// The shadow node ID this render node corresponds to.
    pub node_id: NodeId,
    /// What kind of Freya element to produce.
    pub element_kind: FreyaElementKind,
    /// Collected and normalized styles.
    pub styles: FreyaStyles,
    /// Text content (for text nodes or labels).
    pub text: Option<String>,
    /// Whether this node has a "click" event listener.
    pub has_click_handler: bool,
    /// Whether this node has an "input" event listener (keyboard input).
    pub has_input_handler: bool,
    /// All event listener names attached to this node (for introspection/testing).
    pub event_names: Vec<String>,
    /// Relevant attributes from the shadow node (e.g. src, alt for images).
    pub attributes: std::collections::HashMap<String, String>,
    /// Children render nodes (recursive).
    pub children: Vec<RenderNode>,
}

/// Build a render plan from the shadow tree, starting at `root_id`.
///
/// This walks the tree recursively and produces a `RenderNode` hierarchy
/// that describes what Freya elements to create. This intermediate
/// representation can be tested without Freya dependencies.
pub fn build_render_plan(tree: &Tree, root_id: NodeId) -> Option<RenderNode> {
    let node = tree.get(root_id)?;
    let element_kind = classify_node(node);
    let styles = FreyaStyles::from_node(node);

    let text = match &node.kind {
        NodeKind::Text(t) => Some(t.clone()),
        NodeKind::Element(_) => node.attributes.get("__text_content").cloned(),
    };

    let has_click_handler = node.event_listeners.contains_key("click");
    let has_input_handler = node.event_listeners.contains_key("input");
    let event_names: Vec<String> = node.event_listeners.keys().cloned().collect();

    // Preserve relevant attributes for rendering (e.g. src, alt for images, svg content).
    // Filter out internal attributes (prefixed with "__").
    let attributes: std::collections::HashMap<String, String> = node
        .attributes
        .iter()
        .filter(|(k, _)| !k.starts_with("__"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let children: Vec<RenderNode> = node
        .children
        .iter()
        .filter_map(|&child_id| build_render_plan(tree, child_id))
        .collect();

    Some(RenderNode {
        node_id: root_id,
        element_kind,
        styles,
        text,
        has_click_handler,
        has_input_handler,
        event_names,
        attributes,
        children,
    })
}

/// Count total render nodes in a plan (for testing).
pub fn count_render_nodes(plan: &RenderNode) -> usize {
    1 + plan
        .children
        .iter()
        .map(count_render_nodes)
        .sum::<usize>()
}

// ---------------------------------------------------------------------------
// Freya RSX rendering (only available with freya-backend feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "freya-backend")]
pub mod freya_render {
    //! Actual Freya element rendering from the shadow tree.
    //!
    //! This module contains the Dioxus component that reads the global shadow
    //! tree and produces Freya elements via RSX on each render cycle.

    use super::*;
    use crate::tree::Tree;
    use crate::window;

    use freya::prelude::*;

    /// The root Dioxus/Freya component. On each render, it:
    /// 1. Reads the global shadow tree
    /// 2. Builds a render plan
    /// 3. Produces Freya RSX from the plan
    ///
    /// It uses a signal to track re-render requests triggered by shadow tree mutations.
    pub fn shadow_tree_app() -> Element {
        // Use a signal to trigger re-renders when the shadow tree changes.
        // We poll the REPAINT_REQUESTED flag and update this counter to
        // force Dioxus to re-render.
        let mut render_gen = use_signal(|| 0u64);

        // Set up a periodic check for repaint requests.
        // In a real integration this would use platform events, but polling
        // is the simplest approach that works across all platforms.
        use_effect(move || {
            spawn(async move {
                loop {
                    // Check every 16ms (~60fps)
                    tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                    if window::take_repaint_request() {
                        render_gen += 1;
                    }
                }
            });
        });

        // Read the generation to establish a dependency so Dioxus re-renders
        // when it changes.
        let _gen = render_gen.read();

        // Lock the tree and build the render plan
        let plan = {
            let tree = crate::lock_tree();
            // Find the root node — it's the node with tag "root"
            find_root_and_build_plan(&tree)
        };

        match plan {
            Some(plan) => render_node_to_element(&plan),
            None => rsx! {
                rect {
                    width: "100%",
                    height: "100%",
                    label { "No shadow tree root found" }
                }
            },
        }
    }

    /// Find the root node in the tree and build a render plan from it.
    fn find_root_and_build_plan(tree: &Tree) -> Option<RenderNode> {
        // The root node is created by freya_launch() with tag "root".
        // We need to find it. Since NodeId values are sequential starting from 1,
        // and the root is typically the first node created, we scan for it.
        // In practice, we store the root ID globally.
        let root_id = *crate::ROOT_NODE_ID.lock().unwrap_or_else(|p| p.into_inner());
        if root_id.is_null() {
            return None;
        }
        build_render_plan(tree, root_id)
    }

    /// Recursively render a RenderNode to Freya RSX elements.
    fn render_node_to_element(plan: &RenderNode) -> Element {
        match plan.element_kind {
            FreyaElementKind::Rect => render_rect(plan),
            FreyaElementKind::Label => render_label(plan),
            FreyaElementKind::Paragraph => render_paragraph(plan),
            FreyaElementKind::Image => render_image(plan),
            FreyaElementKind::Svg => render_svg(plan),
        }
    }

    /// Render a `rect` element with its children.
    fn render_rect(plan: &RenderNode) -> Element {
        let s = &plan.styles;
        let children_elements: Vec<Element> = plan
            .children
            .iter()
            .map(render_node_to_element)
            .collect();

        // Build the rect with available styles.
        // Freya RSX attributes are set as string literals.
        let width = s.width.clone().unwrap_or_else(|| "auto".to_string());
        let height = s.height.clone().unwrap_or_else(|| "auto".to_string());
        let background = s.background.clone().unwrap_or_else(|| "transparent".to_string());
        let padding = s.padding.clone().unwrap_or_else(|| "0".to_string());
        let margin = s.margin.clone().unwrap_or_else(|| "0".to_string());
        let corner_radius = s.corner_radius.clone().unwrap_or_else(|| "0".to_string());
        let direction = s.direction.clone().unwrap_or_else(|| "vertical".to_string());
        let main_align = s.main_align.clone().unwrap_or_else(|| "start".to_string());
        let cross_align = s.cross_align.clone().unwrap_or_else(|| "start".to_string());
        let overflow = s.overflow.clone().unwrap_or_else(|| "clip".to_string());
        let color = s.color.clone().unwrap_or_else(|| "rgb(0, 0, 0)".to_string());
        let font_size = s.font_size.clone().unwrap_or_else(|| "16".to_string());

        // Wire up event handlers if the shadow node has them
        let node_id = plan.node_id;
        let has_click = plan.has_click_handler;
        let has_input = plan.has_input_handler;

        rsx! {
            rect {
                width: "{width}",
                height: "{height}",
                background: "{background}",
                padding: "{padding}",
                margin: "{margin}",
                corner_radius: "{corner_radius}",
                direction: "{direction}",
                main_align: "{main_align}",
                cross_align: "{cross_align}",
                overflow: "{overflow}",
                color: "{color}",
                font_size: "{font_size}",
                onclick: move |_| {
                    if has_click {
                        dispatch_shadow_event(node_id, "click");
                    }
                },
                onkeydown: move |_| {
                    if has_input {
                        dispatch_shadow_event(node_id, "input");
                    }
                },
                for (_i, child_el) in children_elements.into_iter().enumerate() {
                    {child_el}
                }
            }
        }
    }

    /// Render a `label` element (simple text).
    fn render_label(plan: &RenderNode) -> Element {
        let text = plan.text.clone().unwrap_or_default();
        let s = &plan.styles;
        let color = s.color.clone().unwrap_or_else(|| "rgb(0, 0, 0)".to_string());
        let font_size = s.font_size.clone().unwrap_or_else(|| "16".to_string());
        let font_weight = s.font_weight.clone().unwrap_or_else(|| "normal".to_string());

        // If this label has children (e.g. a span wrapping text nodes),
        // concatenate their text.
        let full_text = if plan.children.is_empty() {
            text
        } else {
            let mut combined = text;
            for child in &plan.children {
                if let Some(t) = &child.text {
                    combined.push_str(t);
                }
            }
            combined
        };

        rsx! {
            label {
                color: "{color}",
                font_size: "{font_size}",
                font_weight: "{font_weight}",
                "{full_text}"
            }
        }
    }

    /// Render a `paragraph` element (rich text container).
    fn render_paragraph(plan: &RenderNode) -> Element {
        let s = &plan.styles;
        let color = s.color.clone().unwrap_or_else(|| "rgb(0, 0, 0)".to_string());
        let font_size = s.font_size.clone().unwrap_or_else(|| "16".to_string());
        let line_height = s.line_height.clone().unwrap_or_else(|| "1.2".to_string());
        let text_align = s.text_align.clone().unwrap_or_else(|| "start".to_string());

        let text = plan.text.clone().unwrap_or_default();
        let child_texts: Vec<String> = plan
            .children
            .iter()
            .filter_map(|c| c.text.clone())
            .collect();

        rsx! {
            paragraph {
                color: "{color}",
                font_size: "{font_size}",
                line_height: "{line_height}",
                text_align: "{text_align}",
                if !text.is_empty() {
                    text { "{text}" }
                }
                for child_text in child_texts {
                    text { "{child_text}" }
                }
            }
        }
    }

    /// Render an `image` element.
    ///
    /// Freya's native `image` element requires pre-loaded byte data, not a URL.
    /// Until async image loading is implemented, we render a placeholder rect
    /// that preserves the src and alt attributes as a label so the content is
    /// visible during development and testing.
    fn render_image(plan: &RenderNode) -> Element {
        let s = &plan.styles;
        let width = s.width.clone().unwrap_or_else(|| "auto".to_string());
        let height = s.height.clone().unwrap_or_else(|| "auto".to_string());
        let background = s.background.clone().unwrap_or_else(|| "rgb(230, 230, 230)".to_string());

        let src = plan.attributes.get("src").cloned().unwrap_or_default();
        let alt = plan.attributes.get("alt").cloned().unwrap_or_else(|| "[image]".to_string());
        // Show alt text, falling back to src if alt is empty
        let display_text = if alt.is_empty() && !src.is_empty() {
            format!("[img: {}]", src)
        } else {
            alt
        };

        rsx! {
            rect {
                width: "{width}",
                height: "{height}",
                background: "{background}",
                main_align: "center",
                cross_align: "center",
                label { "{display_text}" }
            }
        }
    }

    /// Render an `svg` element.
    ///
    /// Freya's native `svg` element requires inline SVG data as a string.
    /// We look for SVG content in the node's text content or the "data" / "src"
    /// attributes.  If valid SVG data is found, we pass it through to Freya's
    /// `svg` element; otherwise we render a placeholder rect.
    fn render_svg(plan: &RenderNode) -> Element {
        let s = &plan.styles;
        let width = s.width.clone().unwrap_or_else(|| "auto".to_string());
        let height = s.height.clone().unwrap_or_else(|| "auto".to_string());
        let background = s.background.clone().unwrap_or_else(|| "rgb(240, 240, 240)".to_string());

        // Try to find SVG content from attributes or text content for display.
        // Freya's native svg element requires compile-time static byte data,
        // so dynamic SVG loading is deferred to a future milestone. For now,
        // render a placeholder rect that preserves the SVG source info.
        let svg_ref = plan.attributes.get("data")
            .or_else(|| plan.attributes.get("src"))
            .cloned()
            .or_else(|| plan.text.clone())
            .unwrap_or_default();

        let label_text = if svg_ref.is_empty() {
            "[svg]".to_string()
        } else if svg_ref.len() > 40 {
            // Likely inline SVG data — truncate for display
            "[svg: inline data]".to_string()
        } else {
            format!("[svg: {}]", svg_ref)
        };

        rsx! {
            rect {
                width: "{width}",
                height: "{height}",
                background: "{background}",
                main_align: "center",
                cross_align: "center",
                label { "{label_text}" }
            }
        }
    }

    /// Dispatch an event to the shadow tree's event listeners.
    /// This is called from Freya event handlers to bridge back to the
    /// Nim-side callbacks.
    ///
    /// The lock is released before calling callbacks to avoid deadlocks
    /// (callbacks may mutate the tree via FFI).
    pub fn dispatch_shadow_event(node_id: NodeId, event_name: &str) {
        let listeners: Vec<(extern "C" fn(), i32)> = {
            let tree = crate::lock_tree();
            if let Some(node) = tree.get(node_id) {
                node.event_listeners
                    .get(event_name)
                    .map(|ls| ls.iter().map(|l| (l.callback, l.callback_id)).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        // Read the dispatcher once (outside the tree lock)
        let dispatcher = {
            crate::EVENT_DISPATCHER
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone()
        };

        // Call callbacks outside the lock to avoid deadlocks
        for (cb, id) in listeners {
            if id > 0 {
                if let Some(dispatch) = dispatcher {
                    dispatch(id);
                }
            } else {
                cb();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (no Freya dependency required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{EventListener, Node, NodeId, Tree};

    #[test]
    fn test_classify_div() {
        assert_eq!(classify_tag("div"), FreyaElementKind::Rect);
    }

    #[test]
    fn test_classify_rect() {
        assert_eq!(classify_tag("rect"), FreyaElementKind::Rect);
    }

    #[test]
    fn test_classify_root() {
        assert_eq!(classify_tag("root"), FreyaElementKind::Rect);
    }

    #[test]
    fn test_classify_button() {
        assert_eq!(classify_tag("button"), FreyaElementKind::Rect);
    }

    #[test]
    fn test_classify_span() {
        assert_eq!(classify_tag("span"), FreyaElementKind::Label);
    }

    #[test]
    fn test_classify_label() {
        assert_eq!(classify_tag("label"), FreyaElementKind::Label);
    }

    #[test]
    fn test_classify_paragraph() {
        assert_eq!(classify_tag("paragraph"), FreyaElementKind::Paragraph);
    }

    #[test]
    fn test_classify_p() {
        assert_eq!(classify_tag("p"), FreyaElementKind::Paragraph);
    }

    #[test]
    fn test_classify_img() {
        assert_eq!(classify_tag("img"), FreyaElementKind::Image);
    }

    #[test]
    fn test_classify_image() {
        assert_eq!(classify_tag("image"), FreyaElementKind::Image);
    }

    #[test]
    fn test_classify_svg() {
        assert_eq!(classify_tag("svg"), FreyaElementKind::Svg);
    }

    #[test]
    fn test_classify_unknown_defaults_to_rect() {
        assert_eq!(classify_tag("custom-component"), FreyaElementKind::Rect);
    }

    #[test]
    fn test_classify_text_node() {
        let node = Node::new_text("hello");
        assert_eq!(classify_node(&node), FreyaElementKind::Label);
    }

    #[test]
    fn test_classify_element_node() {
        let node = Node::new_element("div");
        assert_eq!(classify_node(&node), FreyaElementKind::Rect);
    }

    #[test]
    fn test_styles_from_node_css_properties() {
        let mut node = Node::new_element("div");
        node.styles.insert("width".into(), "100%".into());
        node.styles.insert("height".into(), "50px".into());
        node.styles
            .insert("background-color".into(), "red".into());
        node.styles.insert("font-size".into(), "16".into());
        node.styles.insert("border-radius".into(), "8".into());

        let styles = FreyaStyles::from_node(&node);
        assert_eq!(styles.width.as_deref(), Some("100%"));
        assert_eq!(styles.height.as_deref(), Some("50px"));
        assert_eq!(styles.background.as_deref(), Some("red"));
        assert_eq!(styles.font_size.as_deref(), Some("16"));
        assert_eq!(styles.corner_radius.as_deref(), Some("8"));
    }

    #[test]
    fn test_styles_from_node_freya_properties() {
        let mut node = Node::new_element("rect");
        node.styles.insert("background".into(), "blue".into());
        node.styles.insert("corner_radius".into(), "4".into());
        node.styles.insert("direction".into(), "horizontal".into());
        node.styles.insert("main_align".into(), "center".into());
        node.styles.insert("cross_align".into(), "end".into());

        let styles = FreyaStyles::from_node(&node);
        assert_eq!(styles.background.as_deref(), Some("blue"));
        assert_eq!(styles.corner_radius.as_deref(), Some("4"));
        assert_eq!(styles.direction.as_deref(), Some("horizontal"));
        assert_eq!(styles.main_align.as_deref(), Some("center"));
        assert_eq!(styles.cross_align.as_deref(), Some("end"));
    }

    #[test]
    fn test_styles_direction_normalization() {
        let mut node = Node::new_element("div");
        node.styles.insert("direction".into(), "row".into());
        let styles = FreyaStyles::from_node(&node);
        assert_eq!(styles.direction.as_deref(), Some("horizontal"));

        let mut node2 = Node::new_element("div");
        node2.styles.insert("direction".into(), "column".into());
        let styles2 = FreyaStyles::from_node(&node2);
        assert_eq!(styles2.direction.as_deref(), Some("vertical"));
    }

    #[test]
    fn test_styles_attribute_fallback() {
        let mut node = Node::new_element("rect");
        node.attributes.insert("width".into(), "200".into());
        node.attributes.insert("height".into(), "100".into());
        // No styles set, so attributes should be used
        let styles = FreyaStyles::from_node(&node);
        assert_eq!(styles.width.as_deref(), Some("200"));
        assert_eq!(styles.height.as_deref(), Some("100"));
    }

    #[test]
    fn test_styles_explicit_style_overrides_attribute() {
        let mut node = Node::new_element("rect");
        node.styles.insert("width".into(), "300".into());
        node.attributes.insert("width".into(), "200".into());
        // Style should win
        let styles = FreyaStyles::from_node(&node);
        assert_eq!(styles.width.as_deref(), Some("300"));
    }

    #[test]
    fn test_styles_internal_attributes_ignored() {
        let mut node = Node::new_element("rect");
        node.attributes
            .insert("__text_content".into(), "hello".into());
        let styles = FreyaStyles::from_node(&node);
        // __text_content should not appear in any style field
        assert!(styles.width.is_none());
        assert!(styles.background.is_none());
    }

    #[test]
    fn test_build_render_plan_single_node() {
        let mut tree = Tree::new();
        let node = Node::new_element("div");
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert_eq!(plan.node_id, id);
        assert_eq!(plan.element_kind, FreyaElementKind::Rect);
        assert!(plan.children.is_empty());
        assert!(plan.text.is_none());
        assert!(!plan.has_click_handler);
    }

    #[test]
    fn test_build_render_plan_text_node() {
        let mut tree = Tree::new();
        let node = Node::new_text("hello world");
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert_eq!(plan.element_kind, FreyaElementKind::Label);
        assert_eq!(plan.text.as_deref(), Some("hello world"));
    }

    #[test]
    fn test_build_render_plan_with_children() {
        let mut tree = Tree::new();
        let root = Node::new_element("root");
        let child1 = Node::new_element("label");
        let child2 = Node::new_text("text");

        let root_id = tree.insert(root);
        let c1_id = tree.insert(child1);
        let c2_id = tree.insert(child2);

        tree.append_child(root_id, c1_id);
        tree.append_child(root_id, c2_id);

        let plan = build_render_plan(&tree, root_id).unwrap();
        assert_eq!(plan.children.len(), 2);
        assert_eq!(plan.children[0].element_kind, FreyaElementKind::Label);
        assert_eq!(plan.children[1].element_kind, FreyaElementKind::Label);
        assert_eq!(plan.children[1].text.as_deref(), Some("text"));
    }

    #[test]
    fn test_build_render_plan_with_styles() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("div");
        node.styles.insert("width".into(), "100%".into());
        node.styles.insert("background".into(), "red".into());
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert_eq!(plan.styles.width.as_deref(), Some("100%"));
        assert_eq!(plan.styles.background.as_deref(), Some("red"));
    }

    #[test]
    fn test_build_render_plan_with_click_handler() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("button");
        extern "C" fn noop() {}
        node.event_listeners
            .entry("click".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert!(plan.has_click_handler);
        assert_eq!(plan.element_kind, FreyaElementKind::Rect);
    }

    #[test]
    fn test_build_render_plan_nonexistent_node() {
        let tree = Tree::new();
        let plan = build_render_plan(&tree, NodeId(999));
        assert!(plan.is_none());
    }

    #[test]
    fn test_build_render_plan_deep_tree() {
        let mut tree = Tree::new();
        let root = Node::new_element("root");
        let div = Node::new_element("div");
        let span = Node::new_element("span");
        let text = Node::new_text("nested");

        let root_id = tree.insert(root);
        let div_id = tree.insert(div);
        let span_id = tree.insert(span);
        let text_id = tree.insert(text);

        tree.append_child(root_id, div_id);
        tree.append_child(div_id, span_id);
        tree.append_child(span_id, text_id);

        let plan = build_render_plan(&tree, root_id).unwrap();
        assert_eq!(count_render_nodes(&plan), 4);
        assert_eq!(plan.children.len(), 1); // div
        assert_eq!(plan.children[0].children.len(), 1); // span
        assert_eq!(plan.children[0].children[0].children.len(), 1); // text
        assert_eq!(
            plan.children[0].children[0].children[0].text.as_deref(),
            Some("nested")
        );
    }

    #[test]
    fn test_build_render_plan_element_with_text_content() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("div");
        node.set_text_content("direct text");
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert_eq!(plan.text.as_deref(), Some("direct text"));
    }

    #[test]
    fn test_count_render_nodes() {
        let plan = RenderNode {
            node_id: NodeId(1),
            element_kind: FreyaElementKind::Rect,
            styles: FreyaStyles::default(),
            text: None,
            has_click_handler: false,
            has_input_handler: false,
            event_names: vec![],
            attributes: std::collections::HashMap::new(),
            children: vec![
                RenderNode {
                    node_id: NodeId(2),
                    element_kind: FreyaElementKind::Label,
                    styles: FreyaStyles::default(),
                    text: Some("a".into()),
                    has_click_handler: false,
                    has_input_handler: false,
                    event_names: vec![],
                    attributes: std::collections::HashMap::new(),
                    children: vec![],
                },
                RenderNode {
                    node_id: NodeId(3),
                    element_kind: FreyaElementKind::Rect,
                    styles: FreyaStyles::default(),
                    text: None,
                    has_click_handler: false,
                    has_input_handler: false,
                    event_names: vec![],
                    attributes: std::collections::HashMap::new(),
                    children: vec![RenderNode {
                        node_id: NodeId(4),
                        element_kind: FreyaElementKind::Label,
                        styles: FreyaStyles::default(),
                        text: Some("b".into()),
                        has_click_handler: false,
                        has_input_handler: false,
                        event_names: vec![],
                        attributes: std::collections::HashMap::new(),
                        children: vec![],
                    }],
                },
            ],
        };
        assert_eq!(count_render_nodes(&plan), 4);
    }

    #[test]
    fn test_build_render_plan_with_input_handler() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("div");
        extern "C" fn noop() {}
        node.event_listeners
            .entry("input".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert!(plan.has_input_handler);
        assert!(!plan.has_click_handler);
        assert!(plan.event_names.contains(&"input".to_string()));
    }

    #[test]
    fn test_build_render_plan_multiple_event_handlers() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("button");
        extern "C" fn noop() {}
        node.event_listeners
            .entry("click".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        node.event_listeners
            .entry("input".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        node.event_listeners
            .entry("hover".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert!(plan.has_click_handler);
        assert!(plan.has_input_handler);
        assert_eq!(plan.event_names.len(), 3);
        assert!(plan.event_names.contains(&"click".to_string()));
        assert!(plan.event_names.contains(&"input".to_string()));
        assert!(plan.event_names.contains(&"hover".to_string()));
    }

    #[test]
    fn test_build_render_plan_no_handlers_by_default() {
        let mut tree = Tree::new();
        let node = Node::new_element("div");
        let id = tree.insert(node);

        let plan = build_render_plan(&tree, id).unwrap();
        assert!(!plan.has_click_handler);
        assert!(!plan.has_input_handler);
        assert!(plan.event_names.is_empty());
    }

    #[test]
    fn test_render_plan_preserves_event_handlers_in_children() {
        let mut tree = Tree::new();
        extern "C" fn noop() {}

        let root = Node::new_element("root");
        let root_id = tree.insert(root);

        let mut btn = Node::new_element("button");
        btn.event_listeners
            .entry("click".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        let btn_id = tree.insert(btn);

        let mut input = Node::new_element("div");
        input
            .event_listeners
            .entry("input".into())
            .or_default()
            .push(EventListener { callback: noop, callback_id: 0 });
        let input_id = tree.insert(input);

        tree.append_child(root_id, btn_id);
        tree.append_child(root_id, input_id);

        let plan = build_render_plan(&tree, root_id).unwrap();
        assert!(!plan.has_click_handler);
        assert!(!plan.has_input_handler);
        assert_eq!(plan.children.len(), 2);
        assert!(plan.children[0].has_click_handler);
        assert!(!plan.children[0].has_input_handler);
        assert!(!plan.children[1].has_click_handler);
        assert!(plan.children[1].has_input_handler);
    }

    #[test]
    fn test_repaint_flag_with_render_plan_rebuild() {
        // Verify that after mutating the tree and rebuilding the render plan,
        // the new plan reflects the changes.
        let mut tree = Tree::new();
        let root = Node::new_element("root");
        let root_id = tree.insert(root);

        let label = Node::new_element("label");
        let label_id = tree.insert(label);
        tree.append_child(root_id, label_id);

        // Build initial plan
        let plan1 = build_render_plan(&tree, root_id).unwrap();
        assert_eq!(plan1.children.len(), 1);

        // Mutate: add another child
        let label2 = Node::new_element("label");
        let label2_id = tree.insert(label2);
        tree.append_child(root_id, label2_id);

        // Rebuild plan — should now have 2 children
        let plan2 = build_render_plan(&tree, root_id).unwrap();
        assert_eq!(plan2.children.len(), 2);
    }

    #[test]
    fn test_html_semantic_tags_classify_correctly() {
        // HTML semantic container tags → Rect
        for tag in &[
            "section", "header", "footer", "nav", "main", "article", "aside", "form", "fieldset",
        ] {
            assert_eq!(
                classify_tag(tag),
                FreyaElementKind::Rect,
                "Expected {tag} to classify as Rect"
            );
        }
        // HTML inline/text tags → Label
        for tag in &[
            "a", "strong", "em", "code", "h1", "h2", "h3", "h4", "h5", "h6",
        ] {
            assert_eq!(
                classify_tag(tag),
                FreyaElementKind::Label,
                "Expected {tag} to classify as Label"
            );
        }
        // HTML block text tags → Paragraph
        for tag in &["pre", "blockquote"] {
            assert_eq!(
                classify_tag(tag),
                FreyaElementKind::Paragraph,
                "Expected {tag} to classify as Paragraph"
            );
        }
    }
}

/// Integration tests that exercise the full FFI → shadow tree → render plan pipeline.
///
/// Unlike the unit tests above (which construct Tree/Node directly), these tests
/// use the actual `extern "C"` FFI functions — the same functions that Nim calls.
/// They then build a render plan and verify the Freya element mapping is correct.
///
/// This is the key layer that was previously untested: the tests verify that
/// elements created through the C FFI produce correct render plans with proper
/// Freya element types, styles, and event handler wiring.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        freya_add_event_listener, freya_append_child, freya_create_element,
        freya_create_text_node, freya_destroy_element, freya_dispatch_event,
        freya_remove_child, freya_render_plan_element_count, freya_render_plan_json,
        freya_free_string, freya_reset_tree, freya_set_attribute, freya_set_style,
        freya_set_text_content, freya_verify_render_plan, lock_tree, render_plan_to_json,
        FreyaElement as FreyaHandle,
    };
    use serial_test::serial;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    /// Helper: get the render plan as a parsed JSON value via the FFI function.
    fn get_plan_json(root: *mut FreyaHandle) -> serde_json::Value {
        unsafe {
            let json_ptr = freya_render_plan_json(root);
            assert!(!json_ptr.is_null(), "render plan JSON should not be null");
            let json_str = CStr::from_ptr(json_ptr).to_str().unwrap().to_string();
            freya_free_string(json_ptr);
            serde_json::from_str(&json_str).expect("render plan should be valid JSON")
        }
    }

    /// Helper: get the render plan directly (Tree API, not FFI).
    fn get_plan_direct(root: *mut FreyaHandle) -> RenderNode {
        let node_id = crate::tree::NodeId(unsafe { (*root).node_id });
        let tree = lock_tree();
        build_render_plan(&tree, node_id).expect("render plan should exist")
    }

    // ===================================================================
    // Full pipeline: counter app
    // ===================================================================

    #[test]
    #[serial]
    fn test_full_pipeline_counter_app() {
        unsafe {
            freya_reset_tree();

            let root = freya_create_element(c("div").as_ptr());
            let label = freya_create_element(c("span").as_ptr());
            let text = freya_create_text_node(c("Count: 0").as_ptr());
            freya_append_child(label, text);
            freya_append_child(root, label);

            let button = freya_create_element(c("button").as_ptr());
            let btn_text = freya_create_text_node(c("+").as_ptr());
            freya_append_child(button, btn_text);

            extern "C" fn on_click() {}
            freya_add_event_listener(button, c("click").as_ptr(), on_click);
            freya_append_child(root, button);

            // Verify via direct render plan (Tree API)
            let plan = get_plan_direct(root);
            assert_eq!(plan.element_kind, FreyaElementKind::Rect); // div → Rect
            assert_eq!(plan.children.len(), 2);

            // span → Label
            assert_eq!(plan.children[0].element_kind, FreyaElementKind::Label);
            // text node → Label with text
            assert_eq!(plan.children[0].children[0].element_kind, FreyaElementKind::Label);
            assert_eq!(plan.children[0].children[0].text.as_deref(), Some("Count: 0"));

            // button → Rect with click handler
            assert_eq!(plan.children[1].element_kind, FreyaElementKind::Rect);
            assert!(plan.children[1].has_click_handler);
            assert_eq!(plan.children[1].children[0].text.as_deref(), Some("+"));

            // Verify via JSON FFI (the path Nim uses)
            let json = get_plan_json(root);
            assert_eq!(json["kind"], "Rect");
            assert_eq!(json["children"].as_array().unwrap().len(), 2);
            assert_eq!(json["children"][0]["kind"], "Label");
            assert_eq!(json["children"][0]["children"][0]["text"], "Count: 0");
            assert_eq!(json["children"][1]["kind"], "Rect");
            assert_eq!(json["children"][1]["has_click_handler"], true);

            // Element count
            assert_eq!(freya_render_plan_element_count(root), 5);
            assert_eq!(freya_verify_render_plan(root), 1);

            freya_destroy_element(root);
            freya_destroy_element(label);
            freya_destroy_element(text);
            freya_destroy_element(button);
            freya_destroy_element(btn_text);
        }
    }

    // ===================================================================
    // Style propagation through render plan
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_styles_via_ffi() {
        unsafe {
            freya_reset_tree();

            let div = freya_create_element(c("div").as_ptr());
            freya_set_style(div, c("background-color").as_ptr(), c("red").as_ptr());
            freya_set_style(div, c("width").as_ptr(), c("200px").as_ptr());
            freya_set_style(div, c("flex-direction").as_ptr(), c("row").as_ptr());
            freya_set_style(div, c("font-size").as_ptr(), c("16").as_ptr());
            freya_set_style(div, c("border-radius").as_ptr(), c("8").as_ptr());
            freya_set_style(div, c("padding").as_ptr(), c("10").as_ptr());

            let plan = get_plan_direct(div);
            assert_eq!(plan.styles.background.as_deref(), Some("red"));
            assert_eq!(plan.styles.width.as_deref(), Some("200px"));
            assert_eq!(plan.styles.direction.as_deref(), Some("horizontal"));
            assert_eq!(plan.styles.font_size.as_deref(), Some("16"));
            assert_eq!(plan.styles.corner_radius.as_deref(), Some("8"));
            assert_eq!(plan.styles.padding.as_deref(), Some("10"));

            // Also verify via JSON
            let json = get_plan_json(div);
            assert_eq!(json["styles"]["background"], "red");
            assert_eq!(json["styles"]["width"], "200px");
            assert_eq!(json["styles"]["direction"], "horizontal");

            freya_destroy_element(div);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_styles_column_direction() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("div").as_ptr());
            freya_set_style(div, c("flex-direction").as_ptr(), c("column").as_ptr());
            let plan = get_plan_direct(div);
            assert_eq!(plan.styles.direction.as_deref(), Some("vertical"));
            freya_destroy_element(div);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_styles_from_attributes_fallback() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("rect").as_ptr());
            freya_set_attribute(div, c("width").as_ptr(), c("300").as_ptr());
            freya_set_attribute(div, c("height").as_ptr(), c("200").as_ptr());
            let plan = get_plan_direct(div);
            assert_eq!(plan.styles.width.as_deref(), Some("300"));
            assert_eq!(plan.styles.height.as_deref(), Some("200"));
            freya_destroy_element(div);
        }
    }

    // ===================================================================
    // Event handler wiring through render plan
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_event_handlers_via_ffi() {
        unsafe {
            freya_reset_tree();

            static CLICKED: AtomicU32 = AtomicU32::new(0);
            extern "C" fn on_click() {
                CLICKED.fetch_add(1, Ordering::SeqCst);
            }
            CLICKED.store(0, Ordering::SeqCst);

            let button = freya_create_element(c("button").as_ptr());
            freya_add_event_listener(button, c("click").as_ptr(), on_click);

            let plan = get_plan_direct(button);
            assert!(plan.has_click_handler);
            assert!(!plan.has_input_handler);

            // Dispatch through the shadow tree — verify callback fires
            freya_dispatch_event(button, c("click").as_ptr());
            assert_eq!(CLICKED.load(Ordering::SeqCst), 1);

            freya_destroy_element(button);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_multiple_event_types_via_ffi() {
        unsafe {
            freya_reset_tree();
            extern "C" fn noop() {}

            let node = freya_create_element(c("div").as_ptr());
            freya_add_event_listener(node, c("click").as_ptr(), noop);
            freya_add_event_listener(node, c("input").as_ptr(), noop);
            freya_add_event_listener(node, c("hover").as_ptr(), noop);

            let plan = get_plan_direct(node);
            assert!(plan.has_click_handler);
            assert!(plan.has_input_handler);
            assert_eq!(plan.event_names.len(), 3);
            assert!(plan.event_names.contains(&"click".to_string()));
            assert!(plan.event_names.contains(&"input".to_string()));
            assert!(plan.event_names.contains(&"hover".to_string()));

            freya_destroy_element(node);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_no_handlers_by_default_via_ffi() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("div").as_ptr());
            let plan = get_plan_direct(div);
            assert!(!plan.has_click_handler);
            assert!(!plan.has_input_handler);
            assert!(plan.event_names.is_empty());
            freya_destroy_element(div);
        }
    }

    // ===================================================================
    // Full demo app render plan (task manager)
    // ===================================================================

    #[test]
    #[serial]
    fn test_full_demo_app_render_plan() {
        unsafe {
            freya_reset_tree();

            let app = freya_create_element(c("div").as_ptr());

            // Header
            let header = freya_create_element(c("header").as_ptr());
            let title = freya_create_element(c("h1").as_ptr());
            let title_text = freya_create_text_node(c("Task Manager").as_ptr());
            freya_append_child(title, title_text);
            freya_append_child(header, title);
            freya_append_child(app, header);

            // Input area
            let input_area = freya_create_element(c("div").as_ptr());
            let input = freya_create_element(c("input").as_ptr());
            let add_btn = freya_create_element(c("button").as_ptr());
            let add_text = freya_create_text_node(c("Add").as_ptr());
            extern "C" fn on_add() {}
            freya_add_event_listener(add_btn, c("click").as_ptr(), on_add);
            freya_append_child(add_btn, add_text);
            freya_append_child(input_area, input);
            freya_append_child(input_area, add_btn);
            freya_append_child(app, input_area);

            // Task list
            let task_list = freya_create_element(c("ul").as_ptr());
            let mut task_handles = Vec::new();
            for task_name in &["Design API", "Write tests"] {
                let li = freya_create_element(c("li").as_ptr());
                let span = freya_create_element(c("span").as_ptr());
                let text = freya_create_text_node(c(task_name).as_ptr());
                freya_append_child(span, text);
                freya_append_child(li, span);
                freya_append_child(task_list, li);
                task_handles.push((li, span, text));
            }
            freya_append_child(app, task_list);

            // Footer
            let footer = freya_create_element(c("footer").as_ptr());
            let count_span = freya_create_element(c("span").as_ptr());
            let count_text = freya_create_text_node(c("2 tasks").as_ptr());
            freya_append_child(count_span, count_text);
            freya_append_child(footer, count_span);
            freya_append_child(app, footer);

            // Verify render plan
            let plan = get_plan_direct(app);

            assert_eq!(plan.element_kind, FreyaElementKind::Rect);
            assert_eq!(plan.children.len(), 4);

            // header → Rect
            assert_eq!(plan.children[0].element_kind, FreyaElementKind::Rect);
            // h1 → Label
            assert_eq!(plan.children[0].children[0].element_kind, FreyaElementKind::Label);

            // input area → Rect with 2 children
            assert_eq!(plan.children[1].element_kind, FreyaElementKind::Rect);
            assert_eq!(plan.children[1].children.len(), 2);
            // add button → Rect with click handler
            assert!(plan.children[1].children[1].has_click_handler);

            // task list → Rect with 2 children
            assert_eq!(plan.children[2].element_kind, FreyaElementKind::Rect);
            assert_eq!(plan.children[2].children.len(), 2);

            // First task: li → Rect > span → Label > text
            assert_eq!(plan.children[2].children[0].element_kind, FreyaElementKind::Rect);
            assert_eq!(
                plan.children[2].children[0].children[0].element_kind,
                FreyaElementKind::Label
            );
            assert_eq!(
                plan.children[2].children[0].children[0].children[0]
                    .text
                    .as_deref(),
                Some("Design API")
            );

            // Second task
            assert_eq!(
                plan.children[2].children[1].children[0].children[0]
                    .text
                    .as_deref(),
                Some("Write tests")
            );

            // footer → Rect > span → Label
            assert_eq!(plan.children[3].element_kind, FreyaElementKind::Rect);
            assert_eq!(
                plan.children[3].children[0].element_kind,
                FreyaElementKind::Label
            );
            assert_eq!(
                plan.children[3].children[0].children[0].text.as_deref(),
                Some("2 tasks")
            );

            // Total element count
            assert_eq!(freya_render_plan_element_count(app), 18);

            // Clean up
            freya_destroy_element(app);
            freya_destroy_element(header);
            freya_destroy_element(title);
            freya_destroy_element(title_text);
            freya_destroy_element(input_area);
            freya_destroy_element(input);
            freya_destroy_element(add_btn);
            freya_destroy_element(add_text);
            freya_destroy_element(task_list);
            freya_destroy_element(footer);
            freya_destroy_element(count_span);
            freya_destroy_element(count_text);
            for (li, span, text) in task_handles {
                freya_destroy_element(li);
                freya_destroy_element(span);
                freya_destroy_element(text);
            }
        }
    }

    // ===================================================================
    // Render plan after mutations
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_after_adding_child() {
        unsafe {
            freya_reset_tree();
            let root = freya_create_element(c("div").as_ptr());
            let label1 = freya_create_element(c("span").as_ptr());
            freya_append_child(root, label1);

            let plan1 = get_plan_direct(root);
            assert_eq!(plan1.children.len(), 1);
            assert_eq!(freya_render_plan_element_count(root), 2);

            let label2 = freya_create_element(c("span").as_ptr());
            freya_append_child(root, label2);

            let plan2 = get_plan_direct(root);
            assert_eq!(plan2.children.len(), 2);
            assert_eq!(freya_render_plan_element_count(root), 3);

            freya_destroy_element(root);
            freya_destroy_element(label1);
            freya_destroy_element(label2);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_after_removing_child() {
        unsafe {
            freya_reset_tree();
            let root = freya_create_element(c("div").as_ptr());
            let c1 = freya_create_element(c("span").as_ptr());
            let c2 = freya_create_element(c("span").as_ptr());
            freya_append_child(root, c1);
            freya_append_child(root, c2);
            assert_eq!(freya_render_plan_element_count(root), 3);

            freya_remove_child(root, c1);
            let plan = get_plan_direct(root);
            assert_eq!(plan.children.len(), 1);
            assert_eq!(freya_render_plan_element_count(root), 2);

            freya_destroy_element(root);
            freya_destroy_element(c1);
            freya_destroy_element(c2);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_after_text_update() {
        unsafe {
            freya_reset_tree();
            let root = freya_create_element(c("div").as_ptr());
            let text_node = freya_create_text_node(c("initial").as_ptr());
            freya_append_child(root, text_node);

            let plan1 = get_plan_direct(root);
            assert_eq!(plan1.children[0].text.as_deref(), Some("initial"));

            freya_set_text_content(text_node, c("updated").as_ptr());

            let plan2 = get_plan_direct(root);
            assert_eq!(plan2.children[0].text.as_deref(), Some("updated"));

            freya_destroy_element(root);
            freya_destroy_element(text_node);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_after_style_change() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("div").as_ptr());
            freya_set_style(div, c("background").as_ptr(), c("red").as_ptr());

            let plan1 = get_plan_direct(div);
            assert_eq!(plan1.styles.background.as_deref(), Some("red"));

            freya_set_style(div, c("background").as_ptr(), c("blue").as_ptr());

            let plan2 = get_plan_direct(div);
            assert_eq!(plan2.styles.background.as_deref(), Some("blue"));

            freya_destroy_element(div);
        }
    }

    #[test]
    #[serial]
    fn test_render_plan_after_adding_event_handler() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("div").as_ptr());

            let plan1 = get_plan_direct(div);
            assert!(!plan1.has_click_handler);

            extern "C" fn noop() {}
            freya_add_event_listener(div, c("click").as_ptr(), noop);

            let plan2 = get_plan_direct(div);
            assert!(plan2.has_click_handler);

            freya_destroy_element(div);
        }
    }

    // ===================================================================
    // Tag classification through FFI → render plan
    // ===================================================================

    #[test]
    #[serial]
    fn test_tag_classification_through_ffi() {
        let test_cases: Vec<(&str, FreyaElementKind)> = vec![
            ("div", FreyaElementKind::Rect),
            ("rect", FreyaElementKind::Rect),
            ("section", FreyaElementKind::Rect),
            ("header", FreyaElementKind::Rect),
            ("footer", FreyaElementKind::Rect),
            ("button", FreyaElementKind::Rect),
            ("span", FreyaElementKind::Label),
            ("label", FreyaElementKind::Label),
            ("h1", FreyaElementKind::Label),
            ("a", FreyaElementKind::Label),
            ("strong", FreyaElementKind::Label),
            ("p", FreyaElementKind::Paragraph),
            ("paragraph", FreyaElementKind::Paragraph),
            ("pre", FreyaElementKind::Paragraph),
            ("img", FreyaElementKind::Image),
            ("image", FreyaElementKind::Image),
            ("svg", FreyaElementKind::Svg),
        ];

        for (tag, expected_kind) in test_cases {
            unsafe {
                freya_reset_tree();
                let elem = freya_create_element(c(tag).as_ptr());
                let plan = get_plan_direct(elem);
                assert_eq!(
                    plan.element_kind, expected_kind,
                    "Tag '{}' should classify as {:?}",
                    tag, expected_kind
                );
                freya_destroy_element(elem);
            }
        }
    }

    #[test]
    #[serial]
    fn test_text_node_classifies_as_label_via_ffi() {
        unsafe {
            freya_reset_tree();
            let text = freya_create_text_node(c("hello").as_ptr());
            let plan = get_plan_direct(text);
            assert_eq!(plan.element_kind, FreyaElementKind::Label);
            assert_eq!(plan.text.as_deref(), Some("hello"));
            freya_destroy_element(text);
        }
    }

    // ===================================================================
    // Null / edge cases
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_null_handle() {
        unsafe {
            let json = freya_render_plan_json(std::ptr::null_mut());
            assert!(json.is_null());
            assert_eq!(freya_render_plan_element_count(std::ptr::null_mut()), 0);
            assert_eq!(freya_verify_render_plan(std::ptr::null_mut()), 0);
        }
    }

    // ===================================================================
    // Child handlers preserved in plan
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_child_handlers_preserved() {
        unsafe {
            freya_reset_tree();
            extern "C" fn on_click() {}
            extern "C" fn on_input() {}

            let root = freya_create_element(c("div").as_ptr());
            let btn = freya_create_element(c("button").as_ptr());
            freya_add_event_listener(btn, c("click").as_ptr(), on_click);
            let input_div = freya_create_element(c("div").as_ptr());
            freya_add_event_listener(input_div, c("input").as_ptr(), on_input);

            freya_append_child(root, btn);
            freya_append_child(root, input_div);

            let plan = get_plan_direct(root);
            assert!(!plan.has_click_handler);
            assert!(!plan.has_input_handler);
            assert!(plan.children[0].has_click_handler);
            assert!(!plan.children[0].has_input_handler);
            assert!(!plan.children[1].has_click_handler);
            assert!(plan.children[1].has_input_handler);

            freya_destroy_element(root);
            freya_destroy_element(btn);
            freya_destroy_element(input_div);
        }
    }

    // ===================================================================
    // Deep tree render plan
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_deep_nested_tree_via_ffi() {
        unsafe {
            freya_reset_tree();
            let root = freya_create_element(c("root").as_ptr());
            let div = freya_create_element(c("div").as_ptr());
            let span = freya_create_element(c("span").as_ptr());
            let text = freya_create_text_node(c("deep").as_ptr());

            freya_append_child(span, text);
            freya_append_child(div, span);
            freya_append_child(root, div);

            let plan = get_plan_direct(root);
            assert_eq!(count_render_nodes(&plan), 4);
            assert_eq!(plan.children[0].children[0].children[0].text.as_deref(), Some("deep"));

            freya_destroy_element(root);
            freya_destroy_element(div);
            freya_destroy_element(span);
            freya_destroy_element(text);
        }
    }

    // ===================================================================
    // Event dispatch + plan rebuild (simulates reactive update)
    // ===================================================================

    #[test]
    #[serial]
    fn test_event_dispatch_then_rebuild_plan() {
        unsafe {
            freya_reset_tree();

            static COUNTER: AtomicU32 = AtomicU32::new(0);
            COUNTER.store(0, Ordering::SeqCst);
            extern "C" fn increment() {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }

            let root = freya_create_element(c("div").as_ptr());
            let btn = freya_create_element(c("button").as_ptr());
            let label = freya_create_text_node(c("Count: 0").as_ptr());
            freya_add_event_listener(btn, c("click").as_ptr(), increment);
            freya_append_child(root, label);
            freya_append_child(root, btn);

            // Initial plan
            let plan1 = get_plan_direct(root);
            assert_eq!(plan1.children[0].text.as_deref(), Some("Count: 0"));

            // Simulate click
            freya_dispatch_event(btn, c("click").as_ptr());
            assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

            // Simulate reactive update
            freya_set_text_content(label, c("Count: 1").as_ptr());

            // Rebuild plan — should reflect the mutation
            let plan2 = get_plan_direct(root);
            assert_eq!(plan2.children[0].text.as_deref(), Some("Count: 1"));

            freya_destroy_element(root);
            freya_destroy_element(btn);
            freya_destroy_element(label);
        }
    }

    // ===================================================================
    // JSON serialization round-trip
    // ===================================================================

    #[test]
    #[serial]
    fn test_render_plan_json_round_trip() {
        unsafe {
            freya_reset_tree();
            let div = freya_create_element(c("div").as_ptr());
            freya_set_style(div, c("background").as_ptr(), c("red").as_ptr());
            let span = freya_create_element(c("span").as_ptr());
            let text = freya_create_text_node(c("hello").as_ptr());
            freya_append_child(span, text);
            freya_append_child(div, span);

            extern "C" fn noop() {}
            freya_add_event_listener(div, c("click").as_ptr(), noop);

            let json = get_plan_json(div);

            // Verify JSON structure matches the direct plan
            assert_eq!(json["kind"], "Rect");
            assert_eq!(json["has_click_handler"], true);
            assert_eq!(json["styles"]["background"], "red");
            assert_eq!(json["children"][0]["kind"], "Label");
            assert_eq!(json["children"][0]["children"][0]["text"], "hello");

            freya_destroy_element(div);
            freya_destroy_element(span);
            freya_destroy_element(text);
        }
    }
}
