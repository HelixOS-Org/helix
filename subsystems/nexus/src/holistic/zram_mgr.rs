//! # Holistic ZRAM Manager
//!
//! ZRAM (compressed RAM) management with holistic awareness:
//! - Compressed page accounting
//! - Compression algorithm selection and stats
//! - Memory savings tracking
//! - Per-device ZRAM state
//! - Writeback to backing store
//! - Compaction and idle page management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZramCompAlgo {
    Lzo,
    LzoRle,
    Lz4,
    Lz4hc,
    Zstd,
    Deflate,
    SnappyLike,
}

/// ZRAM device state
#[derive(Debug, Clone)]
pub struct ZramDevice {
    pub id: u64,
    pub disksize_bytes: u64,
    pub algorithm: ZramCompAlgo,
    pub orig_data_size: u64,
    pub compr_data_size: u64,
    pub mem_used_total: u64,
    pub mem_used_max: u64,
    pub pages_stored: u64,
    pub same_pages: u64,
    pub huge_pages: u64,
    pub idle_pages: u64,
    pub writeback_pages: u64,
    pub num_reads: u64,
    pub num_writes: u64,
    pub failed_reads: u64,
    pub failed_writes: u64,
    pub invalid_io: u64,
    pub notify_free: u64,
    pub max_comp_streams: u32,
}

impl ZramDevice {
    pub fn new(id: u64, disksize: u64, algo: ZramCompAlgo) -> Self {
        Self {
            id, disksize_bytes: disksize, algorithm: algo,
            orig_data_size: 0, compr_data_size: 0, mem_used_total: 0,
            mem_used_max: 0, pages_stored: 0, same_pages: 0,
            huge_pages: 0, idle_pages: 0, writeback_pages: 0,
            num_reads: 0, num_writes: 0, failed_reads: 0,
            failed_writes: 0, invalid_io: 0, notify_free: 0,
            max_comp_streams: 4,
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compr_data_size == 0 { 0.0 } else { self.orig_data_size as f64 / self.compr_data_size as f64 }
    }

    pub fn memory_savings(&self) -> u64 { self.orig_data_size.saturating_sub(self.compr_data_size) }

    pub fn write_page(&mut self, orig_size: u64, compr_size: u64) {
        self.orig_data_size += orig_size;
        self.compr_data_size += compr_size;
        self.mem_used_total += compr_size;
        if self.mem_used_total > self.mem_used_max { self.mem_used_max = self.mem_used_total; }
        self.pages_stored += 1;
        self.num_writes += 1;
    }

    pub fn write_same_page(&mut self) { self.same_pages += 1; self.pages_stored += 1; self.num_writes += 1; }

    pub fn read_page(&mut self) { self.num_reads += 1; }

    pub fn free_page(&mut self, compr_size: u64) {
        self.pages_stored = self.pages_stored.saturating_sub(1);
        self.compr_data_size = self.compr_data_size.saturating_sub(compr_size);
        self.mem_used_total = self.mem_used_total.saturating_sub(compr_size);
        self.notify_free += 1;
    }

    pub fn mark_idle(&mut self, pages: u64) { self.idle_pages = pages; }
    pub fn writeback_idle(&mut self, pages: u64) { self.writeback_pages += pages; self.idle_pages = self.idle_pages.saturating_sub(pages); }
    pub fn usage_pct(&self) -> f64 { if self.disksize_bytes == 0 { 0.0 } else { self.orig_data_size as f64 / self.disksize_bytes as f64 * 100.0 } }
}

/// Writeback record
#[derive(Debug, Clone)]
pub struct ZramWritebackRecord {
    pub device_id: u64,
    pub pages: u64,
    pub bytes: u64,
    pub ts: u64,
    pub reason: ZramWritebackReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZramWritebackReason {
    Idle,
    Huge,
    Incompressible,
    Manual,
}

/// Compression statistics per algorithm
#[derive(Debug, Clone)]
pub struct CompAlgoStats {
    pub algo: ZramCompAlgo,
    pub pages_compressed: u64,
    pub total_input_bytes: u64,
    pub total_output_bytes: u64,
    pub avg_ratio: f64,
    pub max_latency_ns: u64,
    pub avg_latency_ns: u64,
}

impl CompAlgoStats {
    pub fn new(algo: ZramCompAlgo) -> Self {
        Self { algo, pages_compressed: 0, total_input_bytes: 0, total_output_bytes: 0, avg_ratio: 0.0, max_latency_ns: 0, avg_latency_ns: 0 }
    }

