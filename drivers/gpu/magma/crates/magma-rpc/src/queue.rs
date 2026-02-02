//! # RPC Queues
//!
//! Command and response queue implementations.

use core::sync::atomic::{AtomicU32, Ordering};

use magma_core::{Error, Result, GpuAddr, ByteSize};

use crate::message::{RpcMessage, RpcHeader, MAX_MESSAGE_SIZE};

// =============================================================================
// QUEUE LAYOUT
// =============================================================================

/// Queue header in shared memory
#[derive(Debug)]
#[repr(C)]
pub struct QueueHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Queue version
    pub version: u32,
    /// Total queue size in bytes
    pub size: u32,
    /// Entry size
    pub entry_size: u32,
    /// Number of entries
    pub num_entries: u32,
    /// Write pointer (producer)
    pub write_ptr: AtomicU32,
    /// Read pointer (consumer)
    pub read_ptr: AtomicU32,
    /// Flags
    pub flags: u32,
}

impl QueueHeader {
    /// Queue magic number
    pub const MAGIC: u32 = 0x4D47_5251; // "MGRQ"

    /// Current version
    pub const VERSION: u32 = 1;

    /// Header size
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// =============================================================================
// COMMAND QUEUE
// =============================================================================

/// Command queue for sending requests to GSP
#[derive(Debug)]
pub struct CommandQueue {
    /// GPU address of queue memory
    gpu_addr: GpuAddr,
    /// CPU mapping of queue memory
    cpu_ptr: *mut u8,
    /// Queue size
    size: ByteSize,
    /// Number of entries
    num_entries: u32,
    /// Entry size
    entry_size: u32,
    /// Current sequence number
    sequence: AtomicU32,
}

impl CommandQueue {
    /// Create a new command queue
    ///
    /// # Safety
    /// - `gpu_addr` must be a valid GPU address
    /// - `cpu_ptr` must be a valid, properly aligned pointer
    /// - Memory must be at least `size` bytes
    pub unsafe fn new(
        gpu_addr: GpuAddr,
        cpu_ptr: *mut u8,
        size: ByteSize,
        entry_size: u32,
    ) -> Result<Self> {
        let num_entries = (size.as_bytes() as u32 - QueueHeader::SIZE as u32) / entry_size;

        if num_entries == 0 {
            return Err(Error::InvalidParameter);
        }

        let queue = Self {
            gpu_addr,
            cpu_ptr,
            size,
            num_entries,
            entry_size,
            sequence: AtomicU32::new(0),
        };

        // Initialize header
        // SAFETY: Caller guarantees pointer validity
        unsafe {
            queue.init_header()?;
        }

        Ok(queue)
    }

    /// Initialize queue header
    ///
    /// # Safety
    /// Caller must ensure cpu_ptr is valid
    unsafe fn init_header(&self) -> Result<()> {
        let header = self.cpu_ptr as *mut QueueHeader;
        // SAFETY: Caller guarantees pointer is valid and properly aligned
        unsafe {
            (*header).magic = QueueHeader::MAGIC;
            (*header).version = QueueHeader::VERSION;
            (*header).size = self.size.as_bytes() as u32;
            (*header).entry_size = self.entry_size;
            (*header).num_entries = self.num_entries;
            (*header).write_ptr = AtomicU32::new(0);
            (*header).read_ptr = AtomicU32::new(0);
            (*header).flags = 0;
        }
        Ok(())
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u32 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if queue has space for a message
    pub fn has_space(&self, msg_size: usize) -> bool {
        if msg_size > self.entry_size as usize {
            return false;
        }

        let header = self.cpu_ptr as *const QueueHeader;
        // SAFETY: Pointer was validated in constructor
        let (write, read) = unsafe {
            (
                (*header).write_ptr.load(Ordering::Acquire),
                (*header).read_ptr.load(Ordering::Acquire),
            )
        };

        let used = if write >= read {
            write - read
        } else {
            self.num_entries - read + write
        };

        used < self.num_entries - 1
    }

    /// Enqueue a message
    pub fn enqueue(&self, message: &RpcMessage) -> Result<()> {
        if message.size() > self.entry_size as usize {
            return Err(Error::OutOfMemory);
        }

        if !self.has_space(message.size()) {
            return Err(Error::OutOfMemory);
        }

        let header = self.cpu_ptr as *mut QueueHeader;

        // SAFETY: Pointer validated in constructor
        unsafe {
            let write = (*header).write_ptr.load(Ordering::Acquire);
            let entry_offset = QueueHeader::SIZE + (write as usize * self.entry_size as usize);

            // Copy message to queue
            let entry_ptr = self.cpu_ptr.add(entry_offset);
            let msg_bytes = message.to_bytes();
            core::ptr::copy_nonoverlapping(msg_bytes.as_ptr(), entry_ptr, msg_bytes.len());

            // Update write pointer
            let new_write = (write + 1) % self.num_entries;
            (*header).write_ptr.store(new_write, Ordering::Release);
        }

        Ok(())
    }

    /// Get GPU address for doorbell
    pub fn gpu_addr(&self) -> GpuAddr {
        self.gpu_addr
    }
}

// SAFETY: Queue is designed for thread-safe access
unsafe impl Send for CommandQueue {}
unsafe impl Sync for CommandQueue {}

// =============================================================================
// RESPONSE QUEUE
// =============================================================================

/// Response queue for receiving replies from GSP
#[derive(Debug)]
pub struct ResponseQueue {
    /// GPU address of queue memory
    gpu_addr: GpuAddr,
    /// CPU mapping of queue memory
    cpu_ptr: *mut u8,
    /// Queue size
    size: ByteSize,
    /// Number of entries
    num_entries: u32,
    /// Entry size
    entry_size: u32,
}

impl ResponseQueue {
    /// Create a new response queue
    ///
    /// # Safety
    /// Same requirements as CommandQueue::new
    pub unsafe fn new(
        gpu_addr: GpuAddr,
        cpu_ptr: *mut u8,
        size: ByteSize,
        entry_size: u32,
    ) -> Result<Self> {
        let num_entries = (size.as_bytes() as u32 - QueueHeader::SIZE as u32) / entry_size;

        if num_entries == 0 {
            return Err(Error::InvalidParameter);
        }

        let queue = Self {
            gpu_addr,
            cpu_ptr,
            size,
            num_entries,
            entry_size,
        };

        // Initialize header
        // SAFETY: Caller guarantees pointer validity
        unsafe {
            queue.init_header()?;
        }

        Ok(queue)
    }

