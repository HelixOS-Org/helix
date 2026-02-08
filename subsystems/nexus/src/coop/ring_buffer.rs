//! # Coop Ring Buffer
//!
//! Lock-free ring buffer protocol for cooperative subsystems:
//! - Fixed-size circular buffer with wrap-around
//! - Producer/consumer tracking
//! - Batch read/write operations
//! - Overflow policies (drop oldest, reject, overwrite)
//! - Watermark-based flow control
//! - Multi-channel ring buffer management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Overflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Drop the oldest entries to make room
    DropOldest,
    /// Reject new entries when full
    Reject,
    /// Overwrite oldest without tracking
    Overwrite,
}

/// Watermark event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkEvent {
    HighReached,
    LowReached,
    Normal,
}

/// A single ring buffer channel
#[derive(Debug, Clone)]
pub struct RingChannel {
    pub channel_id: u32,
    pub capacity: usize,
    pub head: usize,
    pub tail: usize,
    pub count: usize,
    pub overflow_policy: OverflowPolicy,
    pub high_watermark: f64,
    pub low_watermark: f64,
    pub total_written: u64,
    pub total_read: u64,
    pub total_dropped: u64,
    pub total_rejected: u64,
    pub last_watermark: WatermarkEvent,
    /// Simulated buffer slots (we store u64 tokens representing data)
    slots: Vec<u64>,
}

impl RingChannel {
    pub fn new(id: u32, capacity: usize, policy: OverflowPolicy) -> Self {
        Self {
            channel_id: id, capacity, head: 0, tail: 0, count: 0,
            overflow_policy: policy, high_watermark: 0.8, low_watermark: 0.2,
            total_written: 0, total_read: 0, total_dropped: 0, total_rejected: 0,
            last_watermark: WatermarkEvent::Normal,
            slots: alloc::vec![0u64; capacity],
        }
    }

    pub fn is_full(&self) -> bool { self.count >= self.capacity }
    pub fn is_empty(&self) -> bool { self.count == 0 }
    pub fn fill_ratio(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.count as f64 / self.capacity as f64
    }

    pub fn write_one(&mut self, value: u64) -> bool {
        if self.is_full() {
            match self.overflow_policy {
                OverflowPolicy::Reject => { self.total_rejected += 1; return false; }
                OverflowPolicy::DropOldest => {
                    // advance tail (drop oldest)
                    self.tail = (self.tail + 1) % self.capacity;
                    self.count -= 1;
                    self.total_dropped += 1;
                }
                OverflowPolicy::Overwrite => {
                    self.slots[self.head] = value;
                    self.head = (self.head + 1) % self.capacity;
                    self.tail = (self.tail + 1) % self.capacity;
                    self.total_written += 1;
                    self.update_watermark();
                    return true;
                }
            }
        }
        self.slots[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        self.count += 1;
        self.total_written += 1;
        self.update_watermark();
        true
    }

    pub fn read_one(&mut self) -> Option<u64> {
        if self.is_empty() { return None; }
        let val = self.slots[self.tail];
        self.tail = (self.tail + 1) % self.capacity;
        self.count -= 1;
        self.total_read += 1;
        self.update_watermark();
        Some(val)
    }

    pub fn write_batch(&mut self, values: &[u64]) -> usize {
        let mut written = 0;
        for &v in values {
            if self.write_one(v) { written += 1; } else { break; }
        }
        written
    }

    pub fn read_batch(&mut self, max: usize) -> Vec<u64> {
        let n = max.min(self.count);
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(v) = self.read_one() { result.push(v); } else { break; }
        }
        result
    }

    pub fn peek(&self) -> Option<u64> {
        if self.is_empty() { None } else { Some(self.slots[self.tail]) }
    }

    pub fn drain(&mut self) -> Vec<u64> {
        let mut result = Vec::with_capacity(self.count);
        while let Some(v) = self.read_one() { result.push(v); }
        result
    }

    fn update_watermark(&mut self) {
        let ratio = self.fill_ratio();
        if ratio >= self.high_watermark {
            self.last_watermark = WatermarkEvent::HighReached;
        } else if ratio <= self.low_watermark {
            self.last_watermark = WatermarkEvent::LowReached;
        } else {
            self.last_watermark = WatermarkEvent::Normal;
        }
    }

