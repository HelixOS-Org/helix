//! # Holistic Bandwidth Manager
//!
//! System-wide bandwidth management for IO and network:
//! - Per-device bandwidth tracking
//! - Token bucket rate limiters
//! - Bandwidth reservation and guarantees
//! - Proportional sharing with min guarantees
//! - Burst management
//! - Congestion detection and backpressure

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Bandwidth resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BwResource {
    DiskRead,
    DiskWrite,
    NetworkTx,
    NetworkRx,
    PcieBandwidth,
    MemoryBandwidth,
}

/// Congestion level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel {
    None,
    Light,
    Moderate,
    Heavy,
    Critical,
}

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub rate_bps: u64,       // bytes per second
    pub burst_bytes: u64,
    pub tokens: u64,
    pub last_fill_ns: u64,
    pub total_consumed: u64,
    pub total_throttled: u64,
}

impl TokenBucket {
    pub fn new(rate_bps: u64, burst_bytes: u64) -> Self {
        Self {
            rate_bps,
            burst_bytes,
            tokens: burst_bytes,
            last_fill_ns: 0,
            total_consumed: 0,
            total_throttled: 0,
        }
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, now_ns: u64) {
        if self.last_fill_ns == 0 {
            self.last_fill_ns = now_ns;
            return;
        }
        let elapsed_ns = now_ns.saturating_sub(self.last_fill_ns);
        let new_tokens = (self.rate_bps as u128 * elapsed_ns as u128 / 1_000_000_000) as u64;
        self.tokens = (self.tokens + new_tokens).min(self.burst_bytes);
        self.last_fill_ns = now_ns;
    }

    /// Try to consume bytes. Returns actual consumed.
    pub fn consume(&mut self, bytes: u64) -> u64 {
        let consumed = bytes.min(self.tokens);
        self.tokens -= consumed;
        self.total_consumed += consumed;
        if consumed < bytes {
            self.total_throttled += bytes - consumed;
        }
        consumed
    }

    pub fn available(&self) -> u64 {
        self.tokens
    }

    pub fn utilization(&self) -> f64 {
        if self.burst_bytes == 0 { return 0.0; }
        1.0 - (self.tokens as f64 / self.burst_bytes as f64)
    }
}

/// Per-device bandwidth state
#[derive(Debug, Clone)]
pub struct DeviceBandwidth {
    pub device_id: u32,
    pub resource: BwResource,
    pub max_bps: u64,
    pub current_bps: u64,
    pub rate_limiter: TokenBucket,
    pub guaranteed_bps: u64,
    pub reserved_bps: u64,
    pub congestion: CongestionLevel,
    pub queue_depth: u32,
}

impl DeviceBandwidth {
    pub fn new(device_id: u32, resource: BwResource, max_bps: u64) -> Self {
        let burst = max_bps / 10; // 100ms burst
        Self {
            device_id,
            resource,
            max_bps,
            current_bps: 0,
            rate_limiter: TokenBucket::new(max_bps, burst),
            guaranteed_bps: 0,
            reserved_bps: 0,
            congestion: CongestionLevel::None,
            queue_depth: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_bps == 0 { return 0.0; }
        self.current_bps as f64 / self.max_bps as f64
    }

    pub fn update_congestion(&mut self) {
        let util = self.utilization();
        self.congestion = if util < 0.5 { CongestionLevel::None }
            else if util < 0.7 { CongestionLevel::Light }
            else if util < 0.85 { CongestionLevel::Moderate }
            else if util < 0.95 { CongestionLevel::Heavy }
            else { CongestionLevel::Critical };
    }

    pub fn available_bps(&self) -> u64 {
        self.max_bps.saturating_sub(self.reserved_bps)
    }
}

/// Bandwidth reservation
#[derive(Debug, Clone)]
pub struct BwReservation {
    pub reservation_id: u64,
    pub device_id: u32,
    pub owner_pid: u64,
    pub reserved_bps: u64,
    pub used_bps: u64,
    pub expiry_ns: u64,
}

impl BwReservation {
    pub fn is_expired(&self, now_ns: u64) -> bool {
        self.expiry_ns > 0 && now_ns > self.expiry_ns
    }

