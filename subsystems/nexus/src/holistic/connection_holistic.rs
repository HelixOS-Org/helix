// SPDX-License-Identifier: GPL-2.0
//! Holistic connection — holistic connection tracking analysis

extern crate alloc;

/// Connection health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnHealth { Healthy, Idle, HalfOpen, Zombie, Orphaned }

/// Connection holistic record
#[derive(Debug, Clone)]
pub struct ConnectionHolisticRecord {
    pub health: ConnHealth,
    pub active_conns: u32,
    pub idle_conns: u32,
    pub age_ms: u64,
}

impl ConnectionHolisticRecord {
    pub fn new(health: ConnHealth) -> Self { Self { health, active_conns: 0, idle_conns: 0, age_ms: 0 } }
}

/// Connection holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConnectionHolisticStats { pub total_samples: u64, pub zombies: u64, pub orphaned: u64, pub peak_active: u32 }

/// Main holistic connection
#[derive(Debug)]
pub struct HolisticConnection { pub stats: ConnectionHolisticStats }

impl HolisticConnection {
    pub fn new() -> Self { Self { stats: ConnectionHolisticStats { total_samples: 0, zombies: 0, orphaned: 0, peak_active: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ConnectionHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.health {
            ConnHealth::Zombie | ConnHealth::HalfOpen => self.stats.zombies += 1,
            ConnHealth::Orphaned => self.stats.orphaned += 1,
            _ => {}
        }
        if rec.active_conns > self.stats.peak_active { self.stats.peak_active = rec.active_conns; }
    }
}
