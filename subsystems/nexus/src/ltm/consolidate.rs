//! # LTM Consolidation
//!
//! Memory consolidation for long-term storage.
//! Strengthens, reorganizes, and integrates memories.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONSOLIDATION TYPES
// ============================================================================

/// Consolidation task
#[derive(Debug, Clone)]
pub struct ConsolidationTask {
    /// Task ID
    pub id: u64,
    /// Task type
    pub task_type: ConsolidationType,
    /// Target memories
    pub targets: Vec<u64>,
    /// Priority
    pub priority: Priority,
    /// Status
    pub status: TaskStatus,
    /// Created
    pub created: Timestamp,
    /// Completed
    pub completed: Option<Timestamp>,
    /// Result
    pub result: Option<ConsolidationResult>,
}

/// Consolidation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsolidationType {
    /// Strengthen frequently accessed
    Strengthen,
    /// Integrate related memories
    Integrate,
    /// Abstract patterns
    Abstract,
    /// Prune weak memories
    Prune,
    /// Reorganize structure
    Reorganize,
    /// Compress redundant
    Compress,
}

/// Priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Consolidation result
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    /// Success
    pub success: bool,
    /// Memories affected
    pub memories_affected: usize,
    /// New memories created
    pub new_memories: Vec<u64>,
    /// Memories removed
    pub removed_memories: Vec<u64>,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Description
    pub description: String,
}

/// Memory state (for consolidation)
#[derive(Debug, Clone)]
pub struct MemoryState {
    /// Memory ID
    pub id: u64,
    /// Strength
    pub strength: f64,
    /// Access count
    pub access_count: u64,
    /// Last access
    pub last_access: Timestamp,
    /// Associations
    pub associations: Vec<u64>,
    /// Tags
    pub tags: Vec<String>,
}

// ============================================================================
// CONSOLIDATION ENGINE
// ============================================================================

/// Consolidation engine
pub struct ConsolidationEngine {
    /// Tasks
    tasks: BTreeMap<u64, ConsolidationTask>,
    /// Memory states
    memory_states: BTreeMap<u64, MemoryState>,
    /// Pending queue
    queue: Vec<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ConsolidationConfig,
    /// Statistics
    stats: ConsolidationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Strengthen threshold (access count)
    pub strengthen_threshold: u64,
    /// Prune threshold (strength)
    pub prune_threshold: f64,
    /// Integration similarity threshold
    pub integration_threshold: f64,
    /// Batch size
    pub batch_size: usize,
    /// Decay rate
    pub decay_rate: f64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            strengthen_threshold: 5,
            prune_threshold: 0.1,
            integration_threshold: 0.7,
            batch_size: 100,
            decay_rate: 0.01,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ConsolidationStats {
    /// Tasks completed
    pub tasks_completed: u64,
    /// Memories strengthened
    pub memories_strengthened: u64,
    /// Memories pruned
    pub memories_pruned: u64,
    /// Memories integrated
    pub memories_integrated: u64,
}

impl ConsolidationEngine {
    /// Create new engine
    pub fn new(config: ConsolidationConfig) -> Self {
        Self {
            tasks: BTreeMap::new(),
            memory_states: BTreeMap::new(),
            queue: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ConsolidationStats::default(),
        }
    }

    /// Register memory
    pub fn register_memory(&mut self, state: MemoryState) {
        self.memory_states.insert(state.id, state);
    }

    /// Update memory access
    pub fn record_access(&mut self, memory_id: u64) {
        if let Some(state) = self.memory_states.get_mut(&memory_id) {
            state.access_count += 1;
            state.last_access = Timestamp::now();
        }
    }

    /// Create task
    pub fn create_task(
        &mut self,
        task_type: ConsolidationType,
        targets: Vec<u64>,
        priority: Priority,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let task = ConsolidationTask {
            id,
            task_type,
            targets,
            priority,
            status: TaskStatus::Pending,
            created: Timestamp::now(),
            completed: None,
            result: None,
        };

        self.tasks.insert(id, task);
        self.queue.push(id);

        // Sort queue by priority
        self.queue.sort_by(|&a, &b| {
            let task_a = self.tasks.get(&a).map(|t| t.priority);
            let task_b = self.tasks.get(&b).map(|t| t.priority);
            task_b.cmp(&task_a)
        });

        id
    }