    pub fn set_watermarks(&mut self, low: f64, high: f64) {
        self.low_watermark = low.max(0.0).min(1.0);
        self.high_watermark = high.max(0.0).min(1.0);
    }

    pub fn throughput_ratio(&self) -> f64 {
        if self.total_written == 0 { return 0.0; }
        self.total_read as f64 / self.total_written as f64
    }

    pub fn drop_ratio(&self) -> f64 {
        let total_lost = self.total_dropped + self.total_rejected;
        if self.total_written + total_lost == 0 { return 0.0; }
        total_lost as f64 / (self.total_written + total_lost) as f64
    }
}

/// Producer state
#[derive(Debug, Clone)]
pub struct Producer {
    pub producer_id: u64,
    pub channel_id: u32,
    pub items_produced: u64,
    pub last_produce_ts: u64,
}

/// Consumer state
#[derive(Debug, Clone)]
pub struct Consumer {
    pub consumer_id: u64,
    pub channel_id: u32,
    pub items_consumed: u64,
    pub last_consume_ts: u64,
}

/// Ring buffer stats
#[derive(Debug, Clone, Default)]
pub struct RingBufferStats {
    pub total_channels: usize,
    pub total_producers: usize,
    pub total_consumers: usize,
    pub total_items_written: u64,
    pub total_items_read: u64,
    pub total_items_dropped: u64,
    pub total_items_rejected: u64,
    pub avg_fill_ratio: f64,
    pub high_watermark_channels: usize,
}

/// Cooperative ring buffer manager
pub struct CoopRingBuffer {
    channels: BTreeMap<u32, RingChannel>,
    producers: BTreeMap<u64, Producer>,
    consumers: BTreeMap<u64, Consumer>,
    stats: RingBufferStats,
}

impl CoopRingBuffer {
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(), producers: BTreeMap::new(),
            consumers: BTreeMap::new(), stats: RingBufferStats::default(),
        }
    }

    pub fn create_channel(&mut self, id: u32, capacity: usize, policy: OverflowPolicy) {
        self.channels.insert(id, RingChannel::new(id, capacity, policy));
    }

    pub fn remove_channel(&mut self, id: u32) { self.channels.remove(&id); }

    pub fn register_producer(&mut self, prod_id: u64, channel_id: u32) {
        self.producers.insert(prod_id, Producer { producer_id: prod_id, channel_id, items_produced: 0, last_produce_ts: 0 });
    }

    pub fn register_consumer(&mut self, cons_id: u64, channel_id: u32) {
        self.consumers.insert(cons_id, Consumer { consumer_id: cons_id, channel_id, items_consumed: 0, last_consume_ts: 0 });
    }

    pub fn produce(&mut self, prod_id: u64, value: u64, ts: u64) -> bool {
        let producer = match self.producers.get_mut(&prod_id) { Some(p) => p, None => return false };
        let ch_id = producer.channel_id;
        let channel = match self.channels.get_mut(&ch_id) { Some(c) => c, None => return false };
        let ok = channel.write_one(value);
        if ok {
            producer.items_produced += 1;
            producer.last_produce_ts = ts;
        }
        ok
    }

    pub fn consume(&mut self, cons_id: u64, ts: u64) -> Option<u64> {
        let consumer = match self.consumers.get_mut(&cons_id) { Some(c) => c, None => return None };
        let ch_id = consumer.channel_id;
        let channel = match self.channels.get_mut(&ch_id) { Some(c) => c, None => return None };
        let val = channel.read_one()?;
        consumer.items_consumed += 1;
        consumer.last_consume_ts = ts;
        Some(val)
    }

    pub fn channel(&self, id: u32) -> Option<&RingChannel> { self.channels.get(&id) }

    pub fn recompute(&mut self) {
        self.stats.total_channels = self.channels.len();
        self.stats.total_producers = self.producers.len();
        self.stats.total_consumers = self.consumers.len();
        self.stats.total_items_written = self.channels.values().map(|c| c.total_written).sum();
        self.stats.total_items_read = self.channels.values().map(|c| c.total_read).sum();
        self.stats.total_items_dropped = self.channels.values().map(|c| c.total_dropped).sum();
        self.stats.total_items_rejected = self.channels.values().map(|c| c.total_rejected).sum();
        if !self.channels.is_empty() {
            self.stats.avg_fill_ratio = self.channels.values().map(|c| c.fill_ratio()).sum::<f64>() / self.channels.len() as f64;
            self.stats.high_watermark_channels = self.channels.values().filter(|c| c.last_watermark == WatermarkEvent::HighReached).count();
        }
    }

    pub fn stats(&self) -> &RingBufferStats { &self.stats }
}

