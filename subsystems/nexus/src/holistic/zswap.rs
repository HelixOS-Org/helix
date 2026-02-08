// SPDX-License-Identifier: GPL-2.0
//! Holistic zswap — compressed swap cache.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Zswap compressor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZswapCompressor {
    Lzo,
    Lz4,
    Zstd,
    Deflate,
}

/// Zswap pool
#[derive(Debug)]
pub struct ZswapPool {
    pub compressor: ZswapCompressor,
    pub max_pool_percent: u32,
    pub current_size: u64,
    pub stored_pages: u64,
    pub written_back_pages: u64,
    pub rejected_pages: u64,
    pub duplicate_pages: u64,
    pub same_filled_pages: u64,
}

impl ZswapPool {
    pub fn new(comp: ZswapCompressor, max_pct: u32) -> Self {
        Self { compressor: comp, max_pool_percent: max_pct, current_size: 0, stored_pages: 0, written_back_pages: 0, rejected_pages: 0, duplicate_pages: 0, same_filled_pages: 0 }
    }
}

/// Zswap entry
#[derive(Debug)]
pub struct ZswapEntry {
    pub offset: u64,
    pub compressed_size: u32,
    pub original_size: u32,
    pub checksum: u64,
    pub same_filled: bool,
}

impl ZswapEntry {
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size == 0 { return 0.0; }
        self.original_size as f64 / self.compressed_size as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ZswapStats {
    pub stored_pages: u64,
    pub pool_size_bytes: u64,
    pub written_back: u64,
    pub rejected: u64,
    pub same_filled: u64,
    pub avg_compression_ratio: f64,
}

/// Main holistic zswap
pub struct HolisticZswap {
    pool: ZswapPool,
    entries: BTreeMap<u64, ZswapEntry>,
    total_original: u64,
    total_compressed: u64,
}

impl HolisticZswap {
    pub fn new(comp: ZswapCompressor, max_pct: u32) -> Self {
        Self { pool: ZswapPool::new(comp, max_pct), entries: BTreeMap::new(), total_original: 0, total_compressed: 0 }
    }

    pub fn store(&mut self, offset: u64, original: u32, compressed: u32, checksum: u64, same: bool) {
        if same { self.pool.same_filled_pages += 1; }
        self.pool.stored_pages += 1;
        self.pool.current_size += compressed as u64;
        self.total_original += original as u64;
        self.total_compressed += compressed as u64;
        self.entries.insert(offset, ZswapEntry { offset, compressed_size: compressed, original_size: original, checksum, same_filled: same });
    }

    pub fn load(&mut self, offset: u64) -> Option<&ZswapEntry> { self.entries.get(&offset) }

    pub fn invalidate(&mut self, offset: u64) {
        if let Some(e) = self.entries.remove(&offset) {
            self.pool.current_size -= e.compressed_size as u64;
        }
    }

    pub fn writeback(&mut self, offset: u64) {
        if let Some(e) = self.entries.remove(&offset) {
            self.pool.current_size -= e.compressed_size as u64;
            self.pool.written_back_pages += 1;
        }
    }

    pub fn stats(&self) -> ZswapStats {
        let ratio = if self.total_compressed == 0 { 0.0 } else { self.total_original as f64 / self.total_compressed as f64 };
        ZswapStats { stored_pages: self.pool.stored_pages, pool_size_bytes: self.pool.current_size, written_back: self.pool.written_back_pages, rejected: self.pool.rejected_pages, same_filled: self.pool.same_filled_pages, avg_compression_ratio: ratio }
    }
}
