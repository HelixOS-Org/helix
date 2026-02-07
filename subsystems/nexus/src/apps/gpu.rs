//! # Application GPU Profiling
//!
//! Per-application GPU usage analysis:
//! - GPU compute utilization
//! - VRAM usage tracking
//! - Shader execution profiling
//! - GPU command queue analysis
//! - GPU/CPU synchronization costs
//! - Multi-GPU load balancing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// GPU RESOURCE
// ============================================================================

/// GPU device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuDeviceType {
    /// Integrated GPU
    Integrated,
    /// Discrete GPU
    Discrete,
    /// Compute accelerator
    Compute,
    /// Virtual GPU
    Virtual,
}

/// GPU engine type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuEngine {
    /// 3D rendering
    Render3D,
    /// Compute shader
    Compute,
    /// Video decode
    VideoDecode,
    /// Video encode
    VideoEncode,
    /// Copy/DMA
    Copy,
    /// Display
    Display,
}

/// GPU device profile
#[derive(Debug, Clone)]
pub struct GpuDevice {
    /// Device index
    pub index: u32,
    /// Device type
    pub device_type: GpuDeviceType,
    /// Total VRAM (bytes)
    pub total_vram: u64,
    /// Available VRAM (bytes)
    pub available_vram: u64,
    /// Compute units
    pub compute_units: u32,
    /// Clock speed (MHz)
    pub clock_mhz: u32,
    /// Temperature (Celsius × 10)
    pub temperature: u32,
    /// Power usage (mW)
    pub power_mw: u32,
    /// Per-engine utilization (0.0-1.0)
    pub engine_util: BTreeMap<u8, f64>,
}

impl GpuDevice {
    pub fn new(index: u32, device_type: GpuDeviceType, total_vram: u64) -> Self {
        Self {
            index,
            device_type,
            total_vram,
            available_vram: total_vram,
            compute_units: 0,
            clock_mhz: 0,
            temperature: 0,
            power_mw: 0,
            engine_util: BTreeMap::new(),
        }
    }

    /// VRAM utilization
    pub fn vram_utilization(&self) -> f64 {
        if self.total_vram == 0 {
            return 0.0;
        }
        1.0 - (self.available_vram as f64 / self.total_vram as f64)
    }

    /// Overall utilization
    pub fn overall_utilization(&self) -> f64 {
        if self.engine_util.is_empty() {
            return 0.0;
        }
        self.engine_util.values().sum::<f64>() / self.engine_util.len() as f64
    }

    /// Update engine utilization
    pub fn update_engine(&mut self, engine: GpuEngine, utilization: f64) {
        self.engine_util.insert(engine as u8, utilization);
    }
}

// ============================================================================
// GPU ALLOCATION
// ============================================================================

/// GPU memory allocation
#[derive(Debug, Clone)]
pub struct GpuAllocation {
    /// Allocation ID
    pub id: u64,
    /// Process
    pub pid: u64,
    /// GPU device
    pub device: u32,
    /// Size (bytes)
    pub size: u64,
    /// Allocation type
    pub alloc_type: GpuAllocType,
    /// Timestamp
    pub timestamp: u64,
}

/// GPU allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuAllocType {
    /// Vertex buffer
    VertexBuffer,
    /// Index buffer
    IndexBuffer,
    /// Texture
    Texture,
    /// Render target
    RenderTarget,
    /// Compute buffer
    ComputeBuffer,
    /// Command buffer
    CommandBuffer,
    /// Shader
    Shader,
    /// Other
    Other,
}

// ============================================================================
// GPU PROCESS PROFILE
// ============================================================================

/// Per-process GPU profile
#[derive(Debug, Clone)]
pub struct ProcessGpuProfile {
    /// Process ID
    pub pid: u64,
    /// VRAM usage per device (bytes)
    pub vram_usage: BTreeMap<u32, u64>,
    /// Engine usage
    pub engine_time: BTreeMap<u8, u64>,
    /// GPU calls count
    pub gpu_calls: u64,
    /// GPU wait time (ns)
    pub gpu_wait_ns: u64,
    /// GPU compute time (ns)
    pub gpu_compute_ns: u64,
    /// Allocations
    pub allocation_count: u64,
    /// Peak VRAM
    pub peak_vram: u64,
    /// Sync stalls (CPU waiting for GPU)
    pub sync_stalls: u64,
}

