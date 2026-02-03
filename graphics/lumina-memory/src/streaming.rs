//! Streaming Buffer Management
//!
//! Streaming buffers for dynamic data that changes frequently.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use lumina_core::Handle;

use crate::MemoryLocation;

// ============================================================================
// Streaming Buffer Handle
// ============================================================================

/// Handle to a streaming buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamingBufferHandle(Handle<StreamingBuffer>);

impl StreamingBufferHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Streaming Buffer
// ============================================================================

/// A streaming buffer with per-frame regions.
pub struct StreamingBuffer {
    /// Handle.
    pub handle: StreamingBufferHandle,
    /// Total buffer size.
    pub size: u64,
    /// Size per frame.
    pub frame_size: u64,
    /// Number of frames.
    pub frame_count: u32,
    /// Current frame.
    current_frame: u32,
    /// Current offset within frame.
    frame_offset: AtomicU64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
    /// Debug name.
    pub name: Option<String>,
}

impl StreamingBuffer {
    /// Create a new streaming buffer.
    pub fn new(handle: StreamingBufferHandle, frame_size: u64, frame_count: u32) -> Self {
        Self {
            handle,
            size: frame_size * frame_count as u64,
            frame_size,
            frame_count,
            current_frame: 0,
            frame_offset: AtomicU64::new(0),
            mapped_ptr: None,
            name: None,
        }
    }

    /// Get current frame offset.
    pub fn current_frame_base(&self) -> u64 {
        self.current_frame as u64 * self.frame_size
    }

    /// Get current offset within buffer.
    pub fn current_offset(&self) -> u64 {
        self.current_frame_base() + self.frame_offset.load(Ordering::Relaxed)
    }

    /// Get available space in current frame.
    pub fn available(&self) -> u64 {
        self.frame_size
            .saturating_sub(self.frame_offset.load(Ordering::Relaxed))
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }

    /// Allocate from current frame.
    pub fn allocate(&self, size: u64, alignment: u64) -> Option<StreamingAllocation> {
        loop {
            let current = self.frame_offset.load(Ordering::Relaxed);
            let aligned_offset = self.align(current, alignment);

            if aligned_offset + size > self.frame_size {
                return None;
            }

            let new_offset = aligned_offset + size;

            if self
                .frame_offset
                .compare_exchange_weak(current, new_offset, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let buffer_offset = self.current_frame_base() + aligned_offset;
                return Some(StreamingAllocation {
                    buffer: self.handle,
                    offset: buffer_offset,
                    size,
                    frame: self.current_frame,
                    mapped_ptr: self
                        .mapped_ptr
                        .map(|ptr| unsafe { ptr.add(buffer_offset as usize) }),
                });
            }
        }
    }

    /// Advance to next frame.
    pub fn next_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frame_count;
        self.frame_offset.store(0, Ordering::Relaxed);
    }

    /// Get current frame index.
    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    /// Get frame utilization.
    pub fn frame_utilization(&self) -> f32 {
        if self.frame_size == 0 {
            0.0
        } else {
            self.frame_offset.load(Ordering::Relaxed) as f32 / self.frame_size as f32
        }
    }

    /// Get frame pointer.
    pub fn frame_ptr(&self, frame: u32) -> Option<*mut u8> {
        if frame >= self.frame_count {
            return None;
        }
        self.mapped_ptr
            .map(|ptr| unsafe { ptr.add((frame as u64 * self.frame_size) as usize) })
    }
}

// ============================================================================
// Streaming Allocation
// ============================================================================