    pub fn utilization(&self) -> f64 {
        if self.reserved_bps == 0 { return 0.0; }
        self.used_bps as f64 / self.reserved_bps as f64
    }
}

/// Per-entity bandwidth share
#[derive(Debug, Clone)]
pub struct BwShare {
    pub entity_id: u64,
    pub weight: u32,
    pub min_bps: u64,
    pub max_bps: u64,
    pub current_bps: u64,
    pub bucket: TokenBucket,
}

impl BwShare {
    pub fn new(entity_id: u64, weight: u32, max_bps: u64) -> Self {
        let burst = max_bps / 10;
        Self {
            entity_id,
            weight,
            min_bps: 0,
            max_bps,
            current_bps: 0,
            bucket: TokenBucket::new(max_bps, burst),
        }
    }
}

/// Bandwidth manager stats
#[derive(Debug, Clone, Default)]
pub struct HolisticBandwidthMgrStats {
    pub tracked_devices: usize,
    pub active_reservations: usize,
    pub active_shares: usize,
    pub total_bandwidth_bps: u64,
    pub used_bandwidth_bps: u64,
    pub congested_devices: usize,
    pub total_throttled_bytes: u64,
}

/// Holistic Bandwidth Manager
pub struct HolisticBandwidthMgr {
    devices: BTreeMap<u32, DeviceBandwidth>,
    reservations: BTreeMap<u64, BwReservation>,
    shares: BTreeMap<u64, BwShare>,
    next_reservation_id: u64,
    stats: HolisticBandwidthMgrStats,
}

impl HolisticBandwidthMgr {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            reservations: BTreeMap::new(),
            shares: BTreeMap::new(),
            next_reservation_id: 1,
            stats: HolisticBandwidthMgrStats::default(),
        }
    }

    pub fn add_device(&mut self, device: DeviceBandwidth) {
        self.devices.insert(device.device_id, device);
        self.recompute();
    }

    pub fn update_device_rate(&mut self, device_id: u32, current_bps: u64, queue_depth: u32) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.current_bps = current_bps;
            dev.queue_depth = queue_depth;
            dev.update_congestion();
        }
    }

    /// Reserve bandwidth on a device
    pub fn reserve(&mut self, device_id: u32, owner_pid: u64, bps: u64, duration_ns: u64, now_ns: u64) -> Option<u64> {
        let device = self.devices.get_mut(&device_id)?;
        if device.available_bps() < bps { return None; }

        device.reserved_bps += bps;

        let id = self.next_reservation_id;
        self.next_reservation_id += 1;

        self.reservations.insert(id, BwReservation {
            reservation_id: id,
            device_id,
            owner_pid,
            reserved_bps: bps,
            used_bps: 0,
            expiry_ns: if duration_ns > 0 { now_ns + duration_ns } else { 0 },
        });

        self.recompute();
        Some(id)
    }

    /// Release a reservation
    pub fn release_reservation(&mut self, reservation_id: u64) -> bool {
        if let Some(res) = self.reservations.remove(&reservation_id) {
            if let Some(dev) = self.devices.get_mut(&res.device_id) {
                dev.reserved_bps = dev.reserved_bps.saturating_sub(res.reserved_bps);
            }
            self.recompute();
            true
        } else { false }
    }

    /// Expire old reservations
    pub fn expire_reservations(&mut self, now_ns: u64) {
        let expired: Vec<u64> = self.reservations.iter()
            .filter(|(_, r)| r.is_expired(now_ns))
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.release_reservation(id);
        }
    }

    /// Add a proportional share
    pub fn add_share(&mut self, share: BwShare) {
        self.shares.insert(share.entity_id, share);
        self.recompute();
    }

    /// Try to consume bandwidth for a share
    pub fn consume_share(&mut self, entity_id: u64, bytes: u64, now_ns: u64) -> u64 {
        if let Some(share) = self.shares.get_mut(&entity_id) {
            share.bucket.refill(now_ns);
            share.bucket.consume(bytes)
        } else { 0 }
    }

    /// Recompute proportional shares based on weights
    pub fn rebalance_shares(&mut self, total_available_bps: u64) {
        let total_weight: u32 = self.shares.values().map(|s| s.weight).sum();
        if total_weight == 0 { return; }

        for share in self.shares.values_mut() {
            let proportion = share.weight as f64 / total_weight as f64;
            let allocated = (total_available_bps as f64 * proportion) as u64;
            let effective = allocated.max(share.min_bps).min(share.max_bps);
            share.bucket.rate_bps = effective;
        }
    }

    /// Get congested devices
    pub fn congested_devices(&self) -> Vec<u32> {
        self.devices.values()
            .filter(|d| matches!(d.congestion, CongestionLevel::Heavy | CongestionLevel::Critical))
            .map(|d| d.device_id)
            .collect()
    }

    fn recompute(&mut self) {
        let total_bw: u64 = self.devices.values().map(|d| d.max_bps).sum();
        let used_bw: u64 = self.devices.values().map(|d| d.current_bps).sum();
        let congested = self.devices.values()
            .filter(|d| !matches!(d.congestion, CongestionLevel::None | CongestionLevel::Light))
            .count();
        let throttled: u64 = self.shares.values().map(|s| s.bucket.total_throttled).sum();

        self.stats = HolisticBandwidthMgrStats {
            tracked_devices: self.devices.len(),
            active_reservations: self.reservations.len(),
            active_shares: self.shares.len(),
            total_bandwidth_bps: total_bw,
            used_bandwidth_bps: used_bw,
            congested_devices: congested,
            total_throttled_bytes: throttled,
        };
    }

    pub fn stats(&self) -> &HolisticBandwidthMgrStats {
        &self.stats
    }

    pub fn device(&self, device_id: u32) -> Option<&DeviceBandwidth> {
        self.devices.get(&device_id)
    }
}