    /// Execute next task
    pub fn execute_next(&mut self) -> Option<ConsolidationResult> {
        let task_id = self.queue.pop()?;

        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Running;
        }

        let task = self.tasks.get(&task_id)?.clone();
        let result = self.execute_task(&task);

        if let Some(t) = self.tasks.get_mut(&task_id) {
            t.status = if result.success { TaskStatus::Completed } else { TaskStatus::Failed };
            t.completed = Some(Timestamp::now());
            t.result = Some(result.clone());
        }

        self.stats.tasks_completed += 1;

        Some(result)
    }

    fn execute_task(&mut self, task: &ConsolidationTask) -> ConsolidationResult {
        match task.task_type {
            ConsolidationType::Strengthen => self.strengthen(&task.targets),
            ConsolidationType::Prune => self.prune(&task.targets),
            ConsolidationType::Integrate => self.integrate(&task.targets),
            ConsolidationType::Abstract => self.abstract_pattern(&task.targets),
            ConsolidationType::Reorganize => self.reorganize(&task.targets),
            ConsolidationType::Compress => self.compress(&task.targets),
        }
    }

    fn strengthen(&mut self, targets: &[u64]) -> ConsolidationResult {
        let mut affected = 0;

        for &id in targets {
            if let Some(state) = self.memory_states.get_mut(&id) {
                if state.access_count >= self.config.strengthen_threshold {
                    let boost = (state.access_count as f64).ln() * 0.1;
                    state.strength = (state.strength + boost).min(1.0);
                    affected += 1;
                    self.stats.memories_strengthened += 1;
                }
            }
        }

        ConsolidationResult {
            success: true,
            memories_affected: affected,
            new_memories: Vec::new(),
            removed_memories: Vec::new(),
            duration_ns: 1000,
            description: format!("Strengthened {} memories", affected),
        }
    }

    fn prune(&mut self, targets: &[u64]) -> ConsolidationResult {
        let mut removed = Vec::new();

        for &id in targets {
            let should_remove = self.memory_states.get(&id)
                .map(|s| s.strength < self.config.prune_threshold)
                .unwrap_or(false);

            if should_remove {
                self.memory_states.remove(&id);
                removed.push(id);
                self.stats.memories_pruned += 1;
            }
        }

        ConsolidationResult {
            success: true,
            memories_affected: removed.len(),
            new_memories: Vec::new(),
            removed_memories: removed,
            duration_ns: 1000,
            description: "Pruned weak memories".into(),
        }
    }

    fn integrate(&mut self, targets: &[u64]) -> ConsolidationResult {
        let mut integrated = 0;

        // Find similar memories and link them
        for i in 0..targets.len() {
            for j in (i + 1)..targets.len() {
                let similarity = self.compute_similarity(targets[i], targets[j]);

                if similarity >= self.config.integration_threshold {
                    // Add associations
                    if let Some(state) = self.memory_states.get_mut(&targets[i]) {
                        if !state.associations.contains(&targets[j]) {
                            state.associations.push(targets[j]);
                        }
                    }
                    if let Some(state) = self.memory_states.get_mut(&targets[j]) {
                        if !state.associations.contains(&targets[i]) {
                            state.associations.push(targets[i]);
                        }
                    }
                    integrated += 1;
                }
            }
        }

        self.stats.memories_integrated += integrated;

        ConsolidationResult {
            success: true,
            memories_affected: integrated,
            new_memories: Vec::new(),
            removed_memories: Vec::new(),
            duration_ns: 1000,
            description: format!("Integrated {} memory pairs", integrated),
        }
    }

