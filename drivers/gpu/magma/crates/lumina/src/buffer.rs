//! GPU Buffer types and operations
//!
//! This module provides typed GPU buffers with compile-time safety guarantees.
//! The borrow checker is used to prevent data races on GPU resources.

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut, Range};

use crate::error::{Error, Result};
use crate::types::{BufferHandle, GpuData};

/// Usage hints for GPU buffers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferUsage {
    /// Vertex buffer (VAO binding)
    Vertex,
    /// Index buffer (element array)
    Index,
    /// Uniform buffer (constant data)
    Uniform,
    /// Storage buffer (read/write in shaders)
    Storage,
    /// Indirect draw/dispatch buffer
    Indirect,
    /// Transfer source
    TransferSrc,
    /// Transfer destination
    TransferDst,
}

impl BufferUsage {
    /// Returns the Vulkan usage flags for this usage
    pub const fn vk_flags(self) -> u32 {
        match self {
            Self::Vertex => 0x00000080,   // VK_BUFFER_USAGE_VERTEX_BUFFER_BIT
            Self::Index => 0x00000040,    // VK_BUFFER_USAGE_INDEX_BUFFER_BIT
            Self::Uniform => 0x00000010,  // VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT
            Self::Storage => 0x00000020,  // VK_BUFFER_USAGE_STORAGE_BUFFER_BIT
            Self::Indirect => 0x00000100, // VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT
            Self::TransferSrc => 0x00000001,
            Self::TransferDst => 0x00000002,
        }
    }
}

/// A typed GPU buffer
///
/// `GpuBuffer<T>` provides a type-safe interface to GPU memory.
/// The borrow checker tracks access to prevent data races:
///
/// ```rust
/// let mut buffer: GpuBuffer<f32> = GpuBuffer::new(1024, BufferUsage::Storage);
///
/// // Immutable borrow for reading
/// let read_slice = buffer.as_gpu_slice();
///
/// // This would fail to compile:
/// // let write_slice = buffer.as_gpu_slice_mut();
/// // error: cannot borrow mutably while immutably borrowed
/// ```
pub struct GpuBuffer<T: GpuData> {
    handle: BufferHandle,
    len: usize,
    capacity: usize,
    usage: BufferUsage,
    // Staging data for deferred upload
    staging: Option<Vec<u8>>,
    _marker: PhantomData<T>,
}

impl<T: GpuData> GpuBuffer<T> {
    /// Creates a new GPU buffer with the given capacity
    pub fn new(capacity: usize, usage: BufferUsage) -> Self {
        Self {
            handle: BufferHandle::null(),
            len: 0,
            capacity,
            usage,
            staging: None,
            _marker: PhantomData,
        }
    }

    /// Creates a GPU buffer from a slice of data
    pub fn from_slice(data: &[T], usage: BufferUsage) -> Self {
        let mut buffer = Self::new(data.len(), usage);
        buffer.upload(data);
        buffer
    }

    /// Returns the number of elements in the buffer
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the buffer in elements
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the size of the buffer in bytes
    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.len * T::SIZE
    }

    /// Returns the buffer usage
    #[inline]
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// Returns the underlying handle (for backend use)
    #[inline]
    pub(crate) fn handle(&self) -> BufferHandle {
        self.handle
    }

    /// Sets the underlying handle (for backend use)
    #[inline]
    pub(crate) fn set_handle(&mut self, handle: BufferHandle) {
        self.handle = handle;
    }

    /// Uploads data to the buffer
    ///
    /// This stages the data for upload on the next frame submission.
    pub fn upload(&mut self, data: &[T]) {
        assert!(
            data.len() <= self.capacity,
            "Data exceeds buffer capacity: {} > {}",
            data.len(),
            self.capacity
        );

        self.len = data.len();

        // Stage the data for upload
        let bytes = unsafe {
            core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * T::SIZE)
        };

        self.staging = Some(bytes.to_vec());
    }

    /// Clears the buffer
    pub fn clear(&mut self) {
        self.len = 0;
        self.staging = None;
    }

    /// Fills the buffer with a single value
    pub fn fill(&mut self, value: T) {
        let data: Vec<T> = (0..self.capacity).map(|_| value).collect();
        self.upload(&data);
    }

    /// Returns a GPU slice for reading
    ///
    /// This creates an immutable borrow that can be passed to shaders.
    #[inline]
    pub fn as_gpu_slice(&self) -> GpuSlice<'_, T> {
        GpuSlice {
            buffer: self,
            range: 0..self.len,
        }
    }

    /// Returns a GPU slice for writing
    ///
    /// This creates a mutable borrow that grants exclusive write access.
    #[inline]
    pub fn as_gpu_slice_mut(&mut self) -> GpuSliceMut<'_, T> {
        let len = self.len;
        GpuSliceMut {
            buffer: self,
            range: 0..len,
        }
    }

    /// Returns a range of elements as a GPU slice
    #[inline]
    pub fn slice(&self, range: Range<usize>) -> GpuSlice<'_, T> {
        assert!(range.end <= self.len, "Slice range out of bounds");
        GpuSlice {
            buffer: self,
            range,
        }
    }

    /// Returns a mutable range of elements as a GPU slice
    #[inline]
    pub fn slice_mut(&mut self, range: Range<usize>) -> GpuSliceMut<'_, T> {
        assert!(range.end <= self.len, "Slice range out of bounds");
        GpuSliceMut {
            buffer: self,
            range,
        }
    }

    /// Takes the staging data for upload
    pub(crate) fn take_staging(&mut self) -> Option<Vec<u8>> {
        self.staging.take()
    }
}