/// An allocation from a streaming buffer.
#[derive(Debug, Clone)]
pub struct StreamingAllocation {
    /// Buffer handle.
    pub buffer: StreamingBufferHandle,
    /// Offset in buffer.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Frame index.
    pub frame: u32,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl StreamingAllocation {
    /// Write data.
    pub fn write(&self, data: &[u8]) {
        if let Some(ptr) = self.mapped_ptr {
            let len = data.len().min(self.size as usize);
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, len);
            }
        }
    }

    /// Write at offset.
    pub fn write_at(&self, offset: u64, data: &[u8]) {
        if let Some(ptr) = self.mapped_ptr {
            if offset + data.len() as u64 <= self.size {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        data.as_ptr(),
                        ptr.add(offset as usize),
                        data.len(),
                    );
                }
            }
        }
    }

    /// Get slice.
    pub fn as_slice(&self) -> Option<&[u8]> {
        self.mapped_ptr
            .map(|ptr| unsafe { core::slice::from_raw_parts(ptr, self.size as usize) })
    }

    /// Get mutable slice.
    pub fn as_mut_slice(&mut self) -> Option<&mut [u8]> {
        self.mapped_ptr
            .map(|ptr| unsafe { core::slice::from_raw_parts_mut(ptr, self.size as usize) })
    }
}

// ============================================================================
// Streaming Request
// ============================================================================

/// A streaming data request.
#[derive(Debug)]
pub struct StreamingRequest {
    /// Allocation.
    pub allocation: StreamingAllocation,
    /// Destination offset (in target buffer).
    pub dst_offset: u64,
    /// Size.
    pub size: u64,
}

impl StreamingRequest {
    /// Create a new streaming request.
    pub fn new(allocation: StreamingAllocation, dst_offset: u64) -> Self {
        let size = allocation.size;
        Self {
            allocation,
            dst_offset,
            size,
        }
    }
}

// ============================================================================
// Streaming Buffer Description
// ============================================================================

/// Description for creating a streaming buffer.
#[derive(Debug, Clone)]
pub struct StreamingBufferDesc {
    /// Size per frame.
    pub frame_size: u64,
    /// Number of frames.
    pub frame_count: u32,
    /// Alignment.
    pub alignment: u64,
    /// Debug name.
    pub name: Option<String>,
}

impl Default for StreamingBufferDesc {
    fn default() -> Self {
        Self {
            frame_size: 4 * 1024 * 1024, // 4MB per frame
            frame_count: 3,
            alignment: 256,
            name: None,
        }
    }
}

impl StreamingBufferDesc {
    /// Create a new description.
    pub fn new(frame_size: u64, frame_count: u32) -> Self {
        Self {
            frame_size,
            frame_count,
            ..Default::default()
        }
    }

    /// Set alignment.
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Total size.
    pub fn total_size(&self) -> u64 {
        self.frame_size * self.frame_count as u64
    }
}

// ============================================================================
// Streaming Manager
// ============================================================================

/// Manages streaming buffers.
pub struct StreamingManager {
    /// Streaming buffers.
    buffers: Vec<Option<StreamingBuffer>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Default frame size.
    pub default_frame_size: u64,
    /// Frames in flight.
    pub frames_in_flight: u32,
    /// Current frame.
    current_frame: u64,
}

impl StreamingManager {
    /// Create a new streaming manager.
    pub fn new(default_frame_size: u64, frames_in_flight: u32) -> Self {
        Self {
            buffers: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            default_frame_size,
            frames_in_flight,
            current_frame: 0,
        }
    }

    /// Create a streaming buffer.
    pub fn create_buffer(&mut self, desc: &StreamingBufferDesc) -> StreamingBufferHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.buffers.len() as u32;
            self.buffers.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = StreamingBufferHandle::new(index, generation);

        let mut buffer = StreamingBuffer::new(handle, desc.frame_size, desc.frame_count);
        buffer.name = desc.name.clone();
        self.buffers[index as usize] = Some(buffer);