// ============================================================================
// Merged from ring_buffer_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingStateV2 {
    Empty,
    Partial,
    Full,
    Overrun,
}

/// Ring entry
#[derive(Debug, Clone)]
pub struct RingEntry {
    pub sequence: u64,
    pub timestamp: u64,
    pub data_hash: u64,
    pub size: u32,
}

/// Ring buffer instance
#[derive(Debug)]
pub struct RingBufferV2Instance {
    pub id: u64,
    pub capacity: u32,
    pub entries: Vec<RingEntry>,
    pub head: u32,
    pub tail: u32,
    pub sequence: u64,
    pub total_produced: u64,
    pub total_consumed: u64,
    pub total_dropped: u64,
    pub total_bytes: u64,
}

impl RingBufferV2Instance {
    pub fn new(id: u64, capacity: u32) -> Self {
        Self { id, capacity, entries: Vec::new(), head: 0, tail: 0, sequence: 0, total_produced: 0, total_consumed: 0, total_dropped: 0, total_bytes: 0 }
    }

    pub fn state(&self) -> RingStateV2 {
        let used = self.used();
        if used == 0 { RingStateV2::Empty }
        else if used >= self.capacity { RingStateV2::Full }
        else { RingStateV2::Partial }
    }

    pub fn used(&self) -> u32 { (self.total_produced - self.total_consumed) as u32 }
    pub fn available(&self) -> u32 { self.capacity.saturating_sub(self.used()) }
    pub fn utilization(&self) -> f64 { if self.capacity == 0 { 0.0 } else { self.used() as f64 / self.capacity as f64 } }

    pub fn produce(&mut self, data_hash: u64, size: u32, now: u64) -> bool {
        if self.available() == 0 {
            self.total_dropped += 1;
            return false;
        }
        self.sequence += 1;
        let entry = RingEntry { sequence: self.sequence, timestamp: now, data_hash, size };
        if self.entries.len() < self.capacity as usize { self.entries.push(entry); }
        else { self.entries[self.head as usize % self.capacity as usize] = entry; }
        self.head = (self.head + 1) % self.capacity;
        self.total_produced += 1;
        self.total_bytes += size as u64;
        true
    }

