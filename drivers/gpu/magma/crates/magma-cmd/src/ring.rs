//! # Command Ring
//!
//! GPU command ring buffer for submission scheduling.

use core::sync::atomic::{AtomicU64, Ordering};

use magma_core::{Error, Result, GpuAddr, ByteSize};

// =============================================================================
// RING CONFIGURATION
// =============================================================================

/// Command ring configuration
#[derive(Debug, Clone)]
pub struct RingConfig {
    /// Ring size in bytes
    pub size: ByteSize,
    /// Entry size (typically 4 bytes for GPU methods)
    pub entry_size: u32,
    /// Maximum pending submissions
    pub max_pending: u32,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            size: ByteSize::from_mib(1),
            entry_size: 4,
            max_pending: 1024,
        }
    }
}

// =============================================================================
// RING ENTRY
// =============================================================================

/// A pending entry in the command ring
#[derive(Debug, Clone, Copy)]
pub struct RingEntry {
    /// GPU address of commands
    pub cmd_addr: GpuAddr,
    /// Size of commands
    pub cmd_size: u32,
    /// Fence value to signal on completion
    pub fence_value: u64,
    /// Submission flags
    pub flags: u32,
}

// =============================================================================
// COMMAND RING
// =============================================================================

/// GPU command ring for submission scheduling
#[derive(Debug)]
pub struct CommandRing {
    /// Ring configuration
    config: RingConfig,
    /// GPU address of ring buffer
    gpu_addr: GpuAddr,
    /// CPU mapping
    cpu_ptr: *mut u8,
    /// Write position (host-side)
    write_pos: u64,
    /// Read position (GPU-side, from fence memory)
    read_pos: AtomicU64,
    /// Next fence value
    next_fence: AtomicU64,
    /// Pending submissions
    pending: alloc::vec::Vec<RingEntry>,
    /// Statistics
    stats: RingStats,
}

/// Ring statistics
#[derive(Debug, Clone, Default)]
pub struct RingStats {
    /// Total submissions
    pub total_submissions: u64,
    /// Total completions
    pub total_completions: u64,
    /// Ring wraps
    pub ring_wraps: u64,
    /// Stalls (ring full)
    pub stalls: u64,
}

impl CommandRing {
    /// Create a new command ring
    ///
    /// # Safety
    /// - gpu_addr must be valid
    /// - cpu_ptr must be valid and point to at least config.size bytes
    pub unsafe fn new(config: RingConfig, gpu_addr: GpuAddr, cpu_ptr: *mut u8) -> Self {
        Self {
            config,
            gpu_addr,
            cpu_ptr,
            write_pos: 0,
            read_pos: AtomicU64::new(0),
            next_fence: AtomicU64::new(1),
            pending: alloc::vec::Vec::new(),
            stats: RingStats::default(),
        }
    }

    /// Get ring capacity in entries
    pub fn capacity(&self) -> u64 {
        self.config.size.as_bytes() / self.config.entry_size as u64
    }

    /// Get number of pending submissions
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if ring has space for a submission
    pub fn has_space(&self) -> bool {
        self.pending.len() < self.config.max_pending as usize
    }

    /// Get next fence value
    pub fn next_fence_value(&self) -> u64 {
        self.next_fence.load(Ordering::Relaxed)
    }

    /// Submit commands to the ring
    pub fn submit(&mut self, cmd_addr: GpuAddr, cmd_size: u32, flags: u32) -> Result<u64> {
        if !self.has_space() {
            self.stats.stalls += 1;
            return Err(Error::OutOfMemory);
        }

        let fence_value = self.next_fence.fetch_add(1, Ordering::SeqCst);

        let entry = RingEntry {
            cmd_addr,
            cmd_size,
            fence_value,
            flags,
        };

        // Write entry to ring
        self.write_entry(&entry)?;

        self.pending.push(entry);
        self.stats.total_submissions += 1;

        Ok(fence_value)
    }

    /// Write entry to ring buffer
    fn write_entry(&mut self, entry: &RingEntry) -> Result<()> {
        let capacity = self.capacity();
        let ring_pos = self.write_pos % capacity;

        // Write command address and size to ring
        let offset = ring_pos as usize * 16; // 16 bytes per entry

        // SAFETY: bounds checked via ring_pos < capacity
        unsafe {
            let ptr = self.cpu_ptr.add(offset);
            // Write GPU address (8 bytes)
            core::ptr::write_volatile(ptr as *mut u64, entry.cmd_addr.0);
            // Write size + flags (8 bytes)
            let size_flags = (entry.cmd_size as u64) | ((entry.flags as u64) << 32);
            core::ptr::write_volatile(ptr.add(8) as *mut u64, size_flags);
        }

        self.write_pos += 1;

        if self.write_pos % capacity == 0 {
            self.stats.ring_wraps += 1;
        }

        Ok(())
    }

    /// Update completion status based on fence value
    pub fn update_completions(&mut self, completed_fence: u64) {
        let before = self.pending.len();

        self.pending.retain(|entry| entry.fence_value > completed_fence);

        let completed = before - self.pending.len();
        self.stats.total_completions += completed as u64;

        self.read_pos.store(completed_fence, Ordering::Release);
    }

    /// Wait for all pending work to complete
    pub fn flush(&mut self, completed_fence: u64) {
        self.update_completions(completed_fence);
    }

    /// Get ring statistics
    pub fn stats(&self) -> &RingStats {
        &self.stats
    }

    /// Get GPU address of ring
    pub fn gpu_addr(&self) -> GpuAddr {
        self.gpu_addr
    }

    /// Get current write position
    pub fn write_position(&self) -> u64 {
        self.write_pos
    }
}

// SAFETY: Ring is designed for controlled concurrent access
unsafe impl Send for CommandRing {}
unsafe impl Sync for CommandRing {}

// =============================================================================
// RING KICKOFF
// =============================================================================

/// Methods for kicking off (notifying GPU of) ring submissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickoffMethod {
    /// Doorbell register write
    Doorbell,
    /// MMIO semaphore release
    Semaphore,
    /// Host1x syncpoint
    Syncpoint,
}

/// Kickoff descriptor
#[derive(Debug)]
pub struct Kickoff {
    /// Method
    pub method: KickoffMethod,
    /// Register/address to write
    pub address: u64,
    /// Value to write
    pub value: u64,
}

impl Kickoff {
    /// Create doorbell kickoff
    pub const fn doorbell(address: u64, channel_id: u32) -> Self {
        Self {
            method: KickoffMethod::Doorbell,
            address,
            value: channel_id as u64,
        }
    }
}
