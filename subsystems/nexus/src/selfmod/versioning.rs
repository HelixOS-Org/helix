//! # Code Versioning
//!
//! Year 3 EVOLUTION - Q3 - Code versioning and history system

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ModificationId, SnapshotId, VersionId};

// ============================================================================
// VERSION TYPES
// ============================================================================

/// Commit ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitId(pub u64);

/// Branch ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchId(pub u64);

/// Tag ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagId(pub u64);

static COMMIT_COUNTER: AtomicU64 = AtomicU64::new(1);
static BRANCH_COUNTER: AtomicU64 = AtomicU64::new(1);
static TAG_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Commit
#[derive(Debug, Clone)]
pub struct Commit {
    /// Commit ID
    pub id: CommitId,
    /// Parent commit (if any)
    pub parent: Option<CommitId>,
    /// Secondary parent (for merges)
    pub merge_parent: Option<CommitId>,
    /// Version ID
    pub version: VersionId,
    /// Modifications included
    pub modifications: Vec<ModificationId>,
    /// Snapshot (if captured)
    pub snapshot: Option<SnapshotId>,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
    /// Hash (for integrity)
    pub hash: u64,
}

/// Branch
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch ID
    pub id: BranchId,
    /// Name
    pub name: String,
    /// Head commit
    pub head: CommitId,
    /// Base commit (where branch started)
    pub base: CommitId,
    /// Protected (cannot force push)
    pub protected: bool,
}

/// Tag
#[derive(Debug, Clone)]
pub struct Tag {
    /// Tag ID
    pub id: TagId,
    /// Name
    pub name: String,
    /// Target commit
    pub target: CommitId,
    /// Annotated (has message)
    pub annotated: bool,
    /// Message (if annotated)
    pub message: Option<String>,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// VERSION GRAPH
// ============================================================================

/// Version graph (DAG)
pub struct VersionGraph {
    /// Commits
    commits: BTreeMap<CommitId, Commit>,
    /// Branches
    branches: BTreeMap<BranchId, Branch>,
    /// Tags
    tags: BTreeMap<TagId, Tag>,
    /// Branch name to ID
    branch_names: BTreeMap<String, BranchId>,
    /// Tag name to ID
    tag_names: BTreeMap<String, TagId>,
    /// Current branch
    current_branch: BranchId,
    /// Main branch
    main_branch: BranchId,
}

impl VersionGraph {
    /// Create new version graph
    pub fn new() -> Self {
        // Create initial commit
        let initial_commit = Commit {
            id: CommitId(0),
            parent: None,
            merge_parent: None,
            version: VersionId(0),
            modifications: Vec::new(),
            snapshot: None,
            message: String::from("Initial commit"),
            timestamp: 0,
            hash: 0,
        };

        let mut commits = BTreeMap::new();
        commits.insert(CommitId(0), initial_commit);

        // Create main branch
        let main = Branch {
            id: BranchId(0),
            name: String::from("main"),
            head: CommitId(0),
            base: CommitId(0),
            protected: true,
        };

        let mut branches = BTreeMap::new();
        branches.insert(BranchId(0), main);

        let mut branch_names = BTreeMap::new();
        branch_names.insert(String::from("main"), BranchId(0));

        Self {
            commits,
            branches,
            tags: BTreeMap::new(),
            branch_names,
            tag_names: BTreeMap::new(),
            current_branch: BranchId(0),
            main_branch: BranchId(0),
        }
    }

    /// Create a commit
    pub fn commit(
        &mut self,
        version: VersionId,
        modifications: Vec<ModificationId>,
        message: impl Into<String>,
    ) -> CommitId {
        let id = CommitId(COMMIT_COUNTER.fetch_add(1, Ordering::SeqCst));
        let parent = self.get_head();

        let hash = self.calculate_hash(&modifications, parent);

        let commit = Commit {
            id,
            parent: Some(parent),
            merge_parent: None,
            version,
            modifications,
            snapshot: None,
            message: message.into(),
            timestamp: 0,
            hash,
        };

        self.commits.insert(id, commit);

        // Update branch head
        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head = id;
        }