    pub fn record(&mut self, input: u64, output: u64, latency: u64) {
        self.pages_compressed += 1;
        self.total_input_bytes += input;
        self.total_output_bytes += output;
        if self.total_output_bytes > 0 { self.avg_ratio = self.total_input_bytes as f64 / self.total_output_bytes as f64; }
        if latency > self.max_latency_ns { self.max_latency_ns = latency; }
        self.avg_latency_ns = (self.avg_latency_ns * (self.pages_compressed - 1) + latency) / self.pages_compressed;
    }
}

/// ZRAM manager stats
#[derive(Debug, Clone, Default)]
pub struct ZramStats {
    pub devices: usize,
    pub total_stored_pages: u64,
    pub total_savings_bytes: u64,
    pub avg_compression_ratio: f64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_writeback_pages: u64,
}

/// Holistic ZRAM manager
pub struct HolisticZramMgr {
    devices: BTreeMap<u64, ZramDevice>,
    algo_stats: BTreeMap<u8, CompAlgoStats>,
    writeback_history: Vec<ZramWritebackRecord>,
    stats: ZramStats,
    next_id: u64,
}

impl HolisticZramMgr {
    pub fn new() -> Self {
        Self { devices: BTreeMap::new(), algo_stats: BTreeMap::new(), writeback_history: Vec::new(), stats: ZramStats::default(), next_id: 1 }
    }

    pub fn create_device(&mut self, disksize: u64, algo: ZramCompAlgo) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.devices.insert(id, ZramDevice::new(id, disksize, algo));
        self.algo_stats.entry(algo as u8).or_insert_with(|| CompAlgoStats::new(algo));
        id
    }

    pub fn write_page(&mut self, dev: u64, orig: u64, compr: u64, latency: u64) {
        if let Some(d) = self.devices.get_mut(&dev) {
            let algo = d.algorithm;
            d.write_page(orig, compr);
            if let Some(s) = self.algo_stats.get_mut(&(algo as u8)) { s.record(orig, compr, latency); }
        }
    }

    pub fn write_same(&mut self, dev: u64) { if let Some(d) = self.devices.get_mut(&dev) { d.write_same_page(); } }
    pub fn read_page(&mut self, dev: u64) { if let Some(d) = self.devices.get_mut(&dev) { d.read_page(); } }
    pub fn free_page(&mut self, dev: u64, compr_size: u64) { if let Some(d) = self.devices.get_mut(&dev) { d.free_page(compr_size); } }

    pub fn writeback(&mut self, dev: u64, pages: u64, reason: ZramWritebackReason, ts: u64) {
        if let Some(d) = self.devices.get_mut(&dev) {
            let bytes = pages * 4096;
            d.writeback_idle(pages);
            self.writeback_history.push(ZramWritebackRecord { device_id: dev, pages, bytes, ts, reason });
        }
    }

    pub fn resize(&mut self, dev: u64, new_size: u64) {
        if let Some(d) = self.devices.get_mut(&dev) { d.disksize_bytes = new_size; }
    }

    pub fn change_algo(&mut self, dev: u64, algo: ZramCompAlgo) {
        if let Some(d) = self.devices.get_mut(&dev) { d.algorithm = algo; }
        self.algo_stats.entry(algo as u8).or_insert_with(|| CompAlgoStats::new(algo));
    }

    pub fn recompute(&mut self) {
        self.stats.devices = self.devices.len();
        self.stats.total_stored_pages = self.devices.values().map(|d| d.pages_stored).sum();
        self.stats.total_savings_bytes = self.devices.values().map(|d| d.memory_savings()).sum();
        self.stats.total_reads = self.devices.values().map(|d| d.num_reads).sum();
        self.stats.total_writes = self.devices.values().map(|d| d.num_writes).sum();
        self.stats.total_writeback_pages = self.devices.values().map(|d| d.writeback_pages).sum();
        let ratios: Vec<f64> = self.devices.values().filter(|d| d.compr_data_size > 0).map(|d| d.compression_ratio()).collect();
        if !ratios.is_empty() { self.stats.avg_compression_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64; }
    }

    pub fn device(&self, id: u64) -> Option<&ZramDevice> { self.devices.get(&id) }
    pub fn algo_stats(&self, algo: ZramCompAlgo) -> Option<&CompAlgoStats> { self.algo_stats.get(&(algo as u8)) }
    pub fn stats(&self) -> &ZramStats { &self.stats }
}
