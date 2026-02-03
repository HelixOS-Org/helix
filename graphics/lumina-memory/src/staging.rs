//! Staging Buffer Management
//!
//! Staging buffers for CPU to GPU data transfers.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use lumina_core::Handle;

use crate::MemoryLocation;

// ============================================================================
// Staging Buffer Handle
// ============================================================================

/// Handle to a staging buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StagingBufferHandle(Handle<StagingBuffer>);

impl StagingBufferHandle {
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
// Staging Buffer
// ============================================================================

/// A staging buffer for CPU to GPU transfers.
pub struct StagingBuffer {
    /// Handle.
    pub handle: StagingBufferHandle,
    /// Buffer size.
    pub size: u64,
    /// Current offset.
    offset: AtomicU64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
    /// Frame allocated.
    pub frame_allocated: u64,
    /// Is upload (CPU->GPU) or download (GPU->CPU).
    pub is_upload: bool,
    /// Debug name.
    pub name: Option<String>,
}

impl StagingBuffer {
    /// Create a new staging buffer.
    pub fn new(handle: StagingBufferHandle, size: u64, is_upload: bool, frame: u64) -> Self {
        Self {
            handle,
            size,
            offset: AtomicU64::new(0),
            mapped_ptr: None,
            frame_allocated: frame,
            is_upload,
            name: None,
        }
    }

    /// Get current offset.
    pub fn offset(&self) -> u64 {
        self.offset.load(Ordering::Relaxed)
    }

    /// Get available space.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.offset())
    }

    /// Check if has space for size.
    pub fn has_space(&self, size: u64, alignment: u64) -> bool {
        let aligned_offset = self.align(self.offset(), alignment);
        aligned_offset + size <= self.size
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }

    /// Allocate from staging buffer.
    pub fn allocate(&self, size: u64, alignment: u64) -> Option<StagingAllocation> {
        loop {
            let current = self.offset.load(Ordering::Relaxed);
            let aligned_offset = self.align(current, alignment);

            if aligned_offset + size > self.size {
                return None;
            }

            let new_offset = aligned_offset + size;

            if self
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Some(StagingAllocation {
                    buffer: self.handle,
                    offset: aligned_offset,
                    size,
                    mapped_ptr: self
                        .mapped_ptr
                        .map(|ptr| unsafe { ptr.add(aligned_offset as usize) }),
                });
            }
        }
    }

    /// Reset the buffer.
    pub fn reset(&self) {
        self.offset.store(0, Ordering::Relaxed);
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.offset() as f32 / self.size as f32
        }
    }
}

// ============================================================================
// Staging Allocation
// ============================================================================