        id
    }

    /// Create a branch
    pub fn create_branch(&mut self, name: impl Into<String>) -> BranchId {
        let name = name.into();
        let id = BranchId(BRANCH_COUNTER.fetch_add(1, Ordering::SeqCst));
        let head = self.get_head();

        let branch = Branch {
            id,
            name: name.clone(),
            head,
            base: head,
            protected: false,
        };

        self.branches.insert(id, branch);
        self.branch_names.insert(name, id);

        id
    }

    /// Switch branch
    pub fn checkout(&mut self, branch_id: BranchId) -> Result<(), VersionError> {
        if !self.branches.contains_key(&branch_id) {
            return Err(VersionError::BranchNotFound(branch_id));
        }
        self.current_branch = branch_id;
        Ok(())
    }

    /// Checkout by name
    pub fn checkout_name(&mut self, name: &str) -> Result<(), VersionError> {
        let branch_id = self
            .branch_names
            .get(name)
            .copied()
            .ok_or(VersionError::BranchNotFound(BranchId(0)))?;
        self.checkout(branch_id)
    }

    /// Merge branch into current
    pub fn merge(
        &mut self,
        source: BranchId,
        message: impl Into<String>,
    ) -> Result<CommitId, VersionError> {
        let source_branch = self
            .branches
            .get(&source)
            .ok_or(VersionError::BranchNotFound(source))?
            .clone();

        let target_head = self.get_head();
        let source_head = source_branch.head;

        // Check for conflicts (simplified)
        let conflicts = self.detect_conflicts(target_head, source_head);
        if !conflicts.is_empty() {
            return Err(VersionError::MergeConflict(conflicts));
        }

        // Create merge commit
        let id = CommitId(COMMIT_COUNTER.fetch_add(1, Ordering::SeqCst));

        // Collect modifications from both branches
        let modifications = self.collect_modifications(target_head, source_head);
        let hash = self.calculate_hash(&modifications, target_head);

        let commit = Commit {
            id,
            parent: Some(target_head),
            merge_parent: Some(source_head),
            version: VersionId(id.0),
            modifications,
            snapshot: None,
            message: message.into(),
            timestamp: 0,
            hash,
        };

        self.commits.insert(id, commit);

        // Update branch head
        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head = id;
        }