impl ProcessGpuProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            vram_usage: BTreeMap::new(),
            engine_time: BTreeMap::new(),
            gpu_calls: 0,
            gpu_wait_ns: 0,
            gpu_compute_ns: 0,
            allocation_count: 0,
            peak_vram: 0,
            sync_stalls: 0,
        }
    }

    /// Record GPU call
    pub fn record_call(&mut self, compute_ns: u64, wait_ns: u64) {
        self.gpu_calls += 1;
        self.gpu_compute_ns += compute_ns;
        self.gpu_wait_ns += wait_ns;
    }

    /// Record allocation
    pub fn record_alloc(&mut self, device: u32, size: u64) {
        *self.vram_usage.entry(device).or_insert(0) += size;
        self.allocation_count += 1;
        let total_vram: u64 = self.vram_usage.values().sum();
        if total_vram > self.peak_vram {
            self.peak_vram = total_vram;
        }
    }

    /// Record free
    pub fn record_free(&mut self, device: u32, size: u64) {
        if let Some(usage) = self.vram_usage.get_mut(&device) {
            *usage = usage.saturating_sub(size);
        }
    }

    /// Total VRAM usage
    pub fn total_vram(&self) -> u64 {
        self.vram_usage.values().sum()
    }

    /// GPU efficiency (compute / (compute + wait))
    pub fn gpu_efficiency(&self) -> f64 {
        let total = self.gpu_compute_ns + self.gpu_wait_ns;
        if total == 0 {
            return 1.0;
        }
        self.gpu_compute_ns as f64 / total as f64
    }
}

// ============================================================================
// GPU ANALYZER
// ============================================================================

/// GPU analyzer stats
#[derive(Debug, Clone, Default)]
pub struct AppGpuStats {
    /// Tracked devices
    pub device_count: usize,
    /// Tracked processes
    pub process_count: usize,
    /// Total VRAM used (bytes)
    pub total_vram_used: u64,
    /// Average GPU utilization
    pub avg_utilization: f64,
    /// Total sync stalls
    pub total_sync_stalls: u64,
}

/// Application GPU analyzer
pub struct AppGpuAnalyzer {
    /// GPU devices
    devices: BTreeMap<u32, GpuDevice>,
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessGpuProfile>,
    /// Allocations
    allocations: BTreeMap<u64, GpuAllocation>,
    /// Next alloc ID
    next_alloc_id: u64,
    /// Stats
    stats: AppGpuStats,
}

impl AppGpuAnalyzer {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            profiles: BTreeMap::new(),
            allocations: BTreeMap::new(),
            next_alloc_id: 1,
            stats: AppGpuStats::default(),
        }
    }

    /// Register GPU device
    pub fn register_device(&mut self, device: GpuDevice) {
        self.devices.insert(device.index, device);
        self.stats.device_count = self.devices.len();
    }

    /// Record GPU call
    pub fn record_call(&mut self, pid: u64, compute_ns: u64, wait_ns: u64) {
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessGpuProfile::new(pid));
        profile.record_call(compute_ns, wait_ns);
        self.stats.process_count = self.profiles.len();
    }

    /// Allocate VRAM
    pub fn allocate_vram(
        &mut self,
        pid: u64,
        device: u32,
        size: u64,
        alloc_type: GpuAllocType,
        now: u64,
    ) -> u64 {
        let id = self.next_alloc_id;
        self.next_alloc_id += 1;

        // Update device
        if let Some(dev) = self.devices.get_mut(&device) {
            dev.available_vram = dev.available_vram.saturating_sub(size);
        }

        // Update profile
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessGpuProfile::new(pid));
        profile.record_alloc(device, size);

        self.allocations.insert(
            id,
            GpuAllocation {
                id,
                pid,
                device,
                size,
                alloc_type,
                timestamp: now,
            },
        );

        self.update_stats();
        id
    }

    /// Free VRAM
    pub fn free_vram(&mut self, alloc_id: u64) {
        if let Some(alloc) = self.allocations.remove(&alloc_id) {
            if let Some(dev) = self.devices.get_mut(&alloc.device) {
                dev.available_vram = (dev.available_vram + alloc.size).min(dev.total_vram);
            }
            if let Some(profile) = self.profiles.get_mut(&alloc.pid) {
                profile.record_free(alloc.device, alloc.size);
            }
        }
        self.update_stats();
    }

    /// Record sync stall
    pub fn record_sync_stall(&mut self, pid: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.sync_stalls += 1;
        }
        self.stats.total_sync_stalls += 1;
    }

    fn update_stats(&mut self) {
        self.stats.total_vram_used = self
            .devices
            .values()
            .map(|d| d.total_vram.saturating_sub(d.available_vram))
            .sum();
        if self.devices.is_empty() {
            self.stats.avg_utilization = 0.0;
        } else {
            self.stats.avg_utilization = self
                .devices
                .values()
                .map(|d| d.overall_utilization())
                .sum::<f64>()
                / self.devices.len() as f64;
        }
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessGpuProfile> {
        self.profiles.get(&pid)
    }

    /// Get device
    pub fn device(&self, index: u32) -> Option<&GpuDevice> {
        self.devices.get(&index)
    }

    /// Stats
    pub fn stats(&self) -> &AppGpuStats {
        &self.stats
    }
}
