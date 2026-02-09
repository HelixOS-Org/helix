//! # Coop Bloom Filter
//!
//! Probabilistic membership testing for cooperative protocols:
//! - Standard Bloom filter with configurable false positive rate
//! - Counting Bloom filter with deletion support
//! - Scalable Bloom filter with auto-growth
//! - FNV-1a based multi-hash scheme
//! - Space-efficient membership exchange between peers
//! - Filter merge and intersection operations

extern crate alloc;

use alloc::vec::Vec;

/// Bloom filter hash scheme using FNV-1a with double hashing
fn bloom_hashes(item: u64, num_hashes: usize, bit_count: usize) -> Vec<usize> {
    let mut h1: u64 = 0xcbf29ce484222325;
    h1 ^= item;
    h1 = h1.wrapping_mul(0x100000001b3);

    let mut h2: u64 = 0x84222325cbf29ce4;
    h2 ^= item;
    h2 = h2.wrapping_mul(0x100000001b3);

    let mut positions = Vec::new();
    for i in 0..num_hashes {
        let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
        positions.push((combined as usize) % bit_count);
    }
    positions
}

/// Standard Bloom filter
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<u64>,
    bit_count: usize,
    num_hashes: usize,
    item_count: u64,
}

impl BloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let bit_count = optimal_bits(expected_items, fp_rate).max(64);
        let num_hashes = optimal_hashes(bit_count, expected_items).max(1);
        let words = (bit_count + 63) / 64;
        Self { bits: alloc::vec![0u64; words], bit_count, num_hashes, item_count: 0 }
    }

    #[inline]
    pub fn insert(&mut self, item: u64) {
        let positions = bloom_hashes(item, self.num_hashes, self.bit_count);
        for pos in positions {
            self.bits[pos / 64] |= 1u64 << (pos % 64);
        }
        self.item_count += 1;
    }

    #[inline]
    pub fn contains(&self, item: u64) -> bool {
        let positions = bloom_hashes(item, self.num_hashes, self.bit_count);
        for pos in positions {
            if self.bits[pos / 64] & (1u64 << (pos % 64)) == 0 { return false; }
        }
        true
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        for w in &mut self.bits { *w = 0; }
        self.item_count = 0;
    }

    #[inline]
    pub fn estimated_fp_rate(&self) -> f64 {
        let m = self.bit_count as f64;
        let k = self.num_hashes as f64;
        let n = self.item_count as f64;
        let p = 1.0 - libm::exp(-k * n / m);
        libm::pow(p, k)
    }

    #[inline]
    pub fn merge(&mut self, other: &BloomFilter) {
        if self.bit_count == other.bit_count {
            for (a, b) in self.bits.iter_mut().zip(other.bits.iter()) {
                *a |= *b;
            }
            self.item_count += other.item_count;
        }
    }

    #[inline(always)]
    pub fn item_count(&self) -> u64 { self.item_count }
    #[inline(always)]
    pub fn bit_count(&self) -> usize { self.bit_count }
    #[inline(always)]
    pub fn size_bytes(&self) -> usize { self.bits.len() * 8 }

    #[inline(always)]
    pub fn fill_ratio(&self) -> f64 {
        let set: usize = self.bits.iter().map(|w| w.count_ones() as usize).sum();
        set as f64 / self.bit_count as f64
    }
}

/// Counting Bloom filter (supports deletion)
#[derive(Debug, Clone)]
pub struct CountingBloomFilter {
    counters: Vec<u8>,
    bit_count: usize,
    num_hashes: usize,
    item_count: u64,
    overflow_count: u64,
}

impl CountingBloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let bit_count = optimal_bits(expected_items, fp_rate).max(64);
        let num_hashes = optimal_hashes(bit_count, expected_items).max(1);
        Self { counters: alloc::vec![0u8; bit_count], bit_count, num_hashes, item_count: 0, overflow_count: 0 }
    }

    #[inline]
    pub fn insert(&mut self, item: u64) {
        let positions = bloom_hashes(item, self.num_hashes, self.bit_count);
        for pos in positions {
            if self.counters[pos] < 255 { self.counters[pos] += 1; }
            else { self.overflow_count += 1; }
        }
        self.item_count += 1;
    }

    pub fn remove(&mut self, item: u64) -> bool {
        let positions = bloom_hashes(item, self.num_hashes, self.bit_count);
        // Check membership first
        for &pos in &positions {
            if self.counters[pos] == 0 { return false; }
        }
        for pos in positions {
            if self.counters[pos] > 0 { self.counters[pos] -= 1; }
        }
        self.item_count = self.item_count.saturating_sub(1);
        true
    }

    #[inline(always)]
    pub fn contains(&self, item: u64) -> bool {
        let positions = bloom_hashes(item, self.num_hashes, self.bit_count);
        positions.iter().all(|&pos| self.counters[pos] > 0)
    }

    #[inline(always)]
    pub fn item_count(&self) -> u64 { self.item_count }
    #[inline(always)]
    pub fn size_bytes(&self) -> usize { self.counters.len() }
    #[inline(always)]
    pub fn overflow_count(&self) -> u64 { self.overflow_count }
}