        Ok(id)
    }

    /// Create a tag
    pub fn tag(&mut self, name: impl Into<String>, message: Option<String>) -> TagId {
        let name = name.into();
        let id = TagId(TAG_COUNTER.fetch_add(1, Ordering::SeqCst));
        let target = self.get_head();

        let tag = Tag {
            id,
            name: name.clone(),
            target,
            annotated: message.is_some(),
            message,
            timestamp: 0,
        };

        self.tags.insert(id, tag);
        self.tag_names.insert(name, id);

        id
    }

    /// Get current head
    pub fn get_head(&self) -> CommitId {
        self.branches
            .get(&self.current_branch)
            .map(|b| b.head)
            .unwrap_or(CommitId(0))
    }

    /// Get commit
    pub fn get_commit(&self, id: CommitId) -> Option<&Commit> {
        self.commits.get(&id)
    }

    /// Get branch
    pub fn get_branch(&self, id: BranchId) -> Option<&Branch> {
        self.branches.get(&id)
    }

    /// Get history
    pub fn history(&self, count: usize) -> Vec<&Commit> {
        let mut history = Vec::new();
        let mut current = Some(self.get_head());

        while let Some(id) = current {
            if history.len() >= count {
                break;
            }

            if let Some(commit) = self.commits.get(&id) {
                history.push(commit);
                current = commit.parent;
            } else {
                break;
            }
        }

        history
    }

    /// Find common ancestor
    pub fn find_common_ancestor(&self, a: CommitId, b: CommitId) -> Option<CommitId> {
        let ancestors_a = self.collect_ancestors(a);
        let mut current = Some(b);

        while let Some(id) = current {
            if ancestors_a.contains(&id) {
                return Some(id);
            }

            if let Some(commit) = self.commits.get(&id) {
                current = commit.parent;
            } else {
                break;
            }
        }

        None
    }

    fn collect_ancestors(&self, start: CommitId) -> Vec<CommitId> {
        let mut ancestors = Vec::new();
        let mut current = Some(start);

        while let Some(id) = current {
            ancestors.push(id);
            if let Some(commit) = self.commits.get(&id) {
                current = commit.parent;
            } else {
                break;
            }
        }

        ancestors
    }

    fn detect_conflicts(&self, _a: CommitId, _b: CommitId) -> Vec<Conflict> {
        // Simplified conflict detection
        Vec::new()
    }

    fn collect_modifications(&self, a: CommitId, b: CommitId) -> Vec<ModificationId> {
        let mut mods = Vec::new();

        if let Some(commit) = self.commits.get(&a) {
            mods.extend(commit.modifications.clone());
        }
        if let Some(commit) = self.commits.get(&b) {
            for m in &commit.modifications {
                if !mods.contains(m) {
                    mods.push(*m);
                }
            }
        }

        mods
    }

    fn calculate_hash(&self, modifications: &[ModificationId], parent: CommitId) -> u64 {
        let mut hash = parent.0;
        for m in modifications {
            hash = hash.wrapping_mul(31).wrapping_add(m.0);
        }
        hash
    }

    /// Get current branch
    pub fn current_branch(&self) -> &Branch {
        self.branches.get(&self.current_branch).unwrap()
    }

    /// List branches
    pub fn list_branches(&self) -> impl Iterator<Item = &Branch> {
        self.branches.values()
    }

    /// List tags
    pub fn list_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.values()
    }
}

impl Default for VersionGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Conflicting modification
    pub modification: ModificationId,
    /// Description
    pub description: String,
}

/// Version error
#[derive(Debug)]
pub enum VersionError {
    /// Branch not found
    BranchNotFound(BranchId),
    /// Commit not found
    CommitNotFound(CommitId),
    /// Merge conflict
    MergeConflict(Vec<Conflict>),
    /// Protected branch
    ProtectedBranch,
}

// ============================================================================
// DIFF
// ============================================================================

/// Diff between versions
#[derive(Debug, Clone)]
pub struct Diff {
    /// From commit
    pub from: CommitId,
    /// To commit
    pub to: CommitId,
    /// Changes
    pub changes: Vec<DiffChange>,
    /// Additions
    pub additions: usize,
    /// Deletions
    pub deletions: usize,
}

/// Diff change
#[derive(Debug, Clone)]
pub struct DiffChange {
    /// Change type
    pub change_type: ChangeType,
    /// Modification
    pub modification: ModificationId,
    /// Old bytes (if applicable)
    pub old: Option<Vec<u8>>,
    /// New bytes (if applicable)
    pub new: Option<Vec<u8>>,
}

/// Change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Added
    Added,
    /// Removed
    Removed,
    /// Modified
    Modified,
}

/// Diff calculator
pub struct DiffCalculator;

impl DiffCalculator {
    /// Calculate diff between commits
    pub fn diff(graph: &VersionGraph, from: CommitId, to: CommitId) -> Diff {
        let from_commit = graph.get_commit(from);
        let to_commit = graph.get_commit(to);

        let mut changes = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;

        if let (Some(from_c), Some(to_c)) = (from_commit, to_commit) {
            // Find added
            for m in &to_c.modifications {
                if !from_c.modifications.contains(m) {
                    changes.push(DiffChange {
                        change_type: ChangeType::Added,
                        modification: *m,
                        old: None,
                        new: None,
                    });
                    additions += 1;
                }
            }

            // Find removed
            for m in &from_c.modifications {
                if !to_c.modifications.contains(m) {
                    changes.push(DiffChange {
                        change_type: ChangeType::Removed,
                        modification: *m,
                        old: None,
                        new: None,
                    });
                    deletions += 1;
                }
            }
        }

        Diff {
            from,
            to,
            changes,
            additions,
            deletions,
        }
    }
}