/// An allocation from a staging buffer.
#[derive(Debug, Clone)]
pub struct StagingAllocation {
    /// Buffer handle.
    pub buffer: StagingBufferHandle,
    /// Offset in buffer.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl StagingAllocation {
    /// Write data.
    pub fn write(&self, data: &[u8]) {
        if let Some(ptr) = self.mapped_ptr {
            let len = data.len().min(self.size as usize);
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, len);
            }
        }
    }

    /// Read data.
    pub fn read(&self, buffer: &mut [u8]) {
        if let Some(ptr) = self.mapped_ptr {
            let len = buffer.len().min(self.size as usize);
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, buffer.as_mut_ptr(), len);
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
// Upload Request
// ============================================================================

/// Request for uploading data.
#[derive(Debug)]
pub struct UploadRequest {
    /// Source staging allocation.
    pub staging: StagingAllocation,
    /// Destination buffer offset.
    pub dst_offset: u64,
    /// Size to copy.
    pub size: u64,
}

impl UploadRequest {
    /// Create a new upload request.
    pub fn new(staging: StagingAllocation, dst_offset: u64) -> Self {
        let size = staging.size;
        Self {
            staging,
            dst_offset,
            size,
        }
    }

    /// Create with explicit size.
    pub fn with_size(staging: StagingAllocation, dst_offset: u64, size: u64) -> Self {
        Self {
            staging,
            dst_offset,
            size,
        }
    }
}

// ============================================================================
// Download Request
// ============================================================================

/// Request for downloading data.
#[derive(Debug)]
pub struct DownloadRequest {
    /// Destination staging allocation.
    pub staging: StagingAllocation,
    /// Source buffer offset.
    pub src_offset: u64,
    /// Size to copy.
    pub size: u64,
    /// Callback when complete (frame number).
    pub completion_frame: Option<u64>,
}

impl DownloadRequest {
    /// Create a new download request.
    pub fn new(staging: StagingAllocation, src_offset: u64) -> Self {
        let size = staging.size;
        Self {
            staging,
            src_offset,
            size,
            completion_frame: None,
        }
    }

    /// Set completion frame.
    pub fn with_completion_frame(mut self, frame: u64) -> Self {
        self.completion_frame = Some(frame);
        self
    }
}

// ============================================================================
// Staging Manager
// ============================================================================

/// Manages staging buffers.
pub struct StagingManager {
    /// Upload buffers.
    upload_buffers: Vec<Option<StagingBuffer>>,
    /// Download buffers.
    download_buffers: Vec<Option<StagingBuffer>>,
    /// Free upload buffer indices.
    free_upload_indices: Vec<u32>,
    /// Free download buffer indices.
    free_download_indices: Vec<u32>,
    /// Upload buffer generations.
    upload_generations: Vec<u32>,
    /// Download buffer generations.
    download_generations: Vec<u32>,
    /// Active upload buffer per frame.
    active_upload_buffers: VecDeque<StagingBufferHandle>,
    /// Active download buffers per frame.
    active_download_buffers: VecDeque<StagingBufferHandle>,
    /// Default buffer size.
    pub default_buffer_size: u64,
    /// Current frame.
    current_frame: u64,
    /// Frames in flight.
    pub frames_in_flight: u32,
}

impl StagingManager {
    /// Create a new staging manager.
    pub fn new(default_buffer_size: u64, frames_in_flight: u32) -> Self {
        Self {
            upload_buffers: Vec::new(),
            download_buffers: Vec::new(),
            free_upload_indices: Vec::new(),
            free_download_indices: Vec::new(),
            upload_generations: Vec::new(),
            download_generations: Vec::new(),
            active_upload_buffers: VecDeque::new(),
            active_download_buffers: VecDeque::new(),
            default_buffer_size,
            current_frame: 0,
            frames_in_flight,
        }
    }

    /// Create an upload buffer.
    pub fn create_upload_buffer(&mut self, size: u64) -> StagingBufferHandle {
        let index = if let Some(index) = self.free_upload_indices.pop() {
            index
        } else {
            let index = self.upload_buffers.len() as u32;
            self.upload_buffers.push(None);
            self.upload_generations.push(0);
            index
        };

        let generation = self.upload_generations[index as usize];
        let handle = StagingBufferHandle::new(index, generation);

        let buffer = StagingBuffer::new(handle, size, true, self.current_frame);
        self.upload_buffers[index as usize] = Some(buffer);

        handle
    }

    /// Create a download buffer.
    pub fn create_download_buffer(&mut self, size: u64) -> StagingBufferHandle {
        let index = if let Some(index) = self.free_download_indices.pop() {
            index
        } else {
            let index = self.download_buffers.len() as u32;
            self.download_buffers.push(None);
            self.download_generations.push(0);
            index
        };

        let generation = self.download_generations[index as usize];
        let handle = StagingBufferHandle::new(index, generation);

        let buffer = StagingBuffer::new(handle, size, false, self.current_frame);
        self.download_buffers[index as usize] = Some(buffer);

        handle
    }

    /// Get upload buffer.
    pub fn get_upload(&self, handle: StagingBufferHandle) -> Option<&StagingBuffer> {
        let index = handle.index() as usize;
        if index >= self.upload_buffers.len() {
            return None;
        }
        if self.upload_generations[index] != handle.generation() {
            return None;
        }
        self.upload_buffers[index].as_ref()
    }

    /// Get download buffer.
    pub fn get_download(&self, handle: StagingBufferHandle) -> Option<&StagingBuffer> {
        let index = handle.index() as usize;
        if index >= self.download_buffers.len() {
            return None;
        }
        if self.download_generations[index] != handle.generation() {
            return None;
        }
        self.download_buffers[index].as_ref()
    }

    /// Allocate upload memory.
    pub fn allocate_upload(&mut self, size: u64, alignment: u64) -> Option<StagingAllocation> {
        // Try existing active upload buffer
        if let Some(&handle) = self.active_upload_buffers.back() {
            if let Some(buffer) = self.get_upload(handle) {
                if let Some(allocation) = buffer.allocate(size, alignment) {
                    return Some(allocation);
                }
            }
        }

        // Create new buffer
        let buffer_size = size.max(self.default_buffer_size);
        let handle = self.create_upload_buffer(buffer_size);
        self.active_upload_buffers.push_back(handle);

        // Allocate from new buffer
        self.get_upload(handle)?.allocate(size, alignment)
    }

    /// Allocate download memory.
    pub fn allocate_download(&mut self, size: u64, alignment: u64) -> Option<StagingAllocation> {
        // Try existing active download buffer
        if let Some(&handle) = self.active_download_buffers.back() {
            if let Some(buffer) = self.get_download(handle) {
                if let Some(allocation) = buffer.allocate(size, alignment) {
                    return Some(allocation);
                }
            }
        }

        // Create new buffer
        let buffer_size = size.max(self.default_buffer_size);
        let handle = self.create_download_buffer(buffer_size);
        self.active_download_buffers.push_back(handle);

        // Allocate from new buffer
        self.get_download(handle)?.allocate(size, alignment)
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        self.current_frame += 1;

        // Recycle old upload buffers
        while self.active_upload_buffers.len() > self.frames_in_flight as usize {
            if let Some(handle) = self.active_upload_buffers.pop_front() {
                if let Some(buffer) = self.get_upload(handle) {
                    buffer.reset();
                }
            }
        }

        // Recycle old download buffers
        while self.active_download_buffers.len() > self.frames_in_flight as usize {
            if let Some(handle) = self.active_download_buffers.pop_front() {
                if let Some(buffer) = self.get_download(handle) {
                    buffer.reset();
                }
            }
        }
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }

    /// Get upload buffer count.
    pub fn upload_buffer_count(&self) -> usize {
        self.upload_buffers.iter().filter(|b| b.is_some()).count()
    }

    /// Get download buffer count.
    pub fn download_buffer_count(&self) -> usize {
        self.download_buffers.iter().filter(|b| b.is_some()).count()
    }

    /// Get total upload memory.
    pub fn total_upload_memory(&self) -> u64 {
        self.upload_buffers
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.size)
            .sum()
    }

    /// Get total download memory.
    pub fn total_download_memory(&self) -> u64 {
        self.download_buffers
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.size)
            .sum()
    }
}

