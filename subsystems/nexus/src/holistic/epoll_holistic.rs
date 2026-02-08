// SPDX-License-Identifier: GPL-2.0
//! Holistic epoll — holistic epoll scalability analysis

extern crate alloc;

/// Epoll scalability grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollScalability { Linear, Sublinear, Bottlenecked, Thrashing }

/// Epoll holistic record
#[derive(Debug, Clone)]
pub struct EpollHolisticRecord {
    pub scalability: EpollScalability,
    pub fd_count: u32,
    pub ready_ratio: u8,
    pub wait_latency_us: u64,
}

impl EpollHolisticRecord {
    pub fn new(scalability: EpollScalability) -> Self { Self { scalability, fd_count: 0, ready_ratio: 0, wait_latency_us: 0 } }
}

/// Epoll holistic stats
#[derive(Debug, Clone)]
pub struct EpollHolisticStats { pub total_samples: u64, pub bottlenecks: u64, pub max_fds: u32, pub avg_latency_us: u64 }

/// Main holistic epoll
#[derive(Debug)]
pub struct HolisticEpoll {
    pub stats: EpollHolisticStats,
    latency_sum: u64,
}

impl HolisticEpoll {
    pub fn new() -> Self { Self { stats: EpollHolisticStats { total_samples: 0, bottlenecks: 0, max_fds: 0, avg_latency_us: 0 }, latency_sum: 0 } }
    pub fn record(&mut self, rec: &EpollHolisticRecord) {
        self.stats.total_samples += 1;
        if rec.scalability == EpollScalability::Bottlenecked || rec.scalability == EpollScalability::Thrashing { self.stats.bottlenecks += 1; }
        if rec.fd_count > self.stats.max_fds { self.stats.max_fds = rec.fd_count; }
        self.latency_sum += rec.wait_latency_us;
        self.stats.avg_latency_us = self.latency_sum / self.stats.total_samples;
    }
}
