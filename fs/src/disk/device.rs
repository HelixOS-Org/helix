//! Block device abstraction layer.
//!
//! This module provides a unified interface for interacting with block devices,
//! whether they are physical disks, partitions, or virtual devices like RAM disks.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;

// ============================================================================
// Block Device Traits
// ============================================================================

/// Block device read operations.
pub trait BlockRead {
    /// Read blocks from device.
    ///
    /// # Arguments
    /// * `start` - Starting block number
    /// * `buffer` - Buffer to read into (must be block-aligned)
    ///
    /// # Returns
    /// Number of blocks read
    fn read_blocks(&self, start: BlockNum, buffer: &mut [u8]) -> HfsResult<usize>;

    /// Read a single block
    fn read_block(&self, block: BlockNum, buffer: &mut [u8; 4096]) -> HfsResult<()> {
        let read = self.read_blocks(block, buffer)?;
        if read != 1 {
            return Err(HfsError::IoReadError);
        }
        Ok(())
    }
}

/// Block device write operations.
pub trait BlockWrite {
    /// Write blocks to device.
    ///
    /// # Arguments
    /// * `start` - Starting block number
    /// * `buffer` - Data to write (must be block-aligned)
    ///
    /// # Returns
    /// Number of blocks written
    fn write_blocks(&self, start: BlockNum, buffer: &[u8]) -> HfsResult<usize>;

    /// Write a single block
    fn write_block(&self, block: BlockNum, buffer: &[u8; 4096]) -> HfsResult<()> {
        let written = self.write_blocks(block, buffer)?;
        if written != 1 {
            return Err(HfsError::IoWriteError);
        }
        Ok(())
    }

    /// Sync/flush device
    fn sync(&self) -> HfsResult<()>;
}

/// Device information and capabilities.
pub trait BlockDeviceInfo {
    /// Get block size in bytes
    fn block_size(&self) -> u32;

    /// Get total number of blocks
    fn block_count(&self) -> u64;

    /// Get device capacity in bytes
    fn capacity(&self) -> u64 {
        self.block_count() * self.block_size() as u64
    }

    /// Check if device is read-only
    fn is_readonly(&self) -> bool;

    /// Get device model/name
    fn device_name(&self) -> &[u8];

    /// Get device serial number (if available)
    fn serial(&self) -> Option<&[u8]> {
        None
    }
}

/// Device discard/trim operations.
pub trait BlockDiscard {
    /// Discard blocks (TRIM for SSDs)
    fn discard(&self, start: BlockNum, count: u64) -> HfsResult<()>;

    /// Secure erase blocks
    fn secure_erase(&self, start: BlockNum, count: u64) -> HfsResult<()>;
}

/// Full block device trait combining read, write, and info.
pub trait BlockDevice: BlockRead + BlockWrite + BlockDeviceInfo + Send + Sync {}

/// Read-only block device.
pub trait BlockDeviceRO: BlockRead + BlockDeviceInfo + Send + Sync {}

// ============================================================================
// Device Capabilities
// ============================================================================

/// Device capability flags.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct DeviceCapabilities(pub u32);

impl DeviceCapabilities {
    /// Supports TRIM/discard
    pub const TRIM: u32 = 1 << 0;
    /// Supports secure erase
    pub const SECURE_ERASE: u32 = 1 << 1;
    /// Is a solid-state device
    pub const SSD: u32 = 1 << 2;
    /// Supports FUA (Force Unit Access)
    pub const FUA: u32 = 1 << 3;
    /// Supports barrier operations
    pub const BARRIER: u32 = 1 << 4;
    /// Has volatile write cache
    pub const WRITE_CACHE: u32 = 1 << 5;
    /// Supports atomic writes
    pub const ATOMIC_WRITE: u32 = 1 << 6;
    /// Is a NVME device
    pub const NVME: u32 = 1 << 7;
    /// Is a virtual device
    pub const VIRTUAL: u32 = 1 << 8;

    /// Check if capability is present
    #[inline]
    pub fn has(&self, cap: u32) -> bool {
        (self.0 & cap) != 0
    }

    /// Set capability
    #[inline]
    pub fn set(&mut self, cap: u32) {
        self.0 |= cap;
    }

    /// Clear capability
    #[inline]
    pub fn clear(&mut self, cap: u32) {
        self.0 &= !cap;
    }
}

