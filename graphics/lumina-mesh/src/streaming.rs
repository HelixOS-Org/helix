//! Geometry Streaming System
//!
//! Asynchronous streaming of geometry data for virtual geometry system.
//! Manages page loading, eviction, and memory budget.

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::virtual_geometry::{PageRequest, PageState, StreamingPriority, VirtualPage};

// ============================================================================
// Page ID
// ============================================================================

/// Unique identifier for a geometry page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId {
    /// Mesh index.
    pub mesh: u32,
    /// Page index within mesh.
    pub page: u32,
}

impl PageId {
    /// Create a new page ID.
    pub fn new(mesh: u32, page: u32) -> Self {
        Self { mesh, page }
    }

    /// Create packed u64 for fast comparison.
    pub fn packed(&self) -> u64 {
        ((self.mesh as u64) << 32) | (self.page as u64)
    }

    /// Create from packed u64.
    pub fn from_packed(packed: u64) -> Self {
        Self {
            mesh: (packed >> 32) as u32,
            page: packed as u32,
        }
    }
}

// ============================================================================
// Geometry Page
// ============================================================================

/// A page of geometry data.
#[derive(Debug, Clone)]
pub struct GeometryPage {
    /// Page ID.
    pub id: PageId,
    /// State.
    pub state: PageState,
    /// Data (if loaded).
    pub data: Option<PageData>,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Priority for streaming.
    pub priority: f32,
    /// Last frame this page was requested.
    pub last_requested_frame: u64,
    /// Last frame this page was used.
    pub last_used_frame: u64,
    /// Number of meshlets in this page.
    pub meshlet_count: u32,
    /// GPU buffer offset (if resident).
    pub gpu_offset: u64,
}

/// Page data container.
#[derive(Debug, Clone)]
pub struct PageData {
    /// Meshlet definitions.
    pub meshlets: Vec<u8>,
    /// Meshlet bounds.
    pub bounds: Vec<u8>,
    /// Vertex indices.
    pub vertex_indices: Vec<u8>,
    /// Primitive indices.
    pub primitive_indices: Vec<u8>,
}

impl PageData {
    /// Total size in bytes.
    pub fn size(&self) -> usize {
        self.meshlets.len()
            + self.bounds.len()
            + self.vertex_indices.len()
            + self.primitive_indices.len()
    }
}

impl GeometryPage {
    /// Create a new page.
    pub fn new(id: PageId, size_bytes: u64, meshlet_count: u32) -> Self {
        Self {
            id,
            state: PageState::NotLoaded,
            data: None,
            size_bytes,
            priority: 0.0,
            last_requested_frame: 0,
            last_used_frame: 0,
            meshlet_count,
            gpu_offset: 0,
        }
    }

    /// Check if page is resident.
    pub fn is_resident(&self) -> bool {
        self.state == PageState::Resident
    }

    /// Check if page is loading.
    pub fn is_loading(&self) -> bool {
        self.state == PageState::Loading
    }

    /// Calculate eviction priority (higher = more likely to evict).
    pub fn eviction_priority(&self, current_frame: u64) -> f32 {
        let age = (current_frame - self.last_used_frame) as f32;
        age - self.priority * 100.0
    }
}

// ============================================================================
// Streaming Configuration
// ============================================================================

/// Configuration for geometry streaming.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Total memory budget (bytes).
    pub memory_budget: u64,
    /// Reserve memory for essential pages (bytes).
    pub reserved_memory: u64,
    /// Maximum pages to load per frame.
    pub max_loads_per_frame: u32,
    /// Maximum pages to evict per frame.
    pub max_evictions_per_frame: u32,
    /// Number of frames before a page can be evicted.
    pub eviction_delay_frames: u64,
    /// Target memory usage (fraction of budget).
    pub target_usage: f32,
    /// Prefetch distance (in priority units).
    pub prefetch_distance: f32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            memory_budget: 512 * 1024 * 1024,  // 512 MB
            reserved_memory: 64 * 1024 * 1024, // 64 MB
            max_loads_per_frame: 4,
            max_evictions_per_frame: 8,
            eviction_delay_frames: 10,
            target_usage: 0.9,
            prefetch_distance: 2.0,
        }
    }
}