    /// Initialize queue header
    unsafe fn init_header(&self) -> Result<()> {
        let header = self.cpu_ptr as *mut QueueHeader;
        // SAFETY: Caller guarantees pointer validity
        unsafe {
            (*header).magic = QueueHeader::MAGIC;
            (*header).version = QueueHeader::VERSION;
            (*header).size = self.size.as_bytes() as u32;
            (*header).entry_size = self.entry_size;
            (*header).num_entries = self.num_entries;
            (*header).write_ptr = AtomicU32::new(0);
            (*header).read_ptr = AtomicU32::new(0);
            (*header).flags = 0;
        }
        Ok(())
    }

    /// Check if queue has pending messages
    pub fn has_pending(&self) -> bool {
        let header = self.cpu_ptr as *const QueueHeader;
        // SAFETY: Pointer validated in constructor
        unsafe {
            let write = (*header).write_ptr.load(Ordering::Acquire);
            let read = (*header).read_ptr.load(Ordering::Acquire);
            write != read
        }
    }

    /// Dequeue a message (returns None if empty)
    pub fn dequeue(&self) -> Option<RpcMessage> {
        if !self.has_pending() {
            return None;
        }

        let header = self.cpu_ptr as *mut QueueHeader;

        // SAFETY: Pointer validated in constructor
        unsafe {
            let read = (*header).read_ptr.load(Ordering::Acquire);
            let entry_offset = QueueHeader::SIZE + (read as usize * self.entry_size as usize);

            // Read message from queue
            let entry_ptr = self.cpu_ptr.add(entry_offset);
            let msg_slice = core::slice::from_raw_parts(entry_ptr, self.entry_size as usize);

            // Parse message
            let message = RpcMessage::from_bytes(msg_slice).ok()?;

            // Update read pointer
            let new_read = (read + 1) % self.num_entries;
            (*header).read_ptr.store(new_read, Ordering::Release);

            Some(message)
        }
    }

    /// Poll for response with matching sequence number
    pub fn poll_response(&self, sequence: u32, timeout_us: u64) -> Result<RpcMessage> {
        let mut elapsed = 0u64;

        loop {
            if let Some(msg) = self.dequeue() {
                if msg.header.sequence == sequence {
                    return Ok(msg);
                }
                // TODO: Handle out-of-order responses
            }

            if elapsed >= timeout_us {
                return Err(Error::Timeout);
            }

            // Simple busy-wait
            for _ in 0..100 {
                core::hint::spin_loop();
            }
            elapsed += 1;
        }
    }
}

// SAFETY: Queue is designed for thread-safe access
unsafe impl Send for ResponseQueue {}
unsafe impl Sync for ResponseQueue {}