// ============================================================================
// Device Statistics
// ============================================================================

/// Device I/O statistics.
#[derive(Default)]
pub struct DeviceStats {
    /// Total blocks read
    pub blocks_read: AtomicU64,
    /// Total blocks written
    pub blocks_written: AtomicU64,
    /// Total read operations
    pub read_ops: AtomicU64,
    /// Total write operations
    pub write_ops: AtomicU64,
    /// Read errors
    pub read_errors: AtomicU64,
    /// Write errors
    pub write_errors: AtomicU64,
    /// Total sync operations
    pub sync_ops: AtomicU64,
    /// Total blocks discarded
    pub blocks_discarded: AtomicU64,
}

impl DeviceStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            blocks_read: AtomicU64::new(0),
            blocks_written: AtomicU64::new(0),
            read_ops: AtomicU64::new(0),
            write_ops: AtomicU64::new(0),
            read_errors: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
            sync_ops: AtomicU64::new(0),
            blocks_discarded: AtomicU64::new(0),
        }
    }

    /// Record read
    #[inline]
    pub fn record_read(&self, blocks: u64) {
        self.blocks_read.fetch_add(blocks, Ordering::Relaxed);
        self.read_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record write
    #[inline]
    pub fn record_write(&self, blocks: u64) {
        self.blocks_written.fetch_add(blocks, Ordering::Relaxed);
        self.write_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record read error
    #[inline]
    pub fn record_read_error(&self) {
        self.read_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record write error
    #[inline]
    pub fn record_write_error(&self) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record sync
    #[inline]
    pub fn record_sync(&self) {
        self.sync_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record discard
    #[inline]
    pub fn record_discard(&self, blocks: u64) {
        self.blocks_discarded.fetch_add(blocks, Ordering::Relaxed);
    }

    /// Get snapshot of stats
    pub fn snapshot(&self) -> DeviceStatsSnapshot {
        DeviceStatsSnapshot {
            blocks_read: self.blocks_read.load(Ordering::Relaxed),
            blocks_written: self.blocks_written.load(Ordering::Relaxed),
            read_ops: self.read_ops.load(Ordering::Relaxed),
            write_ops: self.write_ops.load(Ordering::Relaxed),
            read_errors: self.read_errors.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
            sync_ops: self.sync_ops.load(Ordering::Relaxed),
            blocks_discarded: self.blocks_discarded.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of device statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceStatsSnapshot {
    pub blocks_read: u64,
    pub blocks_written: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub read_errors: u64,
    pub write_errors: u64,
    pub sync_ops: u64,
    pub blocks_discarded: u64,
}

impl DeviceStatsSnapshot {
    /// Calculate bytes read
    pub fn bytes_read(&self, block_size: u32) -> u64 {
        self.blocks_read * block_size as u64
    }

    /// Calculate bytes written
    pub fn bytes_written(&self, block_size: u32) -> u64 {
        self.blocks_written * block_size as u64
    }

    /// Average blocks per read
    pub fn avg_read_size(&self) -> f64 {
        if self.read_ops == 0 {
            0.0
        } else {
            self.blocks_read as f64 / self.read_ops as f64
        }
    }

    /// Average blocks per write
    pub fn avg_write_size(&self) -> f64 {
        if self.write_ops == 0 {
            0.0
        } else {
            self.blocks_written as f64 / self.write_ops as f64
        }
    }
}

// ============================================================================
// Device Handle
// ============================================================================

/// Handle to an opened block device.
///
/// Wraps a block device with additional state like statistics
/// and open mode.
pub struct DeviceHandle<D> {
    /// Underlying device
    device: D,
    /// Device capabilities
    capabilities: DeviceCapabilities,
    /// Statistics
    stats: DeviceStats,
    /// Read-only mode
    readonly: AtomicBool,
    /// Device is open
    is_open: AtomicBool,
    /// Reference count
    ref_count: AtomicU32,
}

impl<D> DeviceHandle<D> {
    /// Create new device handle
    pub fn new(device: D, capabilities: DeviceCapabilities) -> Self {
        Self {
            device,
            capabilities,
            stats: DeviceStats::new(),
            readonly: AtomicBool::new(false),
            is_open: AtomicBool::new(true),
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create read-only handle
    pub fn new_readonly(device: D, capabilities: DeviceCapabilities) -> Self {
        let handle = Self::new(device, capabilities);
        handle.readonly.store(true, Ordering::Relaxed);
        handle
    }

    /// Get device reference
    #[inline]
    pub fn device(&self) -> &D {
        &self.device
    }

    /// Get mutable device reference
    #[inline]
    pub fn device_mut(&mut self) -> &mut D {
        &mut self.device
    }

    /// Get capabilities
    #[inline]
    pub fn capabilities(&self) -> DeviceCapabilities {
        self.capabilities
    }

    /// Get statistics
    #[inline]
    pub fn stats(&self) -> &DeviceStats {
        &self.stats
    }

    /// Check if read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.readonly.load(Ordering::Relaxed)
    }

    /// Check if open
    #[inline]
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }

    /// Increment reference count
    pub fn add_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count, returns true if last reference
    pub fn release(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::Relaxed) == 1
    }

    /// Close handle
    pub fn close(&self) {
        self.is_open.store(false, Ordering::Relaxed);
    }
}

impl<D: BlockRead> DeviceHandle<D> {
    /// Read blocks with statistics
    pub fn read(&self, start: BlockNum, buffer: &mut [u8]) -> HfsResult<usize> {
        if !self.is_open() {
            return Err(HfsError::DeviceNotReady);
        }

        match self.device.read_blocks(start, buffer) {
            Ok(count) => {
                self.stats.record_read(count as u64);
                Ok(count)
            },
            Err(e) => {
                self.stats.record_read_error();
                Err(e)
            },
        }
    }
}

impl<D: BlockWrite> DeviceHandle<D> {
    /// Write blocks with statistics
    pub fn write(&self, start: BlockNum, buffer: &[u8]) -> HfsResult<usize> {
        if !self.is_open() {
            return Err(HfsError::DeviceNotReady);
        }
        if self.is_readonly() {
            return Err(HfsError::ReadOnlyFilesystem);
        }

        match self.device.write_blocks(start, buffer) {
            Ok(count) => {
                self.stats.record_write(count as u64);
                Ok(count)
            },
            Err(e) => {
                self.stats.record_write_error();
                Err(e)
            },
        }
    }

    /// Sync with statistics
    pub fn sync(&self) -> HfsResult<()> {
        if !self.is_open() {
            return Err(HfsError::DeviceNotReady);
        }

        self.stats.record_sync();
        self.device.sync()
    }
}

// ============================================================================
// Memory Block Device (for testing)
// ============================================================================

/// In-memory block device for testing.
///
/// Uses a fixed-size memory buffer as storage.
pub struct MemoryBlockDevice {
    /// Storage buffer
    buffer: *mut u8,
    /// Buffer size in bytes
    size: usize,
    /// Block size
    block_size: u32,
    /// Number of blocks
    block_count: u64,
    /// Device name
    name: [u8; 32],
}

impl MemoryBlockDevice {
    /// Create new memory device with given size in blocks.
    ///
    /// # Safety
    /// Caller must ensure the buffer remains valid for the lifetime
    /// of this device.
    pub unsafe fn from_buffer(buffer: *mut u8, size: usize, block_size: u32) -> Self {
        let block_count = (size / block_size as usize) as u64;

        let mut name = [0u8; 32];
        name[..6].copy_from_slice(b"memory");

        Self {
            buffer,
            size,
            block_size,
            block_count,
            name,
        }
    }

    /// Calculate offset for block number
    #[inline]
    fn block_offset(&self, block: BlockNum) -> Option<usize> {
        let offset = block.get() * self.block_size as u64;
        if offset < self.size as u64 {
            Some(offset as usize)
        } else {
            None
        }
    }
}

impl BlockRead for MemoryBlockDevice {
    fn read_blocks(&self, start: BlockNum, buffer: &mut [u8]) -> HfsResult<usize> {
        let offset = self
            .block_offset(start)
            .ok_or(HfsError::InvalidBlockNumber)?;

        let blocks = buffer.len() / self.block_size as usize;
        let bytes = blocks * self.block_size as usize;

        if offset + bytes > self.size {
            return Err(HfsError::InvalidBlockNumber);
        }

        // SAFETY: We've verified bounds
        unsafe {
            core::ptr::copy_nonoverlapping(self.buffer.add(offset), buffer.as_mut_ptr(), bytes);
        }

        Ok(blocks)
    }
}

impl BlockWrite for MemoryBlockDevice {
    fn write_blocks(&self, start: BlockNum, buffer: &[u8]) -> HfsResult<usize> {
        let offset = self
            .block_offset(start)
            .ok_or(HfsError::InvalidBlockNumber)?;

        let blocks = buffer.len() / self.block_size as usize;
        let bytes = blocks * self.block_size as usize;

        if offset + bytes > self.size {
            return Err(HfsError::InvalidBlockNumber);
        }

        // SAFETY: We've verified bounds
        unsafe {
            core::ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                self.buffer.add(offset),
                bytes,
            );
        }

        Ok(blocks)
    }

    fn sync(&self) -> HfsResult<()> {
        // Memory device doesn't need sync
        Ok(())
    }
}

impl BlockDeviceInfo for MemoryBlockDevice {
    fn block_size(&self) -> u32 {
        self.block_size
    }

    fn block_count(&self) -> u64 {
        self.block_count
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn device_name(&self) -> &[u8] {
        &self.name[..6]
    }
}

impl BlockDevice for MemoryBlockDevice {}

// SAFETY: MemoryBlockDevice can be safely shared between threads
// as long as the underlying buffer access is coordinated externally
unsafe impl Send for MemoryBlockDevice {}
unsafe impl Sync for MemoryBlockDevice {}

// ============================================================================
// Block Request (for async I/O)
// ============================================================================

/// I/O request type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum IoRequestType {
    /// Read operation
    Read    = 0,
    /// Write operation
    Write   = 1,
    /// Sync/flush operation
    Sync    = 2,
    /// Discard/trim operation
    Discard = 3,
}

/// I/O request priority
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum IoRequestPriority {
    /// Background I/O (lowest priority)
    Background = 0,
    /// Normal I/O
    Normal     = 1,
    /// High priority (user-facing)
    High       = 2,
    /// Real-time (highest priority)
    Realtime   = 3,
}

/// Block I/O request.
#[derive(Clone, Copy)]
pub struct BlockRequest {
    /// Request type
    pub req_type: IoRequestType,
    /// Priority
    pub priority: IoRequestPriority,
    /// Starting block
    pub start_block: BlockNum,
    /// Number of blocks
    pub block_count: u32,
    /// Buffer pointer (for read/write)
    pub buffer_ptr: usize,
    /// Callback data
    pub callback_data: usize,
    /// Request ID
    pub request_id: u64,
    /// Flags
    pub flags: u32,
}

impl BlockRequest {
    /// Create read request
    pub fn read(
        start_block: BlockNum,
        block_count: u32,
        buffer_ptr: usize,
        request_id: u64,
    ) -> Self {
        Self {
            req_type: IoRequestType::Read,
            priority: IoRequestPriority::Normal,
            start_block,
            block_count,
            buffer_ptr,
            callback_data: 0,
            request_id,
            flags: 0,
        }
    }

    /// Create write request
    pub fn write(
        start_block: BlockNum,
        block_count: u32,
        buffer_ptr: usize,
        request_id: u64,
    ) -> Self {
        Self {
            req_type: IoRequestType::Write,
            priority: IoRequestPriority::Normal,
            start_block,
            block_count,
            buffer_ptr,
            callback_data: 0,
            request_id,
            flags: 0,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: IoRequestPriority) -> Self {
        self.priority = priority;
        self
    }
}

// ============================================================================
// Block Queue (for request scheduling)
// ============================================================================

/// Maximum requests in queue
pub const MAX_QUEUE_REQUESTS: usize = 256;

/// Block I/O request queue.
pub struct BlockQueue {
    /// Pending requests
    requests: [Option<BlockRequest>; MAX_QUEUE_REQUESTS],
    /// Number of pending requests
    pending: AtomicU32,
    /// Next request ID
    next_id: AtomicU64,
    /// Head index (for dequeue)
    head: AtomicU32,
    /// Tail index (for enqueue)
    tail: AtomicU32,
}

impl Default for BlockQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockQueue {
    /// Create new empty queue
    pub const fn new() -> Self {
        const NONE: Option<BlockRequest> = None;
        Self {
            requests: [NONE; MAX_QUEUE_REQUESTS],
            pending: AtomicU32::new(0),
            next_id: AtomicU64::new(1),
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    /// Check if queue is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pending.load(Ordering::Relaxed) == 0
    }

    /// Check if queue is full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.pending.load(Ordering::Relaxed) >= MAX_QUEUE_REQUESTS as u32
    }

    /// Get number of pending requests
    #[inline]
    pub fn len(&self) -> usize {
        self.pending.load(Ordering::Relaxed) as usize
    }

    /// Generate next request ID
    #[inline]
    pub fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

// ============================================================================
// Partition Table Entry
// ============================================================================

/// Partition type GUID (simplified)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct PartitionType(pub [u8; 16]);

impl PartitionType {
    /// Empty/unused partition
    pub const EMPTY: Self = Self([0; 16]);

    /// Linux filesystem
    pub const LINUX_FS: Self = Self([
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ]);

    /// Check if partition is used
    #[inline]
    pub fn is_used(&self) -> bool {
        *self != Self::EMPTY
    }
}

/// Partition table entry.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PartitionEntry {
    /// Partition type
    pub partition_type: PartitionType,
    /// Partition GUID
    pub partition_guid: [u8; 16],
    /// First block
    pub first_block: u64,
    /// Last block (inclusive)
    pub last_block: u64,
    /// Attributes
    pub attributes: u64,
    /// Partition name (UTF-16LE, 36 chars max)
    pub name: [u16; 36],
}

impl PartitionEntry {
    /// Size in bytes
    pub const SIZE: usize = 128;

    /// Create empty entry
    pub const fn empty() -> Self {
        Self {
            partition_type: PartitionType::EMPTY,
            partition_guid: [0; 16],
            first_block: 0,
            last_block: 0,
            attributes: 0,
            name: [0; 36],
        }
    }

    /// Check if partition is used
    #[inline]
    pub fn is_used(&self) -> bool {
        self.partition_type.is_used()
    }

    /// Get partition size in blocks
    #[inline]
    pub fn size_blocks(&self) -> u64 {
        if self.is_used() && self.last_block >= self.first_block {
            self.last_block - self.first_block + 1
        } else {
            0
        }
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<PartitionEntry>() == PartitionEntry::SIZE);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_capabilities() {
        let mut caps = DeviceCapabilities::default();

        assert!(!caps.has(DeviceCapabilities::TRIM));

        caps.set(DeviceCapabilities::TRIM);
        caps.set(DeviceCapabilities::SSD);

        assert!(caps.has(DeviceCapabilities::TRIM));
        assert!(caps.has(DeviceCapabilities::SSD));
        assert!(!caps.has(DeviceCapabilities::NVME));

        caps.clear(DeviceCapabilities::TRIM);
        assert!(!caps.has(DeviceCapabilities::TRIM));
    }

    #[test]
    fn test_device_stats() {
        let stats = DeviceStats::new();

        stats.record_read(10);
        stats.record_read(5);
        stats.record_write(20);
        stats.record_write_error();

        let snap = stats.snapshot();

        assert_eq!(snap.blocks_read, 15);
        assert_eq!(snap.read_ops, 2);
        assert_eq!(snap.blocks_written, 20);
        assert_eq!(snap.write_ops, 1);
        assert_eq!(snap.write_errors, 1);
    }

    #[test]
    fn test_block_request() {
        let req = BlockRequest::read(BlockNum::new(100), 4, 0x1000, 42)
            .with_priority(IoRequestPriority::High);

        assert_eq!(req.req_type, IoRequestType::Read);
        assert_eq!(req.priority, IoRequestPriority::High);
        assert_eq!(req.start_block.get(), 100);
        assert_eq!(req.block_count, 4);
        assert_eq!(req.request_id, 42);
    }

    #[test]
    fn test_partition_entry() {
        let entry = PartitionEntry::empty();
        assert!(!entry.is_used());
        assert_eq!(entry.size_blocks(), 0);

        let mut used = entry;
        used.partition_type = PartitionType::LINUX_FS;
        used.first_block = 2048;
        used.last_block = 1000000;

        assert!(used.is_used());
        assert_eq!(used.size_blocks(), 1000000 - 2048 + 1);
    }
}