// ============================================================================
// Streaming Statistics
// ============================================================================

/// Statistics for geometry streaming.
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    /// Total pages.
    pub total_pages: u32,
    /// Resident pages.
    pub resident_pages: u32,
    /// Loading pages.
    pub loading_pages: u32,
    /// Total memory used (bytes).
    pub memory_used: u64,
    /// Memory budget (bytes).
    pub memory_budget: u64,
    /// Pages loaded this frame.
    pub pages_loaded: u32,
    /// Pages evicted this frame.
    pub pages_evicted: u32,
    /// Cache hit rate.
    pub hit_rate: f32,
    /// Pending requests.
    pub pending_requests: u32,
    /// Bytes loaded this frame.
    pub bytes_loaded: u64,
    /// Bytes evicted this frame.
    pub bytes_evicted: u64,
}

impl StreamingStats {
    /// Calculate memory usage percentage.
    pub fn usage_percent(&self) -> f32 {
        if self.memory_budget > 0 {
            (self.memory_used as f32 / self.memory_budget as f32) * 100.0
        } else {
            0.0
        }
    }

    /// Check if under budget.
    pub fn is_under_budget(&self) -> bool {
        self.memory_used < self.memory_budget
    }
}

// ============================================================================
// Load Request
// ============================================================================

/// Request to load a page.
#[derive(Debug, Clone)]
pub struct LoadRequest {
    /// Page ID.
    pub page_id: PageId,
    /// Priority.
    pub priority: StreamingPriority,
    /// Screen error.
    pub screen_error: f32,
    /// Frame requested.
    pub frame: u64,
}

impl LoadRequest {
    /// Calculate sort priority (lower = more urgent).
    pub fn sort_priority(&self) -> i32 {
        (self.priority as i32) * 1000 - (self.screen_error * 100.0) as i32
    }
}

// ============================================================================
// Geometry Cache
// ============================================================================

/// LRU cache for geometry pages.
pub struct GeometryCache {
    /// Cached pages by ID.
    pages: BTreeMap<u64, GeometryPage>,
    /// LRU order (front = oldest).
    lru_order: VecDeque<u64>,
    /// Current memory usage.
    memory_used: AtomicU64,
    /// Memory budget.
    memory_budget: u64,
}

impl GeometryCache {
    /// Create a new cache.
    pub fn new(memory_budget: u64) -> Self {
        Self {
            pages: BTreeMap::new(),
            lru_order: VecDeque::new(),
            memory_used: AtomicU64::new(0),
            memory_budget,
        }
    }

    /// Get a page.
    pub fn get(&mut self, id: PageId) -> Option<&GeometryPage> {
        let packed = id.packed();
        if self.pages.contains_key(&packed) {
            // Move to end of LRU
            self.lru_order.retain(|&x| x != packed);
            self.lru_order.push_back(packed);
            self.pages.get(&packed)
        } else {
            None
        }
    }

    /// Get mutable page.
    pub fn get_mut(&mut self, id: PageId) -> Option<&mut GeometryPage> {
        let packed = id.packed();
        if self.pages.contains_key(&packed) {
            self.lru_order.retain(|&x| x != packed);
            self.lru_order.push_back(packed);
            self.pages.get_mut(&packed)
        } else {
            None
        }
    }

    /// Insert a page.
    pub fn insert(&mut self, page: GeometryPage) {
        let packed = page.id.packed();
        let size = page.size_bytes;

        if let Some(old) = self.pages.insert(packed, page) {
            self.memory_used
                .fetch_sub(old.size_bytes, Ordering::Relaxed);
        } else {
            self.lru_order.push_back(packed);
        }

        self.memory_used.fetch_add(size, Ordering::Relaxed);
    }

    /// Remove a page.
    pub fn remove(&mut self, id: PageId) -> Option<GeometryPage> {
        let packed = id.packed();
        if let Some(page) = self.pages.remove(&packed) {
            self.lru_order.retain(|&x| x != packed);
            self.memory_used
                .fetch_sub(page.size_bytes, Ordering::Relaxed);
            Some(page)
        } else {
            None
        }
    }

