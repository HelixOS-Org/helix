// SPDX-License-Identifier: GPL-2.0
//! Holistic socket — holistic socket lifecycle analysis

extern crate alloc;

use alloc::collections::BTreeMap;

/// Socket lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketLifecycle { Created, Bound, Listening, Connected, Closing, Leaked }

/// Socket holistic record
#[derive(Debug, Clone)]
pub struct SocketHolisticRecord {
    pub lifecycle: SocketLifecycle,
    pub fd: i32,
    pub age_ms: u64,
    pub bytes_total: u64,
}

impl SocketHolisticRecord {
    pub fn new(lifecycle: SocketLifecycle) -> Self { Self { lifecycle, fd: -1, age_ms: 0, bytes_total: 0 } }
}

/// Socket holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketHolisticStats { pub total_samples: u64, pub active: u32, pub leaked: u64, pub state_counts: BTreeMap<u8, u64> }

/// Main holistic socket
#[derive(Debug)]
pub struct HolisticSocket { pub stats: SocketHolisticStats }

impl HolisticSocket {
    pub fn new() -> Self { Self { stats: SocketHolisticStats { total_samples: 0, active: 0, leaked: 0, state_counts: BTreeMap::new() } } }
    #[inline]
    pub fn record(&mut self, rec: &SocketHolisticRecord) {
        self.stats.total_samples += 1;
        let key = rec.lifecycle as u8;
        *self.stats.state_counts.entry(key).or_insert(0) += 1;
        if rec.lifecycle == SocketLifecycle::Leaked { self.stats.leaked += 1; }
    }
}
