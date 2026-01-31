//! Call graph construction and analysis.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::types::FuncAddr;

// ============================================================================
// CALL GRAPH
// ============================================================================

/// Call graph node
#[derive(Debug, Clone)]
pub struct CallGraphNode {
    /// Function address
    pub func: FuncAddr,
    /// Function name
    pub name: String,
    /// Call count
    pub call_count: u64,
    /// Total time (ns)
    pub total_time_ns: u64,
    /// Self time (ns)
    pub self_time_ns: u64,
    /// Children
    pub children: Vec<FuncAddr>,
    /// Parents
    pub parents: Vec<FuncAddr>,
}

impl CallGraphNode {
    /// Create new node
    pub fn new(func: FuncAddr, name: String) -> Self {
        Self {
            func,
            name,
            call_count: 0,
            total_time_ns: 0,
            self_time_ns: 0,
            children: Vec::new(),
            parents: Vec::new(),
        }
    }

    /// Average time
    pub fn avg_time_ns(&self) -> u64 {
        if self.call_count == 0 {
            return 0;
        }
        self.total_time_ns / self.call_count
    }

    /// Child overhead
    pub fn child_time_ns(&self) -> u64 {
        self.total_time_ns.saturating_sub(self.self_time_ns)
    }

    /// Self time percentage
    pub fn self_time_pct(&self) -> f32 {
        if self.total_time_ns == 0 {
            return 0.0;
        }
        self.self_time_ns as f32 / self.total_time_ns as f32 * 100.0
    }
}

/// Call graph
#[derive(Debug)]
pub struct CallGraph {
    /// Nodes
    nodes: BTreeMap<FuncAddr, CallGraphNode>,
    /// Root functions
    roots: Vec<FuncAddr>,
    /// Total traced time (ns)
    pub total_time_ns: u64,
}

impl CallGraph {
    /// Create new call graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            roots: Vec::new(),
            total_time_ns: 0,
        }
    }

    /// Add or update node
    pub fn add_call(
        &mut self,
        func: FuncAddr,
        name: String,
        parent: Option<FuncAddr>,
        duration_ns: u64,
        self_time_ns: u64,
    ) {
        let node = self
            .nodes
            .entry(func)
            .or_insert_with(|| CallGraphNode::new(func, name));
        node.call_count += 1;
        node.total_time_ns += duration_ns;
        node.self_time_ns += self_time_ns;

        if let Some(parent_addr) = parent {
            if !node.parents.contains(&parent_addr) {
                node.parents.push(parent_addr);
            }
            if let Some(parent_node) = self.nodes.get_mut(&parent_addr) {
                if !parent_node.children.contains(&func) {
                    parent_node.children.push(func);
                }
            }
        } else if !self.roots.contains(&func) {
            self.roots.push(func);
        }
    }

    /// Get node
    pub fn get(&self, func: FuncAddr) -> Option<&CallGraphNode> {
        self.nodes.get(&func)
    }

    /// Get hottest functions
    pub fn hottest(&self, n: usize) -> Vec<&CallGraphNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.total_time_ns.cmp(&a.total_time_ns));
        nodes.into_iter().take(n).collect()
    }

    /// Get functions by self time
    pub fn by_self_time(&self, n: usize) -> Vec<&CallGraphNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.self_time_ns.cmp(&a.self_time_ns));
        nodes.into_iter().take(n).collect()
    }

    /// Total functions
    pub fn function_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}
