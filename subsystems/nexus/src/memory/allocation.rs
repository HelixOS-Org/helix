//! Intelligent memory allocation advisor.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// ALLOCATION INTELLIGENCE
// ============================================================================

/// Intelligent memory allocation advisor
pub struct AllocationIntelligence {
    /// Allocation history
    history: Vec<AllocationRecord>,
    /// Size distribution
    size_distribution: BTreeMap<u32, u32>, // size_class -> count
    /// Lifetime analysis
    lifetime_analysis: BTreeMap<u32, Vec<u64>>, // size_class -> lifetimes
    /// Fragmentation metrics
    fragmentation: FragmentationMetrics,
    /// Max history size
    max_history: usize,
}

/// Allocation record
#[derive(Debug, Clone)]
struct AllocationRecord {
    /// Allocation ID
    id: u64,
    /// Size requested
    size: u64,
    /// Address allocated
    #[allow(dead_code)]
    address: u64,
    /// Allocation timestamp
    alloc_time: NexusTimestamp,
    /// Deallocation timestamp
    dealloc_time: Option<NexusTimestamp>,
    /// Allocation source (caller hash)
    #[allow(dead_code)]
    source: u64,
}

/// Fragmentation metrics
#[derive(Debug, Clone, Default)]
pub struct FragmentationMetrics {
    /// External fragmentation ratio
    pub external: f64,
    /// Internal fragmentation ratio
    pub internal: f64,
    /// Largest free block
    pub largest_free: u64,
    /// Total free memory
    pub total_free: u64,
    /// Number of free blocks
    pub free_blocks: u32,
}

impl AllocationIntelligence {
    /// Create new allocation intelligence
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            size_distribution: BTreeMap::new(),
            lifetime_analysis: BTreeMap::new(),
            fragmentation: FragmentationMetrics::default(),
            max_history: 10000,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, id: u64, size: u64, address: u64, source: u64) {
        let record = AllocationRecord {
            id,
            size,
            address,
            alloc_time: NexusTimestamp::now(),
            dealloc_time: None,
            source,
        };

        self.history.push(record);

        // Update size distribution
        let size_class = self.size_to_class(size);
        *self.size_distribution.entry(size_class).or_insert(0) += 1;

        // Evict old history
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Record deallocation
    pub fn record_dealloc(&mut self, id: u64) {
        // Find the record and get necessary data before any mutable operations
        let record_data = self
            .history
            .iter()
            .find(|r| r.id == id)
            .map(|r| (r.alloc_time, r.size, r.id));

        if let Some((alloc_time, size, _record_id)) = record_data {
            let now = NexusTimestamp::now();
            let size_class = self.size_to_class(size);
            let lifetime = now.duration_since(alloc_time);

            // Now update the record
            if let Some(record) = self.history.iter_mut().find(|r| r.id == id) {
                record.dealloc_time = Some(now);
            }

            // Update lifetime analysis
            self.lifetime_analysis
                .entry(size_class)
                .or_default()
                .push(lifetime);
        }
    }

    /// Convert size to size class
    fn size_to_class(&self, size: u64) -> u32 {
        // Log2-based size classes
        if size == 0 {
            0
        } else {
            (64 - size.leading_zeros()).min(31)
        }
    }

    /// Get recommended size class for new allocation
    pub fn recommend_size(&self, requested_size: u64, source: u64) -> u64 {
        // Find allocations from same source
        let source_allocs: Vec<_> = self.history.iter().filter(|r| r.source == source).collect();

        if source_allocs.is_empty() {
            // Round up to power of 2 if no history
            return requested_size.next_power_of_two();
        }

        // Calculate common sizes from this source
        let avg_size =
            source_allocs.iter().map(|r| r.size).sum::<u64>() / source_allocs.len() as u64;

        // If requested is close to average, use it
        if (requested_size as f64 / avg_size as f64 - 1.0).abs() < 0.5 {
            avg_size.next_power_of_two()
        } else {
            requested_size.next_power_of_two()
        }
    }

    /// Predict lifetime for allocation
    pub fn predict_lifetime(&self, size: u64) -> u64 {
        let size_class = self.size_to_class(size);

        if let Some(lifetimes) = self.lifetime_analysis.get(&size_class) {
            if lifetimes.len() >= 5 {
                // Return median lifetime
                let mut sorted = lifetimes.clone();
                sorted.sort();
                return sorted[sorted.len() / 2];
            }
        }

        // Default: no prediction (0 = unknown)
        0
    }

    /// Should use pooled allocation?
    pub fn should_use_pool(&self, size: u64, _source: u64) -> bool {
        let size_class = self.size_to_class(size);

        // Check frequency of this size class
        let count = self
            .size_distribution
            .get(&size_class)
            .copied()
            .unwrap_or(0);
        if count < 10 {
            return false;
        }

        // Check if short-lived allocations from this source
        if let Some(lifetimes) = self.lifetime_analysis.get(&size_class) {
            if lifetimes.len() >= 5 {
                let avg_lifetime = lifetimes.iter().sum::<u64>() / lifetimes.len() as u64;
                // Short-lived and frequent = use pool
                return avg_lifetime < 1_000_000_000 && count > 100; // < 1 second
            }
        }

        false
    }

    /// Update fragmentation metrics
    pub fn update_fragmentation(&mut self, metrics: FragmentationMetrics) {
        self.fragmentation = metrics;
    }

    /// Get fragmentation level (0.0 = none, 1.0 = severe)
    pub fn fragmentation_level(&self) -> f64 {
        (self.fragmentation.external * 0.6 + self.fragmentation.internal * 0.4).min(1.0)
    }

    /// Should compact memory?
    pub fn should_compact(&self) -> bool {
        self.fragmentation_level() > 0.5
    }

    /// Get allocation statistics
    pub fn stats(&self) -> AllocationStats {
        let total_allocs = self.history.len();
        let live_allocs = self
            .history
            .iter()
            .filter(|r| r.dealloc_time.is_none())
            .count();

        let total_allocated: u64 = self
            .history
            .iter()
            .filter(|r| r.dealloc_time.is_none())
            .map(|r| r.size)
            .sum();

        AllocationStats {
            total_allocations: total_allocs as u64,
            live_allocations: live_allocs as u64,
            total_allocated,
            size_classes: self.size_distribution.len(),
            fragmentation_level: self.fragmentation_level(),
        }
    }
}

impl Default for AllocationIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation statistics
#[derive(Debug, Clone)]
pub struct AllocationStats {
    /// Total allocations tracked
    pub total_allocations: u64,
    /// Currently live allocations
    pub live_allocations: u64,
    /// Total bytes currently allocated
    pub total_allocated: u64,
    /// Number of size classes in use
    pub size_classes: usize,
    /// Current fragmentation level
    pub fragmentation_level: f64,
}
