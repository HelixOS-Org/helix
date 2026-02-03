//! # Scene Graph
//!
//! Node-based scene graph for hierarchy management.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::ecs::Entity;
use super::transform::Transform;

/// Scene graph
pub struct SceneGraph {
    nodes: BTreeMap<NodeHandle, Node>,
    root: NodeHandle,
    next_handle: u64,
}

impl SceneGraph {
    pub fn new() -> Self {
        let root = NodeHandle(0);
        let mut nodes = BTreeMap::new();
        nodes.insert(root, Node {
            name: "Root".into(),
            parent: None,
            children: Vec::new(),
            transform: Transform::new(),
            entity: None,
            enabled: true,
            layer: Layer::DEFAULT,
        });

        Self {
            nodes,
            root,
            next_handle: 1,
        }
    }

    /// Get root node
    pub fn root(&self) -> NodeHandle {
        self.root
    }

    /// Create a new node
    pub fn create_node(&mut self, name: &str) -> NodeHandle {
        let handle = NodeHandle(self.next_handle);
        self.next_handle += 1;

        self.nodes.insert(handle, Node {
            name: name.into(),
            parent: Some(self.root),
            children: Vec::new(),
            transform: Transform::new(),
            entity: None,
            enabled: true,
            layer: Layer::DEFAULT,
        });

        if let Some(root) = self.nodes.get_mut(&self.root) {
            root.children.push(handle);
        }

        handle
    }

    /// Create node as child of parent
    pub fn create_child(&mut self, parent: NodeHandle, name: &str) -> NodeHandle {
        let handle = NodeHandle(self.next_handle);
        self.next_handle += 1;

        self.nodes.insert(handle, Node {
            name: name.into(),
            parent: Some(parent),
            children: Vec::new(),
            transform: Transform::new(),
            entity: None,
            enabled: true,
            layer: Layer::DEFAULT,
        });

        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.push(handle);
        }