    pub fn consume(&mut self) -> Option<RingEntry> {
        if self.used() == 0 { return None; }
        let idx = self.tail as usize % self.entries.len();
        let entry = self.entries[idx].clone();
        self.tail = (self.tail + 1) % self.capacity;
        self.total_consumed += 1;
        Some(entry)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RingBufferV2Stats {
    pub total_buffers: u32,
    pub total_produced: u64,
    pub total_consumed: u64,
    pub total_dropped: u64,
    pub total_bytes: u64,
    pub avg_utilization: f64,
}

/// Main ring buffer v2 manager
pub struct CoopRingBufferV2 {
    buffers: BTreeMap<u64, RingBufferV2Instance>,
    next_id: u64,
}

impl CoopRingBufferV2 {
    pub fn new() -> Self { Self { buffers: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, capacity: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.buffers.insert(id, RingBufferV2Instance::new(id, capacity));
        id
    }

    pub fn produce(&mut self, id: u64, hash: u64, size: u32, now: u64) -> bool {
        self.buffers.get_mut(&id).map(|b| b.produce(hash, size, now)).unwrap_or(false)
    }

    pub fn consume(&mut self, id: u64) -> Option<RingEntry> {
        self.buffers.get_mut(&id)?.consume()
    }

    pub fn stats(&self) -> RingBufferV2Stats {
        let prod: u64 = self.buffers.values().map(|b| b.total_produced).sum();
        let cons: u64 = self.buffers.values().map(|b| b.total_consumed).sum();
        let drop: u64 = self.buffers.values().map(|b| b.total_dropped).sum();
        let bytes: u64 = self.buffers.values().map(|b| b.total_bytes).sum();
        let utils: Vec<f64> = self.buffers.values().map(|b| b.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        RingBufferV2Stats { total_buffers: self.buffers.len() as u32, total_produced: prod, total_consumed: cons, total_dropped: drop, total_bytes: bytes, avg_utilization: avg }
    }
}

// ============================================================================
// Merged from ring_buffer_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSlotStateV3 {
    Empty,
    Writing,
    Ready,
    Reading,
}

/// Ring buffer entry v3
#[derive(Debug)]
pub struct RingEntryV3 {
    pub sequence: u64,
    pub data_hash: u64,
    pub size: u32,
    pub state: RingSlotStateV3,
    pub timestamp: u64,
}

/// Ring buffer v3
#[derive(Debug)]
pub struct RingBufferV3 {
    pub capacity: u32,
    pub mask: u32,
    pub slots: Vec<RingEntryV3>,
    pub write_pos: u64,
    pub read_pos: u64,
    pub total_written: u64,
    pub total_read: u64,
    pub total_overflows: u64,
    pub total_underflows: u64,
}

impl RingBufferV3 {
    pub fn new(capacity: u32) -> Self {
        let cap = capacity.next_power_of_two();
        let mut slots = Vec::with_capacity(cap as usize);
        for i in 0..cap {
            slots.push(RingEntryV3 { sequence: i as u64, data_hash: 0, size: 0, state: RingSlotStateV3::Empty, timestamp: 0 });
        }
        Self { capacity: cap, mask: cap - 1, slots, write_pos: 0, read_pos: 0, total_written: 0, total_read: 0, total_overflows: 0, total_underflows: 0 }
    }

    pub fn write(&mut self, data_hash: u64, size: u32, now: u64) -> bool {
        if self.write_pos - self.read_pos >= self.capacity as u64 {
            self.total_overflows += 1;
            return false;
        }
        let idx = (self.write_pos & self.mask as u64) as usize;
        self.slots[idx] = RingEntryV3 { sequence: self.write_pos, data_hash, size, state: RingSlotStateV3::Ready, timestamp: now };
        self.write_pos += 1;
        self.total_written += 1;
        true
    }

    pub fn read(&mut self) -> Option<&RingEntryV3> {
        if self.read_pos >= self.write_pos {
            self.total_underflows += 1;
            return None;
        }
        let idx = (self.read_pos & self.mask as u64) as usize;
        self.read_pos += 1;
        self.total_read += 1;
        Some(&self.slots[idx])
    }

    pub fn len(&self) -> u64 { self.write_pos - self.read_pos }
    pub fn is_empty(&self) -> bool { self.read_pos >= self.write_pos }
    pub fn is_full(&self) -> bool { self.write_pos - self.read_pos >= self.capacity as u64 }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.len() as f64 / self.capacity as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RingBufferV3Stats {
    pub capacity: u32,
    pub current_len: u64,
    pub total_written: u64,
    pub total_read: u64,
    pub overflows: u64,
    pub underflows: u64,
    pub utilization: f64,
}

/// Main coop ring buffer v3 manager
pub struct CoopRingBufferV3 {
    total_created: u64,
    total_written: u64,
    total_read: u64,
}

impl CoopRingBufferV3 {
    pub fn new() -> Self { Self { total_created: 0, total_written: 0, total_read: 0 } }

    pub fn create_buffer(&mut self, cap: u32) -> RingBufferV3 {
        self.total_created += 1;
        RingBufferV3::new(cap)
    }

    pub fn record_write(&mut self) { self.total_written += 1; }
    pub fn record_read(&mut self) { self.total_read += 1; }
}
