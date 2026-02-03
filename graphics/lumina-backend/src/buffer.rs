//! GPU Buffer Management
//!
//! Buffer creation, mapping, and memory management.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Buffer Usage
// ============================================================================

bitflags! {
    /// Buffer usage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufferUsage: u32 {
        /// Can be used as a transfer source.
        const COPY_SRC = 1 << 0;
        /// Can be used as a transfer destination.
        const COPY_DST = 1 << 1;
        /// Can be used as a uniform buffer.
        const UNIFORM = 1 << 2;
        /// Can be used as a storage buffer.
        const STORAGE = 1 << 3;
        /// Can be used as an index buffer.
        const INDEX = 1 << 4;
        /// Can be used as a vertex buffer.
        const VERTEX = 1 << 5;
        /// Can be used for indirect draw/dispatch.
        const INDIRECT = 1 << 6;
        /// Buffer can be mapped for reading.
        const MAP_READ = 1 << 7;
        /// Buffer can be mapped for writing.
        const MAP_WRITE = 1 << 8;
        /// Acceleration structure storage.
        const ACCELERATION_STRUCTURE = 1 << 9;
        /// Shader binding table.
        const SHADER_BINDING_TABLE = 1 << 10;
        /// Buffer device address can be queried.
        const SHADER_DEVICE_ADDRESS = 1 << 11;
        /// Transform feedback buffer.
        const TRANSFORM_FEEDBACK = 1 << 12;
        /// Conditional rendering.
        const CONDITIONAL_RENDERING = 1 << 13;
    }
}

impl Default for BufferUsage {
    fn default() -> Self {
        BufferUsage::COPY_DST | BufferUsage::UNIFORM
    }
}

// ============================================================================
// Buffer Memory Type
// ============================================================================

/// Memory type for buffer allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferMemoryType {
    /// GPU-only memory (fastest).
    GpuOnly,
    /// CPU-visible, GPU-readable (upload).
    Upload,
    /// GPU-writable, CPU-readable (readback).
    Readback,
    /// Unified memory (shared between CPU/GPU).
    Shared,
}

impl BufferMemoryType {
    /// Check if CPU-visible.
    pub fn is_cpu_visible(&self) -> bool {
        !matches!(self, BufferMemoryType::GpuOnly)
    }

    /// Check if CPU-writable.
    pub fn is_cpu_writable(&self) -> bool {
        matches!(self, BufferMemoryType::Upload | BufferMemoryType::Shared)
    }

    /// Check if CPU-readable.
    pub fn is_cpu_readable(&self) -> bool {
        matches!(self, BufferMemoryType::Readback | BufferMemoryType::Shared)
    }
}

impl Default for BufferMemoryType {
    fn default() -> Self {
        BufferMemoryType::GpuOnly
    }
}

// ============================================================================
// Buffer Description
// ============================================================================

/// Description for buffer creation.
#[derive(Debug, Clone)]
pub struct BufferDesc {
    /// Buffer size in bytes.
    pub size: u64,
    /// Usage flags.
    pub usage: BufferUsage,
    /// Memory type.
    pub memory_type: BufferMemoryType,
    /// Debug label.
    pub label: Option<String>,
    /// Mapped at creation.
    pub mapped_at_creation: bool,
}

impl BufferDesc {
    /// Create a new buffer description.
    pub fn new(size: u64, usage: BufferUsage) -> Self {
        Self {
            size,
            usage,
            memory_type: BufferMemoryType::GpuOnly,
            label: None,
            mapped_at_creation: false,
        }
    }

    /// Set memory type.
    pub fn with_memory_type(mut self, memory_type: BufferMemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set mapped at creation.
    pub fn mapped_at_creation(mut self) -> Self {
        self.mapped_at_creation = true;
        self
    }

    /// Create uniform buffer.
    pub fn uniform(size: u64) -> Self {
        Self::new(size, BufferUsage::UNIFORM | BufferUsage::COPY_DST)
    }

    /// Create storage buffer.
    pub fn storage(size: u64) -> Self {
        Self::new(size, BufferUsage::STORAGE | BufferUsage::COPY_DST)
    }

    /// Create vertex buffer.
    pub fn vertex(size: u64) -> Self {
        Self::new(size, BufferUsage::VERTEX | BufferUsage::COPY_DST)
    }

    /// Create index buffer.
    pub fn index(size: u64) -> Self {
        Self::new(size, BufferUsage::INDEX | BufferUsage::COPY_DST)
    }

    /// Create staging buffer.
    pub fn staging(size: u64) -> Self {
        Self::new(size, BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE)
            .with_memory_type(BufferMemoryType::Upload)
    }

    /// Create readback buffer.
    pub fn readback(size: u64) -> Self {
        Self::new(size, BufferUsage::COPY_DST | BufferUsage::MAP_READ)
            .with_memory_type(BufferMemoryType::Readback)
    }
}

// ============================================================================
// Buffer Handle
// ============================================================================

/// Handle to a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(Handle<Buffer>);

impl BufferHandle {
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
}

// ============================================================================
// Buffer State
// ============================================================================

/// Buffer resource state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    /// Undefined state.
    Undefined,
    /// General state.
    General,
    /// Vertex buffer input.
    VertexBuffer,
    /// Index buffer input.
    IndexBuffer,
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer read.
    StorageRead,
    /// Storage buffer write.
    StorageWrite,
    /// Indirect argument buffer.
    IndirectArgument,
    /// Copy source.
    CopySrc,
    /// Copy destination.
    CopyDst,
    /// Host read.
    HostRead,
    /// Host write.
    HostWrite,
    /// Acceleration structure read.
    AccelerationStructureRead,
    /// Acceleration structure write.
    AccelerationStructureWrite,
}