        handle
    }

    /// Destroy a streaming buffer.
    pub fn destroy_buffer(&mut self, handle: StreamingBufferHandle) -> bool {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return false;
        }
        if self.generations[index] != handle.generation() {
            return false;
        }

        self.buffers[index] = None;
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);

        true
    }

    /// Get a streaming buffer.
    pub fn get(&self, handle: StreamingBufferHandle) -> Option<&StreamingBuffer> {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.buffers[index].as_ref()
    }

    /// Get a streaming buffer (mutable).
    pub fn get_mut(&mut self, handle: StreamingBufferHandle) -> Option<&mut StreamingBuffer> {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.buffers[index].as_mut()
    }

    /// Allocate from buffer.
    pub fn allocate(
        &self,
        handle: StreamingBufferHandle,
        size: u64,
        alignment: u64,
    ) -> Option<StreamingAllocation> {
        self.get(handle)?.allocate(size, alignment)
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        self.current_frame += 1;

        // Advance all buffers to next frame
        for buffer in self.buffers.iter_mut().filter_map(|b| b.as_mut()) {
            buffer.next_frame();
        }
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }

    /// Get buffer count.
    pub fn buffer_count(&self) -> usize {
        self.buffers.iter().filter(|b| b.is_some()).count()
    }

    /// Get total memory.
    pub fn total_memory(&self) -> u64 {
        self.buffers
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.size)
            .sum()
    }
}

impl Default for StreamingManager {
    fn default() -> Self {
        Self::new(
            4 * 1024 * 1024, // 4MB per frame
            3,               // 3 frames in flight
        )
    }
}

// ============================================================================
// Ring Buffer
// ============================================================================

/// A ring buffer for streaming data.
pub struct RingBuffer {
    /// Total size.
    pub size: u64,
    /// Head position (write).
    head: AtomicU64,
    /// Tail position (read).
    tail: AtomicU64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl RingBuffer {
    /// Create a new ring buffer.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            mapped_ptr: None,
        }
    }

    /// Get available space for writing.
    pub fn available_write(&self) -> u64 {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        if head >= tail {
            self.size - (head - tail) - 1
        } else {
            tail - head - 1
        }
    }

    /// Get available data for reading.
    pub fn available_read(&self) -> u64 {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        if head >= tail {
            head - tail
        } else {
            self.size - tail + head
        }
    }

    /// Allocate space for writing.
    pub fn allocate(&self, size: u64) -> Option<RingBufferAllocation> {
        if size > self.available_write() {
            return None;
        }

        let head = self.head.load(Ordering::Relaxed);
        let offset = head % self.size;

        // Check if we need to wrap
        if offset + size > self.size {
            // Not enough contiguous space, allocation fails
            return None;
        }

        // Advance head
        self.head.fetch_add(size, Ordering::Relaxed);

        Some(RingBufferAllocation {
            offset,
            size,
            mapped_ptr: self
                .mapped_ptr
                .map(|ptr| unsafe { ptr.add(offset as usize) }),
        })
    }

    /// Free space after reading.
    pub fn free(&self, size: u64) {
        self.tail.fetch_add(size, Ordering::Relaxed);
    }

    /// Reset the ring buffer.
    pub fn reset(&self) {
        self.head.store(0, Ordering::Relaxed);
        self.tail.store(0, Ordering::Relaxed);
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.available_read() == 0
    }

    /// Check if full.
    pub fn is_full(&self) -> bool {
        self.available_write() == 0
    }
}

/// Ring buffer allocation.
#[derive(Debug, Clone)]
pub struct RingBufferAllocation {
    /// Offset in buffer.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl RingBufferAllocation {
    /// Write data.
    pub fn write(&self, data: &[u8]) {
        if let Some(ptr) = self.mapped_ptr {
            let len = data.len().min(self.size as usize);
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, len);
            }
        }
    }
}

// ============================================================================
// Streaming Statistics
// ============================================================================

/// Streaming statistics.
#[derive(Debug, Clone, Default)]
pub struct StreamingStatistics {
    /// Buffer count.
    pub buffer_count: u32,
    /// Total memory.
    pub total_memory: u64,
    /// Total bytes streamed.
    pub total_bytes_streamed: u64,
    /// Frames processed.
    pub frames_processed: u64,
}
