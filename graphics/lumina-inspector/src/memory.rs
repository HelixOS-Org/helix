//! # GPU Memory Profiler
//!
//! Revolutionary memory analysis with:
//! - Allocation tracking
//! - Leak detection
//! - Fragmentation analysis
//! - Memory timeline

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

/// Memory profiler
pub struct MemoryProfiler {
    enabled: bool,
    allocations: BTreeMap<u64, AllocationInfo>,
    heaps: Vec<HeapInfo>,
    timeline: Vec<MemoryEvent>,
    stats: MemoryStats,
}

impl MemoryProfiler {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            allocations: BTreeMap::new(),
            heaps: Vec::new(),
            timeline: Vec::new(),
            stats: MemoryStats::default(),
        }
    }
    
    /// Record an allocation
    pub fn record_alloc(&mut self, id: u64, info: AllocationInfo) {
        if !self.enabled {
            return;
        }
        
        self.stats.total_allocated += info.size;
        self.stats.allocation_count += 1;
        self.stats.peak_allocated = self.stats.peak_allocated.max(self.stats.total_allocated);
        
        self.timeline.push(MemoryEvent {
            timestamp: get_timestamp(),
            event_type: MemoryEventType::Allocate,
            allocation_id: id,
            size: info.size,
            heap_index: info.heap_index,
        });
        
        self.allocations.insert(id, info);
    }
    
    /// Record a deallocation
    pub fn record_free(&mut self, id: u64) {
        if !self.enabled {
            return;
        }
        
        if let Some(info) = self.allocations.remove(&id) {
            self.stats.total_allocated -= info.size;
            self.stats.free_count += 1;
            
            self.timeline.push(MemoryEvent {
                timestamp: get_timestamp(),
                event_type: MemoryEventType::Free,
                allocation_id: id,
                size: info.size,
                heap_index: info.heap_index,
            });
        }
    }
    
    /// Set heap information
    pub fn set_heaps(&mut self, heaps: Vec<HeapInfo>) {
        self.heaps = heaps;
    }
    
    /// Get current snapshot
    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            total_allocated: self.stats.total_allocated,
            peak_allocated: self.stats.peak_allocated,
            allocation_count: self.allocations.len(),
            heaps: self.heaps.clone(),
            largest_allocations: self.get_largest_allocations(10),
        }
    }
    
    /// Analyze memory usage
    pub fn analyze(&self) -> MemoryAnalysis {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        
        // Check for fragmentation
        for (heap_idx, heap) in self.heaps.iter().enumerate() {
            let fragmentation = calculate_fragmentation(heap);
            if fragmentation > 0.3 {
                issues.push(MemoryIssue {
                    issue_type: MemoryIssueType::Fragmentation,
                    heap_index: Some(heap_idx),
                    description: alloc::format!(
                        "Heap {} has {:.1}% fragmentation",
                        heap_idx,
                        fragmentation * 100.0
                    ),
                    severity: fragmentation,
                });
            }
        }
        
        // Check for potential leaks
        let old_allocations: Vec<_> = self.allocations.values()
            .filter(|a| a.age_frames > 1000)
            .collect();
        
        if !old_allocations.is_empty() {
            issues.push(MemoryIssue {
                issue_type: MemoryIssueType::PotentialLeak,
                heap_index: None,
                description: alloc::format!(
                    "{} allocations are over 1000 frames old",
                    old_allocations.len()
                ),
                severity: 0.8,
            });
        }
        
        // Check for small allocations
        let small_count = self.allocations.values()
            .filter(|a| a.size < 256)
            .count();
        
        if small_count > 100 {
            suggestions.push(String::from(
                "Consider pooling small allocations to reduce overhead"
            ));
        }
        
        // Check heap utilization
        for (idx, heap) in self.heaps.iter().enumerate() {
            let utilization = heap.used as f64 / heap.size as f64;
            if utilization > 0.9 {
                issues.push(MemoryIssue {
                    issue_type: MemoryIssueType::HighUtilization,
                    heap_index: Some(idx),
                    description: alloc::format!(
                        "Heap {} is {:.1}% utilized",
                        idx,
                        utilization * 100.0
                    ),
                    severity: utilization as f32,
                });
            }
        }
        
        MemoryAnalysis {
            issues,
            suggestions,
            fragmentation: self.heaps.iter()
                .map(calculate_fragmentation)
                .sum::<f32>() / self.heaps.len().max(1) as f32,
            leak_score: calculate_leak_score(&self.allocations),
        }
    }
    
    /// Get timeline for visualization
    pub fn get_timeline(&self) -> &[MemoryEvent] {
        &self.timeline
    }
    
    /// Clear timeline
    pub fn clear_timeline(&mut self) {
        self.timeline.clear();
    }
    
    /// Get largest allocations
    fn get_largest_allocations(&self, count: usize) -> Vec<(u64, AllocationInfo)> {
        let mut allocs: Vec<_> = self.allocations.iter()
            .map(|(id, info)| (*id, info.clone()))
            .collect();
        allocs.sort_by(|a, b| b.1.size.cmp(&a.1.size));
        allocs.truncate(count);
        allocs
    }
    
    /// Find allocations by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<(u64, &AllocationInfo)> {
        self.allocations.iter()
            .filter(|(_, info)| info.tag.as_deref() == Some(tag))
            .map(|(id, info)| (*id, info))
            .collect()
    }
    
    /// Get stats
    pub fn stats(&self) -> &MemoryStats {
        &self.stats
    }
}

