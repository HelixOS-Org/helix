//! Device Tree
//!
//! Device tree parsing and node management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Device tree node
#[derive(Debug, Clone)]
pub struct DeviceTreeNode {
    /// Node name
    pub name: String,
    /// Unit address
    pub unit_address: Option<u64>,
    /// Compatible strings
    pub compatible: Vec<String>,
    /// Properties
    pub properties: BTreeMap<String, Vec<u8>>,
    /// Children
    pub children: Vec<DeviceTreeNode>,
    /// Phandle
    pub phandle: Option<u32>,
}

impl DeviceTreeNode {
    /// Create new node
    pub fn new(name: String) -> Self {
        Self {
            name,
            unit_address: None,
            compatible: Vec::new(),
            properties: BTreeMap::new(),
            children: Vec::new(),
            phandle: None,
        }
    }

    /// Get property as string
    #[inline]
    pub fn get_property_string(&self, name: &str) -> Option<String> {
        self.properties
            .get(name)
            .and_then(|v| core::str::from_utf8(v).ok())
            .map(String::from)
    }

    /// Get property as u32
    #[inline]
    pub fn get_property_u32(&self, name: &str) -> Option<u32> {
        self.properties
            .get(name)
            .filter(|v| v.len() >= 4)
            .map(|v| u32::from_be_bytes([v[0], v[1], v[2], v[3]]))
    }

    /// Check if node matches compatible
    #[inline(always)]
    pub fn matches_compatible(&self, compat: &str) -> bool {
        self.compatible.iter().any(|c| c == compat)
    }

    /// Count total nodes in subtree
    #[inline(always)]
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }
}

/// Device tree parser
pub struct DeviceTreeParser {
    /// Root node
    root: Option<DeviceTreeNode>,
    /// Phandle map
    phandle_map: BTreeMap<u32, String>,
    /// Parse errors
    errors: Vec<String>,
}

impl DeviceTreeParser {
    /// Create new parser
    pub fn new() -> Self {
        Self {
            root: None,
            phandle_map: BTreeMap::new(),
            errors: Vec::new(),
        }
    }

    /// Parse device tree blob
    #[inline]
    pub fn parse(&mut self, _dtb: &[u8]) -> bool {
        // Simplified parsing - in real implementation would parse DTB format
        self.root = Some(DeviceTreeNode::new(String::from("/")));
        true
    }

    /// Get root node
    #[inline(always)]
    pub fn root(&self) -> Option<&DeviceTreeNode> {
        self.root.as_ref()
    }

    /// Find node by path
    pub fn find_node(&self, path: &str) -> Option<&DeviceTreeNode> {
        let root = self.root.as_ref()?;

        if path == "/" {
            return Some(root);
        }

        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = root;

        for part in parts {
            let found = current.children.iter().find(|c| c.name == part);
            match found {
                Some(node) => current = node,
                None => return None,
            }
        }

        Some(current)
    }

    /// Find node by phandle
    #[inline(always)]
    pub fn find_by_phandle(&self, phandle: u32) -> Option<&DeviceTreeNode> {
        let path = self.phandle_map.get(&phandle)?;
        self.find_node(path)
    }

    /// Find nodes by compatible
    #[inline]
    pub fn find_compatible(&self, compat: &str) -> Vec<&DeviceTreeNode> {
        let mut result = Vec::new();
        if let Some(root) = &self.root {
            self.find_compatible_recursive(root, compat, &mut result);
        }
        result
    }

    fn find_compatible_recursive<'a>(
        &self,
        node: &'a DeviceTreeNode,
        compat: &str,
        result: &mut Vec<&'a DeviceTreeNode>,
    ) {
        if node.matches_compatible(compat) {
            result.push(node);
        }
        for child in &node.children {
            self.find_compatible_recursive(child, compat, result);
        }
    }

    /// Get parse errors
    #[inline(always)]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

impl Default for DeviceTreeParser {
    fn default() -> Self {
        Self::new()
    }
}