        handle
    }

    /// Get node
    pub fn get(&self, handle: NodeHandle) -> Option<&Node> {
        self.nodes.get(&handle)
    }

    /// Get mutable node
    pub fn get_mut(&mut self, handle: NodeHandle) -> Option<&mut Node> {
        self.nodes.get_mut(&handle)
    }

    /// Set parent
    pub fn set_parent(&mut self, node: NodeHandle, new_parent: NodeHandle) {
        // Remove from old parent
        if let Some(n) = self.nodes.get(&node) {
            if let Some(old_parent) = n.parent {
                if let Some(p) = self.nodes.get_mut(&old_parent) {
                    p.children.retain(|&c| c != node);
                }
            }
        }

        // Set new parent
        if let Some(n) = self.nodes.get_mut(&node) {
            n.parent = Some(new_parent);
        }

        // Add to new parent's children
        if let Some(p) = self.nodes.get_mut(&new_parent) {
            p.children.push(node);
        }
    }

    /// Remove node and reparent children
    pub fn remove(&mut self, handle: NodeHandle) {
        if handle == self.root {
            return;
        }

        if let Some(node) = self.nodes.remove(&handle) {
            // Reparent children to parent
            let parent = node.parent.unwrap_or(self.root);
            for child in &node.children {
                if let Some(c) = self.nodes.get_mut(child) {
                    c.parent = Some(parent);
                }
                if let Some(p) = self.nodes.get_mut(&parent) {
                    p.children.push(*child);
                }
            }

            // Remove from parent's children
            if let Some(p) = self.nodes.get_mut(&parent) {
                p.children.retain(|&c| c != handle);
            }
        }
    }

    /// Remove node and all descendants
    pub fn remove_recursive(&mut self, handle: NodeHandle) {
        if handle == self.root {
            return;
        }

        let mut to_remove = Vec::new();
        self.collect_descendants(handle, &mut to_remove);
        to_remove.push(handle);

        // Remove from parent
        if let Some(node) = self.nodes.get(&handle) {
            if let Some(parent) = node.parent {
                if let Some(p) = self.nodes.get_mut(&parent) {
                    p.children.retain(|&c| c != handle);
                }
            }
        }

        // Remove all
        for h in to_remove {
            self.nodes.remove(&h);
        }
    }

    fn collect_descendants(&self, handle: NodeHandle, out: &mut Vec<NodeHandle>) {
        if let Some(node) = self.nodes.get(&handle) {
            for &child in &node.children {
                out.push(child);
                self.collect_descendants(child, out);
            }
        }
    }

    /// Find node by name
    pub fn find(&self, name: &str) -> Option<NodeHandle> {
        self.nodes
            .iter()
            .find(|(_, n)| n.name == name)
            .map(|(&h, _)| h)
    }

    /// Find nodes by path (e.g., "Parent/Child/GrandChild")
    pub fn find_by_path(&self, path: &str) -> Option<NodeHandle> {
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = self.root;

        for part in parts {
            let mut found = false;
            if let Some(node) = self.nodes.get(&current) {
                for &child in &node.children {
                    if let Some(c) = self.nodes.get(&child) {
                        if c.name == part {
                            current = child;
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                return None;
            }
        }

        Some(current)
    }

    /// Get world transform for node
    pub fn world_transform(&self, handle: NodeHandle) -> Transform {
        let mut transforms = Vec::new();
        let mut current = handle;

        // Collect transforms from node to root
        while let Some(node) = self.nodes.get(&current) {
            transforms.push(node.transform.clone());
            if let Some(parent) = node.parent {
                current = parent;
            } else {
                break;
            }
        }

        // Apply from root to node
        let mut result = Transform::new();
        for t in transforms.iter().rev() {
            result.update_world_matrix(Some(t));
        }

        result
    }

    /// Iterate all nodes depth-first
    pub fn iter(&self) -> impl Iterator<Item = (NodeHandle, &Node)> {
        self.nodes.iter().map(|(&h, n)| (h, n))
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Node handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeHandle(pub u64);

/// Scene node
pub struct Node {
    pub name: String,
    pub parent: Option<NodeHandle>,
    pub children: Vec<NodeHandle>,
    pub transform: Transform,
    pub entity: Option<Entity>,
    pub enabled: bool,
    pub layer: Layer,
}

/// Layer for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer(pub u32);

impl Layer {
    pub const DEFAULT: Layer = Layer(1 << 0);
    pub const UI: Layer = Layer(1 << 1);
    pub const WATER: Layer = Layer(1 << 2);
    pub const TRANSPARENT: Layer = Layer(1 << 3);
    pub const IGNORE_RAYCAST: Layer = Layer(1 << 4);

    pub fn contains(self, other: Layer) -> bool {
        (self.0 & other.0) != 0
    }
}

/// Layer mask for filtering
#[derive(Debug, Clone, Copy)]
pub struct LayerMask(pub u32);

impl LayerMask {
    pub const ALL: LayerMask = LayerMask(u32::MAX);
    pub const NONE: LayerMask = LayerMask(0);

    pub fn from_layers(layers: &[Layer]) -> Self {
        let mut mask = 0u32;
        for l in layers {
            mask |= l.0;
        }
        LayerMask(mask)
    }

    pub fn includes(self, layer: Layer) -> bool {
        (self.0 & layer.0) != 0
    }
}

/// Scene query
pub struct SceneQuery<'a> {
    graph: &'a SceneGraph,
    layer_mask: LayerMask,
    enabled_only: bool,
}

impl<'a> SceneQuery<'a> {
    pub fn new(graph: &'a SceneGraph) -> Self {
        Self {
            graph,
            layer_mask: LayerMask::ALL,
            enabled_only: true,
        }
    }

    pub fn with_layer(mut self, mask: LayerMask) -> Self {
        self.layer_mask = mask;
        self
    }

    pub fn include_disabled(mut self) -> Self {
        self.enabled_only = false;
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeHandle, &'a Node)> {
        let layer_mask = self.layer_mask;
        let enabled_only = self.enabled_only;

        self.graph
            .nodes
            .iter()
            .filter(move |(_, n)| (!enabled_only || n.enabled) && layer_mask.includes(n.layer))
            .map(|(&h, n)| (h, n))
    }
}