    /// Get LRU page ID (oldest).
    pub fn get_lru(&self) -> Option<PageId> {
        self.lru_order
            .front()
            .map(|&packed| PageId::from_packed(packed))
    }

    /// Get memory used.
    pub fn memory_used(&self) -> u64 {
        self.memory_used.load(Ordering::Relaxed)
    }

    /// Check if over budget.
    pub fn is_over_budget(&self) -> bool {
        self.memory_used() > self.memory_budget
    }

    /// Get page count.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Clear all pages.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.lru_order.clear();
        self.memory_used.store(0, Ordering::Relaxed);
    }

    /// Iterate over pages.
    pub fn iter(&self) -> impl Iterator<Item = &GeometryPage> {
        self.pages.values()
    }
}

// ============================================================================
// Geometry Streamer
// ============================================================================

/// Main geometry streaming manager.
pub struct GeometryStreamer {
    /// Configuration.
    config: StreamingConfig,
    /// Page cache.
    cache: GeometryCache,
    /// Pending load requests.
    pending_loads: Vec<LoadRequest>,
    /// Pages currently loading.
    loading: BTreeMap<u64, u64>, // page_id -> frame started
    /// Current frame.
    current_frame: AtomicU64,
    /// Statistics.
    stats: StreamingStats,
    /// Total hits.
    total_hits: AtomicU64,
    /// Total requests.
    total_requests: AtomicU64,
}

impl GeometryStreamer {
    /// Create a new streamer.
    pub fn new(config: StreamingConfig) -> Self {
        let memory_budget = config.memory_budget;
        Self {
            config,
            cache: GeometryCache::new(memory_budget),
            pending_loads: Vec::new(),
            loading: BTreeMap::new(),
            current_frame: AtomicU64::new(0),
            stats: StreamingStats::default(),
            total_hits: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
        }
    }