impl Default for StagingManager {
    fn default() -> Self {
        Self::new(
            64 * 1024 * 1024, // 64MB default
            3,                // 3 frames in flight
        )
    }
}

// ============================================================================
// Staging Statistics
// ============================================================================

/// Staging statistics.
#[derive(Debug, Clone, Default)]
pub struct StagingStatistics {
    /// Upload buffer count.
    pub upload_buffer_count: u32,
    /// Download buffer count.
    pub download_buffer_count: u32,
    /// Total upload memory.
    pub total_upload_memory: u64,
    /// Total download memory.
    pub total_download_memory: u64,
    /// Used upload memory.
    pub used_upload_memory: u64,
    /// Used download memory.
    pub used_download_memory: u64,
    /// Total bytes uploaded.
    pub total_bytes_uploaded: u64,
    /// Total bytes downloaded.
    pub total_bytes_downloaded: u64,
}

impl StagingStatistics {
    /// Calculate from manager.
    pub fn from_manager(manager: &StagingManager) -> Self {
        let used_upload: u64 = manager
            .upload_buffers
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.offset())
            .sum();

        let used_download: u64 = manager
            .download_buffers
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.offset())
            .sum();

        Self {
            upload_buffer_count: manager.upload_buffer_count() as u32,
            download_buffer_count: manager.download_buffer_count() as u32,
            total_upload_memory: manager.total_upload_memory(),
            total_download_memory: manager.total_download_memory(),
            used_upload_memory: used_upload,
            used_download_memory: used_download,
            total_bytes_uploaded: 0,
            total_bytes_downloaded: 0,
        }
    }
}
