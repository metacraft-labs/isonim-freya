//! Shadow element tree for imperative DOM-like manipulation.
//!
//! Freya (built on Dioxus) is declarative — UI is expressed as RSX components
//! that re-render when reactive state changes. IsoNim's RendererBackend needs
//! imperative tree manipulation (createElement, appendChild, etc.).
//!
//! This module bridges the gap by maintaining a mutable tree of `Node` structs.
//! Each node has a tag, attributes, styles, text content, children, parent,
//! and event listeners. The extern "C" shim functions manipulate this tree,
//! and a separate render-sync step (M2+) will translate it into Freya's
//! declarative model.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Globally unique node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl NodeId {
    pub fn new() -> Self {
        NodeId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// The null/sentinel id (0), representing "no node".
    pub const NULL: NodeId = NodeId(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// The kind of a node in the shadow tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// An element node (e.g. "rect", "label", "paragraph").
    Element(String),
    /// A text node with content.
    Text(String),
}

/// A single node in the shadow tree.
#[derive(Debug)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Attributes set via setAttribute (name -> value).
    pub attributes: HashMap<String, String>,
    /// Style properties set via setStyle (property -> value).
    pub styles: HashMap<String, String>,
    /// Ordered list of child node IDs.
    pub children: Vec<NodeId>,
    /// Parent node ID (NULL if this is a root or detached).
    pub parent: NodeId,
    /// Event listeners keyed by event name.
    /// Each event can have multiple listeners.
    pub event_listeners: HashMap<String, Vec<EventListener>>,
}

/// An event listener stored in the shadow tree.
///
/// Supports two dispatch modes:
/// - **Legacy (function pointer):** `callback` is set, `callback_id` is 0.
///   The function pointer is called directly.
/// - **Dispatcher (callback ID):** `callback_id` > 0, dispatched via the
///   global event dispatcher registered by `freya_set_event_dispatcher`.
///   `callback` is set to a dummy no-op in this mode.
#[derive(Debug, Clone, Copy)]
pub struct EventListener {
    pub callback: extern "C" fn(),
    pub callback_id: i32,
}

impl Node {
    /// Create a new element node with the given tag.
    pub fn new_element(tag: &str) -> Self {
        Node {
            id: NodeId::new(),
            kind: NodeKind::Element(tag.to_string()),
            attributes: HashMap::new(),
            styles: HashMap::new(),
            children: Vec::new(),
            parent: NodeId::NULL,
            event_listeners: HashMap::new(),
        }
    }

    /// Create a new text node with the given content.
    pub fn new_text(text: &str) -> Self {
        Node {
            id: NodeId::new(),
            kind: NodeKind::Text(text.to_string()),
            attributes: HashMap::new(),
            styles: HashMap::new(),
            children: Vec::new(),
            parent: NodeId::NULL,
            event_listeners: HashMap::new(),
        }
    }

    /// Get the tag name, if this is an element node.
    pub fn tag(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::Element(tag) => Some(tag.as_str()),
            NodeKind::Text(_) => None,
        }
    }

    /// Get the text content.
    pub fn text_content(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::Text(text) => Some(text.as_str()),
            NodeKind::Element(_) => None,
        }
    }

    /// Set the text content (for text nodes, replaces text; for element nodes, no-op).
    pub fn set_text_content(&mut self, text: &str) {
        match &mut self.kind {
            NodeKind::Text(ref mut t) => *t = text.to_string(),
            NodeKind::Element(_) => {
                // For element nodes, setting text content could clear children
                // and set inner text, but for now we store it as an attribute.
                // This mirrors browser behavior where setting textContent on
                // an element replaces all children with a single text node.
                // We'll handle this properly in the render sync.
                self.attributes
                    .insert("__text_content".to_string(), text.to_string());
            }
        }
    }
}