    /// Get configuration.
    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }

    /// Get mutable configuration.
    pub fn config_mut(&mut self) -> &mut StreamingConfig {
        &mut self.config
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    /// Begin frame.
    pub fn begin_frame(&mut self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_loaded = 0;
        self.stats.pages_evicted = 0;
        self.stats.bytes_loaded = 0;
        self.stats.bytes_evicted = 0;
    }

    /// Request a page.
    pub fn request_page(&mut self, request: PageRequest, mesh_id: u32) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let page_id = PageId::new(mesh_id, request.page_id);
        let frame = self.current_frame();

        // Check if already resident
        if let Some(page) = self.cache.get_mut(page_id) {
            if page.is_resident() {
                self.total_hits.fetch_add(1, Ordering::Relaxed);
                page.last_used_frame = frame;
                page.priority = request.screen_error;
                return;
            }
        }

        // Check if already loading
        if self.loading.contains_key(&page_id.packed()) {
            return;
        }

        // Add to pending loads
        self.pending_loads.push(LoadRequest {
            page_id,
            priority: request.priority,
            screen_error: request.screen_error,
            frame,
        });
    }

    /// Process streaming for this frame.
    pub fn update(&mut self) {
        let frame = self.current_frame();

        // Sort pending loads by priority
        self.pending_loads.sort_by_key(|r| r.sort_priority());

        // Evict pages if needed
        self.evict_pages(frame);

        // Start new loads
        self.start_loads(frame);

        // Check for completed loads (simulated - in real system would be async)
        self.check_completed_loads();

        // Update statistics
        self.update_stats();
    }

    /// Evict pages to make room.
    fn evict_pages(&mut self, frame: u64) {
        let target_memory =
            (self.config.memory_budget as f64 * self.config.target_usage as f64) as u64;
        let mut evicted = 0u32;

        while self.cache.memory_used() > target_memory
            && evicted < self.config.max_evictions_per_frame
        {
            // Find best eviction candidate
            let mut best_candidate: Option<PageId> = None;
            let mut best_priority = f32::MIN;

            for page in self.cache.iter() {
                if frame - page.last_used_frame < self.config.eviction_delay_frames {
                    continue;
                }

                let priority = page.eviction_priority(frame);
                if priority > best_priority {
                    best_priority = priority;
                    best_candidate = Some(page.id);
                }
            }

            if let Some(id) = best_candidate {
                if let Some(page) = self.cache.remove(id) {
                    self.stats.bytes_evicted += page.size_bytes;
                    evicted += 1;
                }
            } else {
                break;
            }
        }

        self.stats.pages_evicted = evicted;
    }

    /// Start new page loads.
    fn start_loads(&mut self, frame: u64) {
        let available_memory = self
            .config
            .memory_budget
            .saturating_sub(self.cache.memory_used());
        let mut loads_started = 0u32;
        let mut memory_allocated = 0u64;

        while loads_started < self.config.max_loads_per_frame {
            if let Some(request) = self.pending_loads.pop() {
                // Estimate page size (would come from metadata in real system)
                let estimated_size = 64 * 1024u64; // 64KB estimate

                if memory_allocated + estimated_size > available_memory {
                    self.pending_loads.push(request);
                    break;
                }

                // Mark as loading
                self.loading.insert(request.page_id.packed(), frame);

                // Create page entry
                let mut page = GeometryPage::new(request.page_id, estimated_size, 0);
                page.state = PageState::Loading;
                page.last_requested_frame = frame;
                page.priority = request.screen_error;

                self.cache.insert(page);

                loads_started += 1;
                memory_allocated += estimated_size;
            } else {
                break;
            }
        }
    }

    /// Check for completed loads.
    fn check_completed_loads(&mut self) {
        let frame = self.current_frame();
        let mut completed = Vec::new();

        // Simulate instant loading for now
        // In real system, this would check async IO completion
        for (&packed, &_start_frame) in &self.loading {
            completed.push(PageId::from_packed(packed));
        }

        for id in completed {
            self.loading.remove(&id.packed());

            if let Some(page) = self.cache.get_mut(id) {
                page.state = PageState::Resident;
                page.last_used_frame = frame;

                // Simulate data loading
                page.data = Some(PageData {
                    meshlets: Vec::new(),
                    bounds: Vec::new(),
                    vertex_indices: Vec::new(),
                    primitive_indices: Vec::new(),
                });

                self.stats.pages_loaded += 1;
                self.stats.bytes_loaded += page.size_bytes;
            }
        }
    }

    /// Update statistics.
    fn update_stats(&mut self) {
        let mut resident = 0u32;
        let mut loading = 0u32;

        for page in self.cache.iter() {
            match page.state {
                PageState::Resident => resident += 1,
                PageState::Loading => loading += 1,
                _ => {},
            }
        }

        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let total_hits = self.total_hits.load(Ordering::Relaxed);

        self.stats.total_pages = self.cache.page_count() as u32;
        self.stats.resident_pages = resident;
        self.stats.loading_pages = loading;
        self.stats.memory_used = self.cache.memory_used();
        self.stats.memory_budget = self.config.memory_budget;
        self.stats.pending_requests = self.pending_loads.len() as u32;
        self.stats.hit_rate = if total_requests > 0 {
            total_hits as f32 / total_requests as f32
        } else {
            1.0
        };
    }

    /// Get statistics.
    pub fn stats(&self) -> &StreamingStats {
        &self.stats
    }

    /// Check if a page is resident.
    pub fn is_page_resident(&mut self, mesh_id: u32, page_id: u32) -> bool {
        let id = PageId::new(mesh_id, page_id);
        self.cache.get(id).map(|p| p.is_resident()).unwrap_or(false)
    }

    /// Get a resident page.
    pub fn get_page(&mut self, mesh_id: u32, page_id: u32) -> Option<&GeometryPage> {
        let id = PageId::new(mesh_id, page_id);
        let page = self.cache.get(id)?;
        if page.is_resident() {
            Some(page)
        } else {
            None
        }
    }

    /// Clear all pages.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.pending_loads.clear();
        self.loading.clear();
        self.stats = StreamingStats::default();
    }

    /// Get memory usage.
    pub fn memory_used(&self) -> u64 {
        self.cache.memory_used()
    }

    /// Get available memory.
    pub fn memory_available(&self) -> u64 {
        self.config
            .memory_budget
            .saturating_sub(self.cache.memory_used())
    }
}

impl Default for GeometryStreamer {
    fn default() -> Self {
        Self::new(StreamingConfig::default())
    }
}
