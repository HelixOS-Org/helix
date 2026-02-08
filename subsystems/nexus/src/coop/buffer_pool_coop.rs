// SPDX-License-Identifier: GPL-2.0
//! Coop buffer pool — cooperative network buffer pooling

extern crate alloc;

/// Buffer pool coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPoolCoopEvent { PoolShare, BufferRecycle, FragmentMerge, SkbClone }

/// Buffer pool coop record
#[derive(Debug, Clone)]
pub struct BufferPoolCoopRecord {
    pub event: BufferPoolCoopEvent,
    pub buffers: u32,
    pub bytes: u64,
    pub pool_utilization_pct: u8,
}

impl BufferPoolCoopRecord {
    pub fn new(event: BufferPoolCoopEvent) -> Self { Self { event, buffers: 0, bytes: 0, pool_utilization_pct: 0 } }
}

/// Buffer pool coop stats
#[derive(Debug, Clone)]
pub struct BufferPoolCoopStats { pub total_events: u64, pub shares: u64, pub recycled: u64, pub bytes_saved: u64 }

/// Main coop buffer pool
#[derive(Debug)]
pub struct CoopBufferPool { pub stats: BufferPoolCoopStats }

impl CoopBufferPool {
    pub fn new() -> Self { Self { stats: BufferPoolCoopStats { total_events: 0, shares: 0, recycled: 0, bytes_saved: 0 } } }
    pub fn record(&mut self, rec: &BufferPoolCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            BufferPoolCoopEvent::PoolShare | BufferPoolCoopEvent::SkbClone => self.stats.shares += 1,
            BufferPoolCoopEvent::BufferRecycle | BufferPoolCoopEvent::FragmentMerge => {
                self.stats.recycled += 1;
                self.stats.bytes_saved += rec.bytes;
            }
        }
    }
}
