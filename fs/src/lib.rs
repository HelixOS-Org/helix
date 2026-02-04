//! # HelixFS - Revolutionary Copy-on-Write Filesystem
//!
//! HelixFS is a next-generation filesystem designed for modern hardware and workloads.
//!
//! ## Key Features
//!
//! - **Copy-on-Write (CoW)**: Never overwrites data in place, ensuring crash consistency
//! - **Log-Structured Metadata**: LSM-tree inspired metadata for write optimization
//! - **Instant Snapshots**: O(1) snapshot creation via CoW semantics
//! - **Temporal Versioning**: Built-in file history with point-in-time recovery
//! - **Adaptive Compression**: Per-extent compression with algorithm selection
//! - **Native Encryption**: AEAD encryption with per-file keys
//! - **Cryptographic Integrity**: Merkle DAG for data verification
//! - **Lock-Free Concurrency**: Designed for massively parallel access
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        VFS Interface                             │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │   File Ops  │ │   Dir Ops   │ │  Inode Ops  │               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                     Transaction Layer                            │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │   Journal   │ │   Atomic    │ │   Rollback  │               │
//! │  │   Engine    │ │   Commits   │ │   Support   │               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                      Metadata Layer                              │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │  B+Tree     │ │   Radix     │ │  Snapshot   │               │
//! │  │  Engine     │ │   Tree      │ │  Manager    │               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                       Data Layer                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │   Extent    │ │   Block     │ │   Cache     │               │
//! │  │   Manager   │ │  Allocator  │ │   (ARC)     │               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                      Security Layer                              │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │  Crypto     │ │  Integrity  │ │   Access    │               │
//! │  │  Engine     │ │  (Merkle)   │ │   Control   │               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                      Block Device                                │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## On-Disk Format
//!
//! ```text
//! Block 0-7:      Primary Superblock (replicated 8x)
//! Block 8-15:     Backup Superblock
//! Block 16-1023:  Allocation Bitmap
//! Block 1024+:    Data/Metadata Region (CoW)
//! ```

#![no_std]
// #![cfg_attr(feature = "alloc", feature(allocator_api))] // Requires nightly
#![allow(dead_code)]
#![allow(unused_variables)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "alloc")]
extern crate alloc as alloc_crate;

// std is only available when the "std" feature is enabled AND we're not on a no_std target
// Note: This feature should only be enabled for testing on standard platforms

// ============================================================================
// Core Module Exports
// ============================================================================

pub mod alloc;
pub mod api;
pub mod cache;
pub mod compress;
pub mod core;
pub mod crypto;
pub mod disk;
pub mod journal;
pub mod ops;
pub mod snapshot;
pub mod tree;
pub mod vfs;

// Re-exports for convenience
pub use crate::core::error::{HfsError, HfsResult};
pub use crate::core::types::*;
pub use crate::disk::superblock::Superblock;
// pub use crate::api::filesystem::HelixFs; // TODO: implement filesystem module

/// HelixFS version information
pub const VERSION_MAJOR: u16 = 1;
pub const VERSION_MINOR: u16 = 0;
pub const VERSION_PATCH: u16 = 0;
pub const VERSION_STRING: &str = "1.0.0";

/// Magic number for HelixFS: "HELIXFS1" in little-endian
pub const HFS_MAGIC: u64 = 0x3153_4658_494C_4548; // "HELIXFS1"

/// Default block size (4KB, aligned to page size)
pub const BLOCK_SIZE: usize = 4096;
pub const BLOCK_SHIFT: u32 = 12;
pub const BLOCK_MASK: u64 = (BLOCK_SIZE - 1) as u64;

/// Maximum filename length
pub const MAX_NAME_LEN: usize = 255;

/// Maximum path length
pub const MAX_PATH_LEN: usize = 4096;

/// Maximum file size (16 EB theoretical, limited by extent tree depth)
pub const MAX_FILE_SIZE: u64 = 1 << 60; // 1 EB practical limit

/// Maximum filesystem size (256 ZB theoretical)
pub const MAX_FS_SIZE: u64 = u64::MAX;

/// Number of superblock replicas
pub const SUPERBLOCK_REPLICAS: usize = 8;

/// Root inode number
pub const ROOT_INO: u64 = 1;

/// Null block/inode marker
pub const NULL_BLOCK: u64 = 0;
pub const NULL_INO: u64 = 0;
