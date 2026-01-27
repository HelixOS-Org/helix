//! # Dependency Resolution
//!
//! Handles module dependency resolution and ordering.

use crate::{
    ModuleId, ModuleMetadata, ModuleDependency, ModuleVersion,
    ModuleResult, ModuleError,
    registry::{self, ModuleRegistry},
};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Dependency graph
pub struct DependencyGraph {
    /// Edges: module -> dependencies
    edges: BTreeMap<ModuleId, Vec<ModuleId>>,
    /// Reverse edges: module -> dependents
    reverse_edges: BTreeMap<ModuleId, Vec<ModuleId>>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
            reverse_edges: BTreeMap::new(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, id: ModuleId) {
        self.edges.entry(id).or_default();
        self.reverse_edges.entry(id).or_default();
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        self.edges.entry(from).or_default().push(to);
        self.reverse_edges.entry(to).or_default().push(from);
    }

    /// Get dependencies of a module
    pub fn dependencies(&self, id: ModuleId) -> &[ModuleId] {
        self.edges.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get dependents of a module
    pub fn dependents(&self, id: ModuleId) -> &[ModuleId] {
        self.reverse_edges.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self) -> Option<Vec<ModuleId>> {
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();
        let mut path = Vec::new();

        for &id in self.edges.keys() {
            if self.has_cycle_util(id, &mut visited, &mut rec_stack, &mut path) {
                return Some(path);
            }
        }

        None
    }

    fn has_cycle_util(
        &self,
        id: ModuleId,
        visited: &mut BTreeSet<ModuleId>,
        rec_stack: &mut BTreeSet<ModuleId>,
        path: &mut Vec<ModuleId>,
    ) -> bool {
        if rec_stack.contains(&id) {
            path.push(id);
            return true;
        }

        if visited.contains(&id) {
            return false;
        }

        visited.insert(id);
        rec_stack.insert(id);
        path.push(id);

        for &dep in self.dependencies(id) {
            if self.has_cycle_util(dep, visited, rec_stack, path) {
                return true;
            }
        }

        rec_stack.remove(&id);
        path.pop();
        false
    }

    /// Topological sort (returns load order)
    pub fn topological_sort(&self) -> ModuleResult<Vec<ModuleId>> {
        if let Some(cycle) = self.has_cycle() {
            return Err(ModuleError::CircularDependency);
        }

        let mut result = Vec::new();
        let mut visited = BTreeSet::new();

        for &id in self.edges.keys() {
            self.topo_visit(id, &mut visited, &mut result);
        }

        Ok(result)
    }

    fn topo_visit(
        &self,
        id: ModuleId,
        visited: &mut BTreeSet<ModuleId>,
        result: &mut Vec<ModuleId>,
    ) {
        if visited.contains(&id) {
            return;
        }

        visited.insert(id);

        for &dep in self.dependencies(id) {
            self.topo_visit(dep, visited, result);
        }

        result.push(id);
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency resolver
pub struct DependencyResolver<'a> {
    registry: &'a ModuleRegistry,
}

impl<'a> DependencyResolver<'a> {
    /// Create a new resolver
    pub fn new(registry: &'a ModuleRegistry) -> Self {
        Self { registry }
    }

    /// Resolve dependencies for a module
    pub fn resolve(&self, metadata: &ModuleMetadata) -> ModuleResult<Vec<ModuleId>> {
        let mut graph = DependencyGraph::new();
        let mut to_resolve = vec![metadata.clone()];
        let mut resolved = BTreeSet::new();

        while let Some(current) = to_resolve.pop() {
            if resolved.contains(&current.id) {
                continue;
            }

            graph.add_module(current.id);

            for dep in &current.dependencies {
                let dep_metadata = self.resolve_dependency(dep)?;
                graph.add_dependency(current.id, dep_metadata.id);

                if !resolved.contains(&dep_metadata.id) {
                    to_resolve.push(dep_metadata);
                }
            }

            resolved.insert(current.id);
        }

        graph.topological_sort()
    }

    /// Resolve a single dependency
    fn resolve_dependency(&self, dep: &ModuleDependency) -> ModuleResult<ModuleMetadata> {
        let metadata = self.registry.get_by_name(&dep.name)
            .ok_or_else(|| {
                if dep.optional {
                    // For optional deps, return a placeholder error
                    // The caller should handle this
                    ModuleError::NotFound
                } else {
                    ModuleError::DependencyNotSatisfied(dep.name.clone())
                }
            })?;

        // Check version compatibility
        if !metadata.version.is_compatible_with(&dep.min_version) {
            return Err(ModuleError::VersionMismatch {
                expected: dep.min_version,
                found: metadata.version,
            });
        }

        if let Some(max) = dep.max_version {
            if metadata.version > max {
                return Err(ModuleError::VersionMismatch {
                    expected: max,
                    found: metadata.version,
                });
            }
        }

        Ok(metadata)
    }

    /// Check if all dependencies can be satisfied
    pub fn can_satisfy(&self, metadata: &ModuleMetadata) -> bool {
        for dep in &metadata.dependencies {
            if dep.optional {
                continue;
            }

            if self.resolve_dependency(dep).is_err() {
                return false;
            }
        }
        true
    }

    /// Get unload order (reverse of load order, respecting dependents)
    pub fn unload_order(&self, id: ModuleId) -> ModuleResult<Vec<ModuleId>> {
        let mut to_unload = vec![id];
        let mut result = Vec::new();

        while let Some(current) = to_unload.pop() {
            let dependents = self.registry.get_dependents(current);
            
            for dep in dependents {
                if !to_unload.contains(&dep) && !result.contains(&dep) {
                    to_unload.push(dep);
                }
            }

            result.push(current);
        }

        Ok(result)
    }
}

/// Version constraint
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    /// Exact version
    Exact(ModuleVersion),
    /// Minimum version
    AtLeast(ModuleVersion),
    /// Maximum version
    AtMost(ModuleVersion),
    /// Range (inclusive)
    Range(ModuleVersion, ModuleVersion),
    /// Any version
    Any,
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint
    pub fn satisfies(&self, version: &ModuleVersion) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::AtLeast(v) => version >= v,
            VersionConstraint::AtMost(v) => version <= v,
            VersionConstraint::Range(min, max) => version >= min && version <= max,
            VersionConstraint::Any => true,
        }
    }
}