// ============================================================================
// Buffer
// ============================================================================

/// A GPU buffer.
pub struct Buffer {
    /// Handle.
    pub handle: BufferHandle,
    /// Size in bytes.
    pub size: u64,
    /// Usage flags.
    pub usage: BufferUsage,
    /// Memory type.
    pub memory_type: BufferMemoryType,
    /// GPU address (if applicable).
    pub gpu_address: u64,
    /// Is mapped.
    pub is_mapped: bool,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
    /// Current state.
    pub state: BufferState,
    /// Debug label.
    pub label: Option<String>,
}

impl Buffer {
    /// Create a new buffer.
    pub fn new(handle: BufferHandle, desc: &BufferDesc) -> Self {
        Self {
            handle,
            size: desc.size,
            usage: desc.usage,
            memory_type: desc.memory_type,
            gpu_address: 0,
            is_mapped: desc.mapped_at_creation,
            mapped_ptr: None,
            state: BufferState::Undefined,
            label: desc.label.clone(),
        }
    }

    /// Check if buffer can be mapped.
    pub fn can_map(&self) -> bool {
        self.memory_type.is_cpu_visible()
    }

    /// Check if buffer has device address.
    pub fn has_device_address(&self) -> bool {
        self.usage.contains(BufferUsage::SHADER_DEVICE_ADDRESS)
    }

    /// Get size.
    pub fn size(&self) -> u64 {
        self.size
    }
}

// ============================================================================
// Buffer View
// ============================================================================

/// A view into a buffer.
#[derive(Debug, Clone, Copy)]
pub struct BufferView {
    /// Buffer handle.
    pub buffer: BufferHandle,
    /// Offset in bytes.
    pub offset: u64,
    /// Size in bytes.
    pub size: u64,
}

impl BufferView {
    /// Create a new buffer view.
    pub fn new(buffer: BufferHandle, offset: u64, size: u64) -> Self {
        Self { buffer, offset, size }
    }

    /// Create view of entire buffer.
    pub fn whole(buffer: BufferHandle, buffer_size: u64) -> Self {
        Self {
            buffer,
            offset: 0,
            size: buffer_size,
        }
    }
}

// ============================================================================
// Buffer Range
// ============================================================================

/// Range in a buffer.
#[derive(Debug, Clone, Copy)]
pub struct BufferRange {
    /// Offset in bytes.
    pub offset: u64,
    /// Size in bytes (0 = rest of buffer).
    pub size: u64,
}

impl BufferRange {
    /// Create a new range.
    pub fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Entire buffer.
    pub fn whole() -> Self {
        Self { offset: 0, size: 0 }
    }
}

impl Default for BufferRange {
    fn default() -> Self {
        Self::whole()
    }
}

// ============================================================================
// Buffer Manager
// ============================================================================

/// Manages buffer resources.
pub struct BufferManager {
    /// Buffers.
    buffers: Vec<Option<Buffer>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Total memory used.
    memory_used: AtomicU64,
    /// Buffer count.
    buffer_count: u32,
}

impl BufferManager {
    /// Create a new buffer manager.
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            memory_used: AtomicU64::new(0),
            buffer_count: 0,
        }
    }

    /// Create a buffer.
    pub fn create(&mut self, desc: &BufferDesc) -> BufferHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.buffers.len() as u32;
            self.buffers.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = BufferHandle::new(index, generation);
        let buffer = Buffer::new(handle, desc);

        self.memory_used.fetch_add(desc.size, Ordering::Relaxed);
        self.buffers[index as usize] = Some(buffer);
        self.buffer_count += 1;

        handle
    }

    /// Get a buffer.
    pub fn get(&self, handle: BufferHandle) -> Option<&Buffer> {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return None;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return None;
        }
        self.buffers[index].as_ref()
    }

    /// Get mutable buffer.
    pub fn get_mut(&mut self, handle: BufferHandle) -> Option<&mut Buffer> {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return None;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return None;
        }
        self.buffers[index].as_mut()
    }

    /// Destroy a buffer.
    pub fn destroy(&mut self, handle: BufferHandle) {
        let index = handle.index() as usize;
        if index >= self.buffers.len() {
            return;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return;
        }

        if let Some(buffer) = self.buffers[index].take() {
            self.memory_used.fetch_sub(buffer.size, Ordering::Relaxed);
            self.buffer_count -= 1;
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get total memory used.
    pub fn memory_used(&self) -> u64 {
        self.memory_used.load(Ordering::Relaxed)
    }

    /// Get buffer count.
    pub fn count(&self) -> u32 {
        self.buffer_count
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}