/// Allocation information
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub size: u64,
    pub alignment: u64,
    pub heap_index: usize,
    pub memory_type: GpuMemoryType,
    pub usage: AllocationUsage,
    pub tag: Option<String>,
    pub callstack: Option<Vec<String>>,
    pub creation_frame: u64,
    pub age_frames: u64,
}

/// GPU memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMemoryType {
    DeviceLocal,
    HostVisible,
    HostCoherent,
    HostCached,
    LazilyAllocated,
}

/// Allocation usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationUsage {
    Buffer,
    Image,
    RenderTarget,
    DepthStencil,
    Staging,
    Readback,
    AccelerationStructure,
    Unknown,
}

/// Heap information
#[derive(Debug, Clone)]
pub struct HeapInfo {
    pub size: u64,
    pub used: u64,
    pub heap_flags: HeapFlags,
    pub memory_type_bits: u32,
    pub blocks: Vec<MemoryBlock>,
}

/// Heap flags
#[derive(Debug, Clone, Copy)]
pub struct HeapFlags {
    pub device_local: bool,
    pub multi_instance: bool,
}

/// Memory block in a heap
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub offset: u64,
    pub size: u64,
    pub is_free: bool,
    pub allocation_id: Option<u64>,
}

/// Memory event for timeline
#[derive(Debug, Clone)]
pub struct MemoryEvent {
    pub timestamp: u64,
    pub event_type: MemoryEventType,
    pub allocation_id: u64,
    pub size: u64,
    pub heap_index: usize,
}

/// Memory event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryEventType {
    Allocate,
    Free,
    Defragment,
    MapMemory,
    UnmapMemory,
    Flush,
    Invalidate,
}

/// Memory snapshot
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub total_allocated: u64,
    pub peak_allocated: u64,
    pub allocation_count: usize,
    pub heaps: Vec<HeapInfo>,
    pub largest_allocations: Vec<(u64, AllocationInfo)>,
}

impl Default for MemorySnapshot {
    fn default() -> Self {
        Self {
            total_allocated: 0,
            peak_allocated: 0,
            allocation_count: 0,
            heaps: Vec::new(),
            largest_allocations: Vec::new(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: u64,
    pub peak_allocated: u64,
    pub allocation_count: u64,
    pub free_count: u64,
}

/// Memory analysis result
#[derive(Debug, Clone)]
pub struct MemoryAnalysis {
    pub issues: Vec<MemoryIssue>,
    pub suggestions: Vec<String>,
    pub fragmentation: f32,
    pub leak_score: f32,
}

/// Memory issue
#[derive(Debug, Clone)]
pub struct MemoryIssue {
    pub issue_type: MemoryIssueType,
    pub heap_index: Option<usize>,
    pub description: String,
    pub severity: f32,
}

/// Memory issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryIssueType {
    Fragmentation,
    PotentialLeak,
    HighUtilization,
    SmallAllocations,
    InvalidAccess,
    MemoryThrashing,
}

fn calculate_fragmentation(heap: &HeapInfo) -> f32 {
    if heap.blocks.is_empty() {
        return 0.0;
    }
    
    let free_blocks: Vec<_> = heap.blocks.iter()
        .filter(|b| b.is_free)
        .collect();
    
    if free_blocks.is_empty() {
        return 0.0;
    }
    
    let total_free: u64 = free_blocks.iter().map(|b| b.size).sum();
    let largest_free = free_blocks.iter().map(|b| b.size).max().unwrap_or(0);
    
    if total_free == 0 {
        return 0.0;
    }
    
    1.0 - (largest_free as f32 / total_free as f32)
}

fn calculate_leak_score(allocations: &BTreeMap<u64, AllocationInfo>) -> f32 {
    if allocations.is_empty() {
        return 0.0;
    }
    
    let old_count = allocations.values()
        .filter(|a| a.age_frames > 500)
        .count();
    
    old_count as f32 / allocations.len() as f32
}

fn get_timestamp() -> u64 {
    0
}

/// Memory budget manager
pub struct MemoryBudget {
    limits: BTreeMap<AllocationUsage, u64>,
    current: BTreeMap<AllocationUsage, u64>,
}

impl MemoryBudget {
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            current: BTreeMap::new(),
        }
    }
    