/// Scalable Bloom filter that grows as needed
#[derive(Debug, Clone)]
pub struct ScalableBloomFilter {
    filters: Vec<BloomFilter>,
    initial_capacity: usize,
    fp_rate: f64,
    growth_factor: usize,
    total_items: u64,
}

impl ScalableBloomFilter {
    pub fn new(initial_capacity: usize, fp_rate: f64) -> Self {
        let first = BloomFilter::new(initial_capacity, fp_rate / 2.0);
        Self {
            filters: alloc::vec![first], initial_capacity, fp_rate,
            growth_factor: 2, total_items: 0,
        }
    }

    #[inline]
    pub fn insert(&mut self, item: u64) {
        let last = self.filters.last().unwrap();
        if last.fill_ratio() > 0.5 {
            let new_cap = self.initial_capacity * self.growth_factor.pow(self.filters.len() as u32);
            let tighter_fp = self.fp_rate / libm::pow(2.0, (self.filters.len() + 1) as f64);
            self.filters.push(BloomFilter::new(new_cap, tighter_fp));
        }
        let last = self.filters.last_mut().unwrap();
        last.insert(item);
        self.total_items += 1;
    }

    #[inline(always)]
    pub fn contains(&self, item: u64) -> bool {
        self.filters.iter().any(|f| f.contains(item))
    }

    #[inline(always)]
    pub fn total_items(&self) -> u64 { self.total_items }
    #[inline(always)]
    pub fn filter_count(&self) -> usize { self.filters.len() }
    #[inline(always)]
    pub fn total_size_bytes(&self) -> usize { self.filters.iter().map(|f| f.size_bytes()).sum() }
}

/// Bloom filter stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BloomFilterStats {
    pub total_filters: usize,
    pub total_items: u64,
    pub total_memory_bytes: usize,
    pub avg_fill_ratio: f64,
    pub estimated_fp_rate: f64,
}

/// Cooperative bloom filter manager
pub struct CoopBloomFilter {
    filters: Vec<(u64, BloomFilter)>,
    stats: BloomFilterStats,
}

impl CoopBloomFilter {
    pub fn new() -> Self {
        Self { filters: Vec::new(), stats: BloomFilterStats::default() }
    }

    #[inline(always)]
    pub fn create_filter(&mut self, id: u64, capacity: usize, fp_rate: f64) {
        self.filters.push((id, BloomFilter::new(capacity, fp_rate)));
    }

    #[inline]
    pub fn insert(&mut self, filter_id: u64, item: u64) {
        if let Some((_, f)) = self.filters.iter_mut().find(|(id, _)| *id == filter_id) {
            f.insert(item);
        }
    }

    #[inline(always)]
    pub fn contains(&self, filter_id: u64, item: u64) -> bool {
        self.filters.iter().find(|(id, _)| *id == filter_id).map(|(_, f)| f.contains(item)).unwrap_or(false)
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_filters = self.filters.len();
        self.stats.total_items = self.filters.iter().map(|(_, f)| f.item_count()).sum();
        self.stats.total_memory_bytes = self.filters.iter().map(|(_, f)| f.size_bytes()).sum();
        if !self.filters.is_empty() {
            self.stats.avg_fill_ratio = self.filters.iter().map(|(_, f)| f.fill_ratio()).sum::<f64>() / self.filters.len() as f64;
            self.stats.estimated_fp_rate = self.filters.iter().map(|(_, f)| f.estimated_fp_rate()).sum::<f64>() / self.filters.len() as f64;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &BloomFilterStats { &self.stats }
}

fn optimal_bits(n: usize, p: f64) -> usize {
    let ln2 = core::f64::consts::LN_2;
    let m = -(n as f64 * libm::log(p)) / (ln2 * ln2);
    m as usize
}

fn optimal_hashes(m: usize, n: usize) -> usize {
    let k = (m as f64 / n as f64) * core::f64::consts::LN_2;
    k as usize
}