    fn compute_similarity(&self, id1: u64, id2: u64) -> f64 {
        let state1 = match self.memory_states.get(&id1) {
            Some(s) => s,
            None => return 0.0,
        };

        let state2 = match self.memory_states.get(&id2) {
            Some(s) => s,
            None => return 0.0,
        };

        // Tag-based similarity
        let tags1: BTreeSet<_> = state1.tags.iter().collect();
        let tags2: BTreeSet<_> = state2.tags.iter().collect();

        let intersection = tags1.intersection(&tags2).count();
        let union = tags1.union(&tags2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    fn abstract_pattern(&mut self, targets: &[u64]) -> ConsolidationResult {
        // Find common patterns among targets
        let mut common_tags: Option<BTreeSet<String>> = None;

        for &id in targets {
            if let Some(state) = self.memory_states.get(&id) {
                let tags: BTreeSet<String> = state.tags.iter().cloned().collect();
                common_tags = Some(match common_tags {
                    Some(existing) => existing.intersection(&tags).cloned().collect(),
                    None => tags,
                });
            }
        }

        let pattern_tags: Vec<String> = common_tags.unwrap_or_default().into_iter().collect();

        if !pattern_tags.is_empty() {
            // Create abstract memory
            let new_id = self.next_id.fetch_add(1, Ordering::Relaxed);

            let abstract_state = MemoryState {
                id: new_id,
                strength: 0.5,
                access_count: 0,
                last_access: Timestamp::now(),
                associations: targets.to_vec(),
                tags: pattern_tags,
            };

            self.memory_states.insert(new_id, abstract_state);

            return ConsolidationResult {
                success: true,
                memories_affected: targets.len(),
                new_memories: vec![new_id],
                removed_memories: Vec::new(),
                duration_ns: 1000,
                description: "Created abstract pattern".into(),
            };
        }

        ConsolidationResult {
            success: false,
            memories_affected: 0,
            new_memories: Vec::new(),
            removed_memories: Vec::new(),
            duration_ns: 1000,
            description: "No common pattern found".into(),
        }
    }

    fn reorganize(&mut self, _targets: &[u64]) -> ConsolidationResult {
        // Reorganize associations based on similarity
        let ids: Vec<u64> = self.memory_states.keys().copied().collect();

        for &id in &ids {
            // Find strongly associated memories
            let associations: Vec<u64> = self.memory_states.get(&id)
                .map(|s| s.associations.clone())
                .unwrap_or_default();

            // Keep only strong associations
            let strong: Vec<u64> = associations.into_iter()
                .filter(|&assoc_id| {
                    self.memory_states.get(&assoc_id)
                        .map(|s| s.strength > 0.5)
                        .unwrap_or(false)
                })
                .collect();

            if let Some(state) = self.memory_states.get_mut(&id) {
                state.associations = strong;
            }
        }

        ConsolidationResult {
            success: true,
            memories_affected: ids.len(),
            new_memories: Vec::new(),
            removed_memories: Vec::new(),
            duration_ns: 1000,
            description: "Reorganized memory associations".into(),
        }
    }

    fn compress(&mut self, targets: &[u64]) -> ConsolidationResult {
        let mut removed = Vec::new();

        // Find redundant memories (same tags, low strength)
        let mut tag_groups: BTreeMap<Vec<String>, Vec<u64>> = BTreeMap::new();

        for &id in targets {
            if let Some(state) = self.memory_states.get(&id) {
                let mut tags = state.tags.clone();
                tags.sort();
                tag_groups.entry(tags)
                    .or_insert_with(Vec::new)
                    .push(id);
            }
        }

        // For each group, keep strongest
        for (_, group) in tag_groups {
            if group.len() > 1 {
                let mut group_sorted: Vec<_> = group.iter()
                    .filter_map(|&id| {
                        self.memory_states.get(&id)
                            .map(|s| (id, s.strength))
                    })
                    .collect();

                group_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                // Remove all but the strongest
                for &(id, _) in group_sorted.iter().skip(1) {
                    self.memory_states.remove(&id);
                    removed.push(id);
                }
            }
        }

        ConsolidationResult {
            success: true,
            memories_affected: removed.len(),
            new_memories: Vec::new(),
            removed_memories: removed,
            duration_ns: 1000,
            description: "Compressed redundant memories".into(),
        }
    }

    /// Apply decay
    pub fn apply_decay(&mut self) {
        for state in self.memory_states.values_mut() {
            state.strength *= 1.0 - self.config.decay_rate;
        }
    }

    /// Get task
    pub fn get_task(&self, id: u64) -> Option<&ConsolidationTask> {
        self.tasks.get(&id)
    }

    /// Get memory state
    pub fn get_state(&self, id: u64) -> Option<&MemoryState> {
        self.memory_states.get(&id)
    }

    /// Get pending tasks
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Get statistics
    pub fn stats(&self) -> &ConsolidationStats {
        &self.stats
    }
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self::new(ConsolidationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state(id: u64, strength: f64, tags: Vec<&str>) -> MemoryState {
        MemoryState {
            id,
            strength,
            access_count: 0,
            last_access: Timestamp::now(),
            associations: Vec::new(),
            tags: tags.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_register_memory() {
        let mut engine = ConsolidationEngine::default();

        let state = create_test_state(1, 0.5, vec!["test"]);
        engine.register_memory(state);

        assert!(engine.get_state(1).is_some());
    }

    #[test]
    fn test_strengthen() {
        let mut engine = ConsolidationEngine::default();

        let mut state = create_test_state(1, 0.5, vec!["test"]);
        state.access_count = 10;
        engine.register_memory(state);

        let task_id = engine.create_task(
            ConsolidationType::Strengthen,
            vec![1],
            Priority::Normal,
        );

        let result = engine.execute_next().unwrap();
        assert!(result.success);

        let new_state = engine.get_state(1).unwrap();
        assert!(new_state.strength > 0.5);
    }

    #[test]
    fn test_prune() {
        let mut engine = ConsolidationEngine::default();

        engine.register_memory(create_test_state(1, 0.05, vec!["weak"]));
        engine.register_memory(create_test_state(2, 0.8, vec!["strong"]));

        engine.create_task(
            ConsolidationType::Prune,
            vec![1, 2],
            Priority::Normal,
        );

        let result = engine.execute_next().unwrap();
        assert!(result.removed_memories.contains(&1));
        assert!(!result.removed_memories.contains(&2));
    }

    #[test]
    fn test_integrate() {
        let mut engine = ConsolidationEngine::default();

        engine.register_memory(create_test_state(1, 0.5, vec!["cat", "animal"]));
        engine.register_memory(create_test_state(2, 0.5, vec!["cat", "pet"]));

        engine.create_task(
            ConsolidationType::Integrate,
            vec![1, 2],
            Priority::Normal,
        );

        let result = engine.execute_next().unwrap();
        assert!(result.success);

        let state1 = engine.get_state(1).unwrap();
        assert!(state1.associations.contains(&2));
    }

    #[test]
    fn test_abstract() {
        let mut engine = ConsolidationEngine::default();

        engine.register_memory(create_test_state(1, 0.5, vec!["animal", "cat"]));
        engine.register_memory(create_test_state(2, 0.5, vec!["animal", "dog"]));

        engine.create_task(
            ConsolidationType::Abstract,
            vec![1, 2],
            Priority::Normal,
        );

        let result = engine.execute_next().unwrap();
        assert!(result.success);
        assert!(!result.new_memories.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let mut engine = ConsolidationEngine::default();

        engine.create_task(ConsolidationType::Prune, vec![], Priority::Low);
        engine.create_task(ConsolidationType::Strengthen, vec![], Priority::Urgent);
        engine.create_task(ConsolidationType::Integrate, vec![], Priority::Normal);

        // Urgent should be first
        let task_id = engine.queue.last().unwrap();
        let task = engine.get_task(*task_id).unwrap();
        assert_eq!(task.priority, Priority::Urgent);
    }
}
