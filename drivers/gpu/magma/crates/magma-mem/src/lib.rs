//! # MAGMA Memory Management
//!
//! VRAM allocators, memory pools, and GPU address space management.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      MAGMA Memory System                        │
//! │                                                                 │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │                    Address Space Manager                  │  │
//! │  │         (GPU Virtual Address → Physical Mapping)          │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! │                              │                                  │
//! │  ┌─────────────────┐  ┌─────┴─────┐  ┌────────────────────┐    │
//! │  │  Buddy Allocator│  │   Heap    │  │   Buffer Object    │    │
//! │  │   (VRAM Pool)   │  │  Manager  │  │     Tracker        │    │
//! │  └─────────────────┘  └───────────┘  └────────────────────┘    │
//! │           │                                     │               │
//! │  ┌────────┴─────────────────────────────────────┴───────────┐  │
//! │  │                    Physical VRAM                          │  │
//! │  │         (GDDR6/6X, HBM2/3 - up to 80GB)                  │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Allocator Design
//!
//! The buddy allocator provides O(log N) allocation/deallocation with
//! minimal fragmentation. It supports:
//!
//! - Power-of-2 block sizes (4KB to 2GB)
//! - Memory coloring for cache optimization
//! - Deferred freeing for GPU timeline safety
//! - Fragmentation metrics and defragmentation hints

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod address_space;
pub mod buddy;
pub mod heap;
pub mod pool;
pub mod tracker;

// Re-exports
pub use address_space::{AddressSpace, VaRange};
pub use buddy::{BuddyAllocator, BuddyBlock};
pub use heap::{HeapType, VramHeap};
pub use pool::{MemoryPool, PoolConfig};