impl<T: GpuData + Default> GpuBuffer<T> {
    /// Creates a buffer filled with the default value
    pub fn with_default(capacity: usize, usage: BufferUsage) -> Self {
        let mut buffer = Self::new(capacity, usage);
        buffer.fill(T::default());
        buffer
    }
}

/// An immutable view into a GPU buffer
///
/// This type represents an immutable borrow of GPU memory,
/// allowing safe concurrent reads from shaders.
pub struct GpuSlice<'a, T: GpuData> {
    buffer: &'a GpuBuffer<T>,
    range: Range<usize>,
}

impl<'a, T: GpuData> GpuSlice<'a, T> {
    /// Returns the number of elements in the slice
    #[inline]
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Returns true if the slice is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Returns the offset in elements
    #[inline]
    pub fn offset(&self) -> usize {
        self.range.start
    }

    /// Returns the underlying buffer handle
    #[inline]
    pub(crate) fn handle(&self) -> BufferHandle {
        self.buffer.handle
    }

    /// Returns the byte offset
    #[inline]
    pub(crate) fn byte_offset(&self) -> usize {
        self.range.start * T::SIZE
    }

    /// Returns the byte size
    #[inline]
    pub(crate) fn byte_size(&self) -> usize {
        self.range.len() * T::SIZE
    }
}

impl<'a, T: GpuData> Clone for GpuSlice<'a, T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            range: self.range.clone(),
        }
    }
}

impl<'a, T: GpuData> Copy for GpuSlice<'a, T> {}

/// A mutable view into a GPU buffer
///
/// This type represents a mutable borrow of GPU memory,
/// providing exclusive write access to prevent data races.
pub struct GpuSliceMut<'a, T: GpuData> {
    buffer: &'a mut GpuBuffer<T>,
    range: Range<usize>,
}

impl<'a, T: GpuData> GpuSliceMut<'a, T> {
    /// Returns the number of elements in the slice
    #[inline]
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Returns true if the slice is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Returns the offset in elements
    #[inline]
    pub fn offset(&self) -> usize {
        self.range.start
    }

    /// Returns the underlying buffer handle
    #[inline]
    pub(crate) fn handle(&self) -> BufferHandle {
        self.buffer.handle
    }

    /// Returns the byte offset
    #[inline]
    pub(crate) fn byte_offset(&self) -> usize {
        self.range.start * T::SIZE
    }

    /// Returns the byte size
    #[inline]
    pub(crate) fn byte_size(&self) -> usize {
        self.range.len() * T::SIZE
    }

    /// Downgrades to an immutable slice
    #[inline]
    pub fn as_slice(&self) -> GpuSlice<'_, T> {
        GpuSlice {
            buffer: self.buffer,
            range: self.range.clone(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INDEX BUFFER
// ═══════════════════════════════════════════════════════════════════════════

/// Type of indices in an index buffer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexType {
    /// 16-bit unsigned integer indices
    U16,
    /// 32-bit unsigned integer indices
    U32,
}

impl IndexType {
    /// Returns the size of one index in bytes
    pub const fn size(self) -> usize {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    /// Returns the Vulkan index type constant
    pub const fn vk_type(self) -> u32 {
        match self {
            Self::U16 => 0, // VK_INDEX_TYPE_UINT16
            Self::U32 => 1, // VK_INDEX_TYPE_UINT32
        }
    }
}

/// An index buffer with type-erased index type
pub struct IndexBuffer {
    handle: BufferHandle,
    count: usize,
    index_type: IndexType,
    staging: Option<Vec<u8>>,
}

impl IndexBuffer {
    /// Creates a new index buffer from u16 indices
    pub fn from_u16(indices: &[u16]) -> Self {
        let bytes =
            unsafe { core::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 2) };

        Self {
            handle: BufferHandle::null(),
            count: indices.len(),
            index_type: IndexType::U16,
            staging: Some(bytes.to_vec()),
        }
    }

    /// Creates a new index buffer from u32 indices
    pub fn from_u32(indices: &[u32]) -> Self {
        let bytes =
            unsafe { core::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 4) };

        Self {
            handle: BufferHandle::null(),
            count: indices.len(),
            index_type: IndexType::U32,
            staging: Some(bytes.to_vec()),
        }
    }

    /// Returns the number of indices
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the index type
    #[inline]
    pub fn index_type(&self) -> IndexType {
        self.index_type
    }

    /// Returns the underlying handle
    #[inline]
    pub(crate) fn handle(&self) -> BufferHandle {
        self.handle
    }

    /// Sets the underlying handle
    #[inline]
    pub(crate) fn set_handle(&mut self, handle: BufferHandle) {
        self.handle = handle;
    }

    /// Takes the staging data
    pub(crate) fn take_staging(&mut self) -> Option<Vec<u8>> {
        self.staging.take()
    }
}
