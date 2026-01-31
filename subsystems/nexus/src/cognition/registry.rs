//! # Cognitive Registry
//!
//! Central registry for cognitive components and services.
//! Provides discovery, lookup, and lifecycle management.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ComponentId, DomainId, Timestamp};

// ============================================================================
// REGISTRY TYPES
// ============================================================================

/// Registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    /// Entry ID
    pub id: u64,
    /// Entry name
    pub name: String,
    /// Entry type
    pub entry_type: EntryType,
    /// Entry info
    pub info: EntryInfo,
    /// Owner domain
    pub owner: DomainId,
    /// Status
    pub status: EntryStatus,
    /// Version
    pub version: Version,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Tags
    pub tags: Vec<String>,
    /// Registered at
    pub registered_at: Timestamp,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryType {
    /// Component
    Component,
    /// Service
    Service,
    /// Handler
    Handler,
    /// Plugin
    Plugin,
    /// Model
    Model,
    /// Pipeline
    Pipeline,
    /// Resource
    Resource,
    /// Configuration
    Config,
}

/// Entry info
#[derive(Debug, Clone)]
pub struct EntryInfo {
    /// Description
    pub description: String,
    /// Interface
    pub interface: Option<String>,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Requirements
    pub requirements: Vec<String>,
    /// Configuration schema
    pub config_schema: Option<String>,
}

/// Entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStatus {
    /// Registered but not ready
    Registered,
    /// Initializing
    Initializing,
    /// Active and available
    Active,
    /// Paused
    Paused,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Error state
    Error,
    /// Deprecated
    Deprecated,
}

/// Version
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

// ============================================================================
// LOOKUP
// ============================================================================

/// Lookup query
#[derive(Debug, Clone, Default)]
pub struct LookupQuery {
    /// Filter by name pattern
    pub name_pattern: Option<String>,
    /// Filter by type
    pub entry_type: Option<EntryType>,
    /// Filter by owner
    pub owner: Option<DomainId>,
    /// Filter by status
    pub status: Option<EntryStatus>,
    /// Filter by tags (all must match)
    pub tags: Option<Vec<String>>,
    /// Filter by capability (all must match)
    pub capabilities: Option<Vec<String>>,
    /// Minimum version
    pub min_version: Option<Version>,
}

impl LookupQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, pattern: &str) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    pub fn with_type(mut self, entry_type: EntryType) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    pub fn with_status(mut self, status: EntryStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = Some(caps);
        self
    }
}

/// Lookup result
#[derive(Debug, Clone)]
pub struct LookupResult {
    /// Matching entries
    pub entries: Vec<RegistryEntry>,
    /// Total matches
    pub total: usize,
    /// Query time (ns)
    pub query_time_ns: u64,
}

// ============================================================================
// REGISTRY
// ============================================================================

/// Cognitive registry
pub struct CognitiveRegistry {
    /// Entries
    entries: BTreeMap<u64, RegistryEntry>,
    /// Index by name
    by_name: BTreeMap<String, u64>,
    /// Index by type
    by_type: BTreeMap<EntryType, Vec<u64>>,
    /// Index by owner
    by_owner: BTreeMap<DomainId, Vec<u64>>,
    /// Index by tag
    by_tag: BTreeMap<String, Vec<u64>>,
    /// Watchers
    watchers: BTreeMap<u64, RegistryWatcher>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RegistryConfig,
    /// Statistics
    stats: RegistryStats,
}

/// Registry watcher
#[derive(Debug, Clone)]
pub struct RegistryWatcher {
    /// Watcher ID
    pub id: u64,
    /// Filter
    pub filter: LookupQuery,
    /// Events to watch
    pub events: Vec<RegistryEvent>,
}

/// Registry event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryEvent {
    Registered,
    Updated,
    StatusChanged,
    Unregistered,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum entries
    pub max_entries: usize,
    /// Enable dependency validation
    pub validate_dependencies: bool,
    /// Enable version compatibility check
    pub check_versions: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            validate_dependencies: true,
            check_versions: true,
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// Total entries
    pub total_entries: u64,
    /// Active entries
    pub active_entries: u64,
    /// Lookups performed
    pub lookups: u64,
    /// Registrations
    pub registrations: u64,
    /// Unregistrations
    pub unregistrations: u64,
}