// ============================================================================
// VERSION MANAGER
// ============================================================================

/// Version manager
pub struct VersionManager {
    /// Graph
    graph: VersionGraph,
    /// Configuration
    config: VersionConfig,
    /// Statistics
    stats: VersionStats,
}

/// Version configuration
#[derive(Debug, Clone)]
pub struct VersionConfig {
    /// Auto-commit on deploy
    pub auto_commit: bool,
    /// Auto-tag releases
    pub auto_tag: bool,
    /// Maximum history
    pub max_history: usize,
    /// Snapshot interval
    pub snapshot_interval: u64,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            auto_commit: true,
            auto_tag: false,
            max_history: 1000,
            snapshot_interval: 10,
        }
    }
}

/// Version statistics
#[derive(Debug, Clone, Default)]
pub struct VersionStats {
    /// Total commits
    pub commits: u64,
    /// Total branches
    pub branches: usize,
    /// Total tags
    pub tags: usize,
    /// Merges
    pub merges: u64,
}

impl VersionManager {
    /// Create new manager
    pub fn new(config: VersionConfig) -> Self {
        Self {
            graph: VersionGraph::new(),
            config,
            stats: VersionStats::default(),
        }
    }

    /// Commit changes
    pub fn commit(
        &mut self,
        version: VersionId,
        modifications: Vec<ModificationId>,
        message: impl Into<String>,
    ) -> CommitId {
        let id = self.graph.commit(version, modifications, message);
        self.stats.commits += 1;
        id
    }

    /// Create branch
    pub fn create_branch(&mut self, name: impl Into<String>) -> BranchId {
        let id = self.graph.create_branch(name);
        self.stats.branches = self.graph.branches.len();
        id
    }

    /// Merge
    pub fn merge(
        &mut self,
        source: BranchId,
        message: impl Into<String>,
    ) -> Result<CommitId, VersionError> {
        let id = self.graph.merge(source, message)?;
        self.stats.merges += 1;
        Ok(id)
    }

    /// Tag
    pub fn tag(&mut self, name: impl Into<String>) -> TagId {
        let id = self.graph.tag(name, None);
        self.stats.tags = self.graph.tags.len();
        id
    }

    /// Get graph
    pub fn graph(&self) -> &VersionGraph {
        &self.graph
    }

    /// Get graph mutable
    pub fn graph_mut(&mut self) -> &mut VersionGraph {
        &mut self.graph
    }

    /// Get statistics
    pub fn stats(&self) -> &VersionStats {
        &self.stats
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new(VersionConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_graph_creation() {
        let graph = VersionGraph::new();
        assert_eq!(graph.get_head(), CommitId(0));
    }

    #[test]
    fn test_commit() {
        let mut graph = VersionGraph::new();

        let id = graph.commit(VersionId(1), vec![ModificationId(1)], "Test commit");

        let commit = graph.get_commit(id).unwrap();
        assert_eq!(commit.modifications.len(), 1);
    }

    #[test]
    fn test_branch() {
        let mut graph = VersionGraph::new();

        let branch_id = graph.create_branch("feature");
        assert!(graph.checkout(branch_id).is_ok());

        let branch = graph.current_branch();
        assert_eq!(branch.name, "feature");
    }

    #[test]
    fn test_tag() {
        let mut graph = VersionGraph::new();

        let tag_id = graph.tag("v1.0.0", Some(String::from("First release")));

        let tag = graph.tags.get(&tag_id).unwrap();
        assert!(tag.annotated);
    }
}
