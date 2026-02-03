//! LUMINA Memory - GPU Memory Management
//!
//! This crate provides comprehensive GPU memory management for LUMINA,
//! including allocators, memory pools, streaming, and defragmentation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Memory Management                     │
//! ├─────────────────┬──────────────────┬───────────────────┤
//! │   Allocators    │    Memory Pools  │    Streaming      │
//! │  (TLSF, Buddy)  │  (Ring, Linear)  │  (Upload/Read)    │
//! ├─────────────────┼──────────────────┼───────────────────┤
//! │   Heap Types    │   Suballocation  │   Defragment      │
//! │  (Device/Host)  │   (Block/Page)   │   (Compaction)    │
//! ├─────────────────┼──────────────────┼───────────────────┤
//! │   Compression   │   Deduplication  │   Delta Encode    │
//! │   (LZ4/ZSTD)    │   (Content Hash) │   (Frame Delta)   │
//! └─────────────────┴──────────────────┴───────────────────┘
//! ```
//!
//! # Features
//!
//! - **Allocators**: TLSF, buddy, linear allocators
//! - **Memory Pools**: Ring buffers, linear allocators
//! - **Streaming**: Upload and readback management
//! - **Defragmentation**: Memory compaction and optimization
//! - **Virtual Memory**: Sparse resource support
//! - **Compression**: LZ4/ZSTD compression with deduplication

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod allocator;
pub mod block;
pub mod compression;
pub mod heap;
pub mod pool;
pub mod staging;
pub mod streaming;
pub mod suballocator;
pub mod virtual_alloc;

/// Prelude for common imports
pub mod prelude {
    pub use crate::allocator::{
        Allocation, AllocationInfo, AllocationType, GpuAllocator, MemoryLocation,
    };
    pub use crate::block::{MemoryBlock, MemoryBlockInfo};
    pub use crate::compression::{CompressionAlgorithm, CompressionManager, CompressionSettings};
    pub use crate::heap::{HeapFlags, HeapInfo, HeapType, MemoryHeap};
    pub use crate::pool::{MemoryPool, PoolAllocation, PoolDesc};
    pub use crate::staging::{StagingBuffer, StagingManager, UploadRequest};
    pub use crate::streaming::{StreamingBuffer, StreamingManager, StreamingRequest};
    pub use crate::suballocator::{BuddyAllocator, LinearAllocator, TlsfAllocator};
    pub use crate::virtual_alloc::{VirtualAllocation, VirtualAllocator};
}

pub use prelude::*;