impl CognitiveRegistry {
    /// Create new registry
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            entries: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_owner: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            watchers: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: RegistryStats::default(),
        }
    }

    /// Register entry
    pub fn register(
        &mut self,
        name: &str,
        entry_type: EntryType,
        info: EntryInfo,
        owner: DomainId,
        version: Version,
        dependencies: Vec<u64>,
        tags: Vec<String>,
    ) -> Result<u64, &'static str> {
        // Check capacity
        if self.entries.len() >= self.config.max_entries {
            return Err("Registry capacity exceeded");
        }

        // Check name uniqueness
        if self.by_name.contains_key(name) {
            return Err("Name already registered");
        }

        // Validate dependencies
        if self.config.validate_dependencies {
            for dep_id in &dependencies {
                if !self.entries.contains_key(dep_id) {
                    return Err("Unknown dependency");
                }
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let entry = RegistryEntry {
            id,
            name: name.into(),
            entry_type,
            info,
            owner,
            status: EntryStatus::Registered,
            version,
            dependencies,
            metadata: BTreeMap::new(),
            tags: tags.clone(),
            registered_at: now,
            updated_at: now,
        };

        // Add to indices
        self.by_name.insert(name.into(), id);
        self.by_type
            .entry(entry_type)
            .or_insert_with(Vec::new)
            .push(id);
        self.by_owner.entry(owner).or_insert_with(Vec::new).push(id);
        for tag in &tags {
            self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.entries.insert(id, entry);

        // Update stats
        self.stats.total_entries += 1;
        self.stats.registrations += 1;

        Ok(id)
    }

    /// Unregister entry
    pub fn unregister(&mut self, id: u64) -> Result<(), &'static str> {
        let entry = self.entries.remove(&id).ok_or("Entry not found")?;

        // Check dependents
        let dependents: Vec<u64> = self
            .entries
            .values()
            .filter(|e| e.dependencies.contains(&id))
            .map(|e| e.id)
            .collect();

        if !dependents.is_empty() {
            // Restore entry
            self.entries.insert(id, entry);
            return Err("Entry has dependents");
        }

        // Remove from indices
        self.by_name.remove(&entry.name);

        if let Some(ids) = self.by_type.get_mut(&entry.entry_type) {
            ids.retain(|&i| i != id);
        }

        if let Some(ids) = self.by_owner.get_mut(&entry.owner) {
            ids.retain(|&i| i != id);
        }

        for tag in &entry.tags {
            if let Some(ids) = self.by_tag.get_mut(tag) {
                ids.retain(|&i| i != id);
            }
        }

        self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
        if entry.status == EntryStatus::Active {
            self.stats.active_entries = self.stats.active_entries.saturating_sub(1);
        }
        self.stats.unregistrations += 1;

        Ok(())
    }

    /// Update entry status
    pub fn set_status(&mut self, id: u64, status: EntryStatus) -> Result<(), &'static str> {
        let entry = self.entries.get_mut(&id).ok_or("Entry not found")?;

        let was_active = entry.status == EntryStatus::Active;
        let is_active = status == EntryStatus::Active;

        entry.status = status;
        entry.updated_at = Timestamp::now();

        // Update active count
        match (was_active, is_active) {
            (false, true) => self.stats.active_entries += 1,
            (true, false) => {
                self.stats.active_entries = self.stats.active_entries.saturating_sub(1)
            },
            _ => {},
        }

        Ok(())
    }

    /// Update entry metadata
    pub fn set_metadata(&mut self, id: u64, key: &str, value: &str) -> Result<(), &'static str> {
        let entry = self.entries.get_mut(&id).ok_or("Entry not found")?;
        entry.metadata.insert(key.into(), value.into());
        entry.updated_at = Timestamp::now();
        Ok(())
    }

    /// Lookup entries
    pub fn lookup(&mut self, query: &LookupQuery) -> LookupResult {
        let start = Timestamp::now();
        self.stats.lookups += 1;

        let entries: Vec<_> = self
            .entries
            .values()
            .filter(|e| self.matches_query(e, query))
            .cloned()
            .collect();

        let total = entries.len();
        let query_time_ns = Timestamp::now().elapsed_since(start);

        LookupResult {
            entries,
            total,
            query_time_ns,
        }
    }

    fn matches_query(&self, entry: &RegistryEntry, query: &LookupQuery) -> bool {
        // Name pattern
        if let Some(pattern) = &query.name_pattern {
            if !entry.name.contains(pattern) {
                return false;
            }
        }

        // Entry type
        if let Some(et) = query.entry_type {
            if entry.entry_type != et {
                return false;
            }
        }

        // Owner
        if let Some(owner) = query.owner {
            if entry.owner != owner {
                return false;
            }
        }

        // Status
        if let Some(status) = query.status {
            if entry.status != status {
                return false;
            }
        }

        // Tags
        if let Some(tags) = &query.tags {
            if !tags.iter().all(|t| entry.tags.contains(t)) {
                return false;
            }
        }

        // Capabilities
        if let Some(caps) = &query.capabilities {
            if !caps.iter().all(|c| entry.info.capabilities.contains(c)) {
                return false;
            }
        }

        // Version
        if let Some(min_version) = &query.min_version {
            if entry.version < *min_version {
                return false;
            }
        }

        true
    }

    /// Get entry by ID
    pub fn get(&self, id: u64) -> Option<&RegistryEntry> {
        self.entries.get(&id)
    }

    /// Get entry by name
    pub fn get_by_name(&self, name: &str) -> Option<&RegistryEntry> {
        let id = self.by_name.get(name)?;
        self.entries.get(id)
    }

    /// Get entries by type
    pub fn get_by_type(&self, entry_type: EntryType) -> Vec<&RegistryEntry> {
        self.by_type
            .get(&entry_type)
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get entries by owner
    pub fn get_by_owner(&self, owner: DomainId) -> Vec<&RegistryEntry> {
        self.by_owner
            .get(&owner)
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get dependencies
    pub fn get_dependencies(&self, id: u64) -> Vec<&RegistryEntry> {
        self.entries
            .get(&id)
            .map(|e| {
                e.dependencies
                    .iter()
                    .filter_map(|d| self.entries.get(d))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get dependents
    pub fn get_dependents(&self, id: u64) -> Vec<&RegistryEntry> {
        self.entries
            .values()
            .filter(|e| e.dependencies.contains(&id))
            .collect()
    }

    /// Add watcher
    pub fn watch(&mut self, filter: LookupQuery, events: Vec<RegistryEvent>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let watcher = RegistryWatcher { id, filter, events };

        self.watchers.insert(id, watcher);
        id
    }

    /// Remove watcher
    pub fn unwatch(&mut self, id: u64) {
        self.watchers.remove(&id);
    }

    /// Get all active services
    pub fn active_services(&self) -> Vec<&RegistryEntry> {
        self.entries
            .values()
            .filter(|e| e.entry_type == EntryType::Service && e.status == EntryStatus::Active)
            .collect()
    }

    /// Get all active components
    pub fn active_components(&self) -> Vec<&RegistryEntry> {
        self.entries
            .values()
            .filter(|e| e.entry_type == EntryType::Component && e.status == EntryStatus::Active)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &RegistryStats {
        &self.stats
    }
}

impl Default for CognitiveRegistry {
    fn default() -> Self {
        Self::new(RegistryConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registration() {
        let mut registry = CognitiveRegistry::default();
        let owner = DomainId::new(1);

        let id = registry
            .register(
                "test_component",
                EntryType::Component,
                EntryInfo {
                    description: "Test component".into(),
                    interface: None,
                    capabilities: vec!["compute".into()],
                    requirements: vec![],
                    config_schema: None,
                },
                owner,
                Version::new(1, 0, 0),
                vec![],
                vec!["test".into()],
            )
            .unwrap();

        assert!(registry.get(id).is_some());
        assert!(registry.get_by_name("test_component").is_some());
    }

    #[test]
    fn test_lookup() {
        let mut registry = CognitiveRegistry::default();
        let owner = DomainId::new(1);

        registry
            .register(
                "service_a",
                EntryType::Service,
                EntryInfo {
                    description: "Service A".into(),
                    interface: None,
                    capabilities: vec!["api".into()],
                    requirements: vec![],
                    config_schema: None,
                },
                owner,
                Version::new(1, 0, 0),
                vec![],
                vec![],
            )
            .unwrap();

        let query = LookupQuery::new().with_type(EntryType::Service);

        let result = registry.lookup(&query);
        assert_eq!(result.total, 1);
    }

    #[test]
    fn test_dependencies() {
        let mut registry = CognitiveRegistry::default();
        let owner = DomainId::new(1);

        let dep_id = registry
            .register(
                "dependency",
                EntryType::Component,
                EntryInfo {
                    description: "Dep".into(),
                    interface: None,
                    capabilities: vec![],
                    requirements: vec![],
                    config_schema: None,
                },
                owner,
                Version::new(1, 0, 0),
                vec![],
                vec![],
            )
            .unwrap();

        let main_id = registry
            .register(
                "main",
                EntryType::Component,
                EntryInfo {
                    description: "Main".into(),
                    interface: None,
                    capabilities: vec![],
                    requirements: vec![],
                    config_schema: None,
                },
                owner,
                Version::new(1, 0, 0),
                vec![dep_id],
                vec![],
            )
            .unwrap();

        let deps = registry.get_dependencies(main_id);
        assert_eq!(deps.len(), 1);

        // Should fail - has dependents
        assert!(registry.unregister(dep_id).is_err());

        // Should succeed
        assert!(registry.unregister(main_id).is_ok());
        assert!(registry.unregister(dep_id).is_ok());
    }

    #[test]
    fn test_status_update() {
        let mut registry = CognitiveRegistry::default();
        let owner = DomainId::new(1);

        let id = registry
            .register(
                "test",
                EntryType::Service,
                EntryInfo {
                    description: "Test".into(),
                    interface: None,
                    capabilities: vec![],
                    requirements: vec![],
                    config_schema: None,
                },
                owner,
                Version::new(1, 0, 0),
                vec![],
                vec![],
            )
            .unwrap();

        assert_eq!(registry.get(id).unwrap().status, EntryStatus::Registered);

        registry.set_status(id, EntryStatus::Active).unwrap();
        assert_eq!(registry.get(id).unwrap().status, EntryStatus::Active);
        assert_eq!(registry.stats().active_entries, 1);
    }
}