    /// Set budget limit for a usage type
    pub fn set_limit(&mut self, usage: AllocationUsage, limit: u64) {
        self.limits.insert(usage, limit);
    }
    
    /// Check if allocation would exceed budget
    pub fn can_allocate(&self, usage: AllocationUsage, size: u64) -> bool {
        if let Some(limit) = self.limits.get(&usage) {
            let current = self.current.get(&usage).copied().unwrap_or(0);
            current + size <= *limit
        } else {
            true
        }
    }
    
    /// Record allocation
    pub fn record_alloc(&mut self, usage: AllocationUsage, size: u64) {
        *self.current.entry(usage).or_insert(0) += size;
    }
    
    /// Record deallocation
    pub fn record_free(&mut self, usage: AllocationUsage, size: u64) {
        if let Some(current) = self.current.get_mut(&usage) {
            *current = current.saturating_sub(size);
        }
    }
    
    /// Get utilization for a usage type
    pub fn utilization(&self, usage: AllocationUsage) -> Option<f32> {
        let limit = self.limits.get(&usage)?;
        let current = self.current.get(&usage).copied().unwrap_or(0);
        Some(current as f32 / *limit as f32)
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// Virtual memory system for sparse resources
pub struct VirtualMemoryTracker {
    pages: BTreeMap<u64, VirtualPage>,
    page_size: u64,
    total_virtual: u64,
    total_committed: u64,
}

impl VirtualMemoryTracker {
    pub fn new(page_size: u64) -> Self {
        Self {
            pages: BTreeMap::new(),
            page_size,
            total_virtual: 0,
            total_committed: 0,
        }
    }
    
    /// Reserve virtual memory
    pub fn reserve(&mut self, start_page: u64, count: u64) {
        for i in 0..count {
            let page_id = start_page + i;
            self.pages.insert(page_id, VirtualPage {
                page_id,
                state: PageState::Reserved,
                physical_offset: None,
            });
        }
        self.total_virtual += count * self.page_size;
    }
    
    /// Commit a page
    pub fn commit(&mut self, page_id: u64, physical_offset: u64) {
        if let Some(page) = self.pages.get_mut(&page_id) {
            if page.state == PageState::Reserved {
                page.state = PageState::Committed;
                page.physical_offset = Some(physical_offset);
                self.total_committed += self.page_size;
            }
        }
    }
    
    /// Evict a page
    pub fn evict(&mut self, page_id: u64) {
        if let Some(page) = self.pages.get_mut(&page_id) {
            if page.state == PageState::Committed {
                page.state = PageState::Reserved;
                page.physical_offset = None;
                self.total_committed -= self.page_size;
            }
        }
    }
    
    /// Get commitment ratio
    pub fn commitment_ratio(&self) -> f32 {
        if self.total_virtual == 0 {
            return 0.0;
        }
        self.total_committed as f32 / self.total_virtual as f32
    }
}

/// Virtual memory page
#[derive(Debug, Clone)]
pub struct VirtualPage {
    pub page_id: u64,
    pub state: PageState,
    pub physical_offset: Option<u64>,
}

/// Page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Reserved,
    Committed,
    Mapped,
}

/// Memory access validator
pub struct AccessValidator {
    valid_ranges: Vec<(u64, u64)>,
    access_log: Vec<AccessViolation>,
}

impl AccessValidator {
    pub fn new() -> Self {
        Self {
            valid_ranges: Vec::new(),
            access_log: Vec::new(),
        }
    }
    
    /// Add valid memory range
    pub fn add_valid_range(&mut self, start: u64, size: u64) {
        self.valid_ranges.push((start, start + size));
    }
    
    /// Remove valid memory range
    pub fn remove_valid_range(&mut self, start: u64) {
        self.valid_ranges.retain(|(s, _)| *s != start);
    }
    
    /// Validate an access
    pub fn validate(&mut self, address: u64, size: u64) -> bool {
        let end = address + size;
        
        for &(start, range_end) in &self.valid_ranges {
            if address >= start && end <= range_end {
                return true;
            }
        }
        
        self.access_log.push(AccessViolation {
            address,
            size,
            timestamp: get_timestamp(),
        });
        
        false
    }
    
    /// Get access violations
    pub fn violations(&self) -> &[AccessViolation] {
        &self.access_log
    }
}

impl Default for AccessValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory access violation
#[derive(Debug, Clone)]
pub struct AccessViolation {
    pub address: u64,
    pub size: u64,
    pub timestamp: u64,
}