/// The shadow tree: a flat store of nodes indexed by NodeId.
#[derive(Debug)]
pub struct Tree {
    nodes: HashMap<u64, Node>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            nodes: HashMap::new(),
        }
    }

    /// Insert a node into the tree. Returns its NodeId.
    pub fn insert(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.insert(id.0, node);
        id
    }

    /// Get a reference to a node by id.
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id.0)
    }

    /// Get a mutable reference to a node by id.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id.0)
    }

    /// Remove a node from the store (does NOT detach from parent/children).
    pub fn remove(&mut self, id: NodeId) -> Option<Node> {
        self.nodes.remove(&id.0)
    }

    /// Append `child_id` as the last child of `parent_id`.
    /// Detaches child from its current parent first if needed.
    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        // Detach from old parent
        self.detach(child_id);

        // Set new parent
        if let Some(child) = self.nodes.get_mut(&child_id.0) {
            child.parent = parent_id;
        }

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(&parent_id.0) {
            parent.children.push(child_id);
        }
    }

    /// Insert `child_id` before `ref_id` within `parent_id`.
    /// If `ref_id` is NULL or not found among parent's children, appends instead.
    pub fn insert_before(&mut self, parent_id: NodeId, child_id: NodeId, ref_id: NodeId) {
        if ref_id.is_null() {
            self.append_child(parent_id, child_id);
            return;
        }

        // Detach from old parent
        self.detach(child_id);

        // Set new parent
        if let Some(child) = self.nodes.get_mut(&child_id.0) {
            child.parent = parent_id;
        }

        // Insert before reference in parent's children
        if let Some(parent) = self.nodes.get_mut(&parent_id.0) {
            if let Some(pos) = parent.children.iter().position(|&c| c == ref_id) {
                parent.children.insert(pos, child_id);
            } else {
                // Reference not found, append
                parent.children.push(child_id);
            }
        }
    }

    /// Remove `child_id` from `parent_id`'s children list and clear parent ref.
    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        if let Some(parent) = self.nodes.get_mut(&parent_id.0) {
            parent.children.retain(|&c| c != child_id);
        }
        if let Some(child) = self.nodes.get_mut(&child_id.0) {
            child.parent = NodeId::NULL;
        }
    }

    /// Detach a node from its current parent (internal helper).
    fn detach(&mut self, child_id: NodeId) {
        let old_parent = self
            .nodes
            .get(&child_id.0)
            .map(|n| n.parent)
            .unwrap_or(NodeId::NULL);
        if !old_parent.is_null() {
            if let Some(parent) = self.nodes.get_mut(&old_parent.0) {
                parent.children.retain(|&c| c != child_id);
            }
        }
    }

    /// Get the first child of a node.
    pub fn first_child(&self, node_id: NodeId) -> NodeId {
        self.nodes
            .get(&node_id.0)
            .and_then(|n| n.children.first().copied())
            .unwrap_or(NodeId::NULL)
    }

    /// Get the next sibling of a node (the node after it in its parent's children list).
    pub fn next_sibling(&self, node_id: NodeId) -> NodeId {
        let parent_id = self
            .nodes
            .get(&node_id.0)
            .map(|n| n.parent)
            .unwrap_or(NodeId::NULL);
        if parent_id.is_null() {
            return NodeId::NULL;
        }
        if let Some(parent) = self.nodes.get(&parent_id.0) {
            if let Some(pos) = parent.children.iter().position(|&c| c == node_id) {
                if pos + 1 < parent.children.len() {
                    return parent.children[pos + 1];
                }
            }
        }
        NodeId::NULL
    }

    /// Get the parent of a node.
    pub fn parent_node(&self, node_id: NodeId) -> NodeId {
        self.nodes
            .get(&node_id.0)
            .map(|n| n.parent)
            .unwrap_or(NodeId::NULL)
    }

    /// Get the total number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate mutably over all nodes in the tree.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.nodes.values_mut()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_element() {
        let node = Node::new_element("rect");
        assert_eq!(node.tag(), Some("rect"));
        assert!(node.text_content().is_none());
        assert!(!node.id.is_null());
    }

    #[test]
    fn test_create_text() {
        let node = Node::new_text("hello");
        assert!(node.tag().is_none());
        assert_eq!(node.text_content(), Some("hello"));
    }

    #[test]
    fn test_append_child() {
        let mut tree = Tree::new();
        let parent = Node::new_element("rect");
        let child = Node::new_element("label");
        let pid = tree.insert(parent);
        let cid = tree.insert(child);

        tree.append_child(pid, cid);

        assert_eq!(tree.get(pid).unwrap().children, vec![cid]);
        assert_eq!(tree.get(cid).unwrap().parent, pid);
    }

    #[test]
    fn test_insert_before() {
        let mut tree = Tree::new();
        let parent = Node::new_element("rect");
        let c1 = Node::new_element("label");
        let c2 = Node::new_element("label");
        let c3 = Node::new_element("label");
        let pid = tree.insert(parent);
        let c1id = tree.insert(c1);
        let c2id = tree.insert(c2);
        let c3id = tree.insert(c3);

        tree.append_child(pid, c1id);
        tree.append_child(pid, c2id);
        // Insert c3 before c2
        tree.insert_before(pid, c3id, c2id);

        assert_eq!(
            tree.get(pid).unwrap().children,
            vec![c1id, c3id, c2id]
        );
    }

    #[test]
    fn test_remove_child() {
        let mut tree = Tree::new();
        let parent = Node::new_element("rect");
        let child = Node::new_element("label");
        let pid = tree.insert(parent);
        let cid = tree.insert(child);

        tree.append_child(pid, cid);
        tree.remove_child(pid, cid);

        assert!(tree.get(pid).unwrap().children.is_empty());
        assert_eq!(tree.get(cid).unwrap().parent, NodeId::NULL);
    }

    #[test]
    fn test_first_child_next_sibling() {
        let mut tree = Tree::new();
        let parent = Node::new_element("rect");
        let c1 = Node::new_element("label");
        let c2 = Node::new_element("label");
        let pid = tree.insert(parent);
        let c1id = tree.insert(c1);
        let c2id = tree.insert(c2);

        tree.append_child(pid, c1id);
        tree.append_child(pid, c2id);

        assert_eq!(tree.first_child(pid), c1id);
        assert_eq!(tree.next_sibling(c1id), c2id);
        assert_eq!(tree.next_sibling(c2id), NodeId::NULL);
    }

    #[test]
    fn test_parent_node() {
        let mut tree = Tree::new();
        let parent = Node::new_element("rect");
        let child = Node::new_element("label");
        let pid = tree.insert(parent);
        let cid = tree.insert(child);

        tree.append_child(pid, cid);

        assert_eq!(tree.parent_node(cid), pid);
        assert_eq!(tree.parent_node(pid), NodeId::NULL);
    }

    #[test]
    fn test_set_attributes_and_styles() {
        let mut tree = Tree::new();
        let mut node = Node::new_element("rect");
        node.attributes.insert("width".into(), "100%".into());
        node.styles.insert("background".into(), "red".into());
        let id = tree.insert(node);

        let n = tree.get(id).unwrap();
        assert_eq!(n.attributes.get("width").map(|s| s.as_str()), Some("100%"));
        assert_eq!(n.styles.get("background").map(|s| s.as_str()), Some("red"));
    }

    #[test]
    fn test_set_text_content() {
        let mut node = Node::new_text("hello");
        node.set_text_content("world");
        assert_eq!(node.text_content(), Some("world"));
    }

    #[test]
    fn test_reparent_detaches_from_old() {
        let mut tree = Tree::new();
        let p1 = Node::new_element("rect");
        let p2 = Node::new_element("rect");
        let child = Node::new_element("label");
        let p1id = tree.insert(p1);
        let p2id = tree.insert(p2);
        let cid = tree.insert(child);

        tree.append_child(p1id, cid);
        assert_eq!(tree.get(p1id).unwrap().children.len(), 1);

        // Reparent to p2
        tree.append_child(p2id, cid);
        assert!(tree.get(p1id).unwrap().children.is_empty());
        assert_eq!(tree.get(p2id).unwrap().children, vec![cid]);
        assert_eq!(tree.get(cid).unwrap().parent, p2id);
    }
}
