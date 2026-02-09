//! # Cooperative Timeline
//!
//! Temporal coordination for cooperative scheduling:
//! - Timeline-based resource reservation
//! - Slot allocation and management
//! - Temporal conflict detection
//! - Priority-aware scheduling windows
//! - Periodic and one-shot reservations

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TIMELINE TYPES
// ============================================================================

/// Resource type for timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimelineResource {
    /// CPU core
    CpuCore,
    /// Memory bandwidth
    MemoryBandwidth,
    /// I/O channel
    IoChannel,
    /// Network link
    NetworkLink,
    /// GPU engine
    GpuEngine,
}

/// Reservation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationType {
    /// One-shot
    OneShot,
    /// Periodic
    Periodic,
    /// Best-effort
    BestEffort,
}

/// Reservation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    /// Pending (not yet active)
    Pending,
    /// Active
    Active,
    /// Completed
    Completed,
    /// Cancelled
    Cancelled,
    /// Conflicted (overlapping)
    Conflicted,
}

// ============================================================================
// TIME SLOT
// ============================================================================

/// A time slot on the timeline
#[derive(Debug, Clone)]
pub struct TimeSlot {
    /// Start time (ns)
    pub start: u64,
    /// End time (ns)
    pub end: u64,
}

impl TimeSlot {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// Duration
    #[inline(always)]
    pub fn duration(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Overlaps with another slot
    #[inline(always)]
    pub fn overlaps(&self, other: &TimeSlot) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Contains a timestamp
    #[inline(always)]
    pub fn contains(&self, timestamp: u64) -> bool {
        timestamp >= self.start && timestamp < self.end
    }

    /// Merge with another overlapping slot
    #[inline]
    pub fn merge(&self, other: &TimeSlot) -> TimeSlot {
        TimeSlot {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

// ============================================================================
// RESERVATION
// ============================================================================

/// Timeline reservation
#[derive(Debug, Clone)]
pub struct TimelineReservation {
    /// Reservation ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Resource
    pub resource: TimelineResource,
    /// Resource instance (core number, channel, etc.)
    pub instance: u32,
    /// Slot
    pub slot: TimeSlot,
    /// Reservation type
    pub reservation_type: ReservationType,
    /// Period (ns, for periodic)
    pub period_ns: u64,
    /// State
    pub state: ReservationState,
    /// Priority (higher = more important)
    pub priority: u32,
    /// Utilization during slot (fraction)
    pub utilization: f64,
}

impl TimelineReservation {
    pub fn new(
        id: u64,
        pid: u64,
        resource: TimelineResource,
        instance: u32,
        slot: TimeSlot,
        reservation_type: ReservationType,
        priority: u32,
    ) -> Self {
        Self {
            id,
            pid,
            resource,
            instance,
            slot,
            reservation_type,
            period_ns: 0,
            state: ReservationState::Pending,
            priority,
            utilization: 1.0,
        }
    }

    /// Generate next periodic slot
    #[inline]
    pub fn next_periodic_slot(&self) -> Option<TimeSlot> {
        if self.reservation_type != ReservationType::Periodic || self.period_ns == 0 {
            return None;
        }
        let duration = self.slot.duration();
        Some(TimeSlot::new(
            self.slot.start + self.period_ns,
            self.slot.start + self.period_ns + duration,
        ))
    }

    /// Is active at timestamp
    #[inline(always)]
    pub fn is_active_at(&self, timestamp: u64) -> bool {
        self.state == ReservationState::Active && self.slot.contains(timestamp)
    }
}

// ============================================================================
// TIMELINE
// ============================================================================

/// Resource timeline
#[derive(Debug, Clone)]
pub struct ResourceTimeline {
    /// Resource type
    pub resource: TimelineResource,
    /// Instance
    pub instance: u32,
    /// Reservations sorted by start time
    reservations: Vec<u64>,
    /// Total reserved time (ns)
    pub total_reserved_ns: u64,
}

impl ResourceTimeline {
    pub fn new(resource: TimelineResource, instance: u32) -> Self {
        Self {
            resource,
            instance,
            reservations: Vec::new(),
            total_reserved_ns: 0,
        }
    }

    /// Add reservation
    #[inline(always)]
    pub fn add_reservation(&mut self, id: u64, duration_ns: u64) {
        self.reservations.push(id);
        self.total_reserved_ns += duration_ns;
    }

    /// Remove reservation
    #[inline(always)]
    pub fn remove_reservation(&mut self, id: u64, duration_ns: u64) {
        self.reservations.retain(|&r| r != id);
        self.total_reserved_ns = self.total_reserved_ns.saturating_sub(duration_ns);
    }

    /// Reservation count
    #[inline(always)]
    pub fn reservation_count(&self) -> usize {
        self.reservations.len()
    }
}

// ============================================================================
// CONFLICT
// ============================================================================

/// Timeline conflict
#[derive(Debug, Clone)]
pub struct TimelineConflict {
    /// First reservation
    pub reservation_a: u64,
    /// Second reservation
    pub reservation_b: u64,
    /// Overlap start
    pub overlap_start: u64,
    /// Overlap end
    pub overlap_end: u64,
    /// Resource
    pub resource: TimelineResource,
    /// Instance
    pub instance: u32,
}

impl TimelineConflict {
    /// Overlap duration
    #[inline(always)]
    pub fn overlap_duration(&self) -> u64 {
        self.overlap_end.saturating_sub(self.overlap_start)
    }
}

// ============================================================================
// TIMELINE MANAGER
// ============================================================================

/// Timeline manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopTimelineStats {
    /// Active reservations
    pub active_reservations: usize,
    /// Total reservations
    pub total_reservations: u64,
    /// Conflicts detected
    pub conflicts_detected: u64,
    /// Timelines managed
    pub timeline_count: usize,
    /// Total reserved time (ns)
    pub total_reserved_ns: u64,
}

/// Cooperative timeline manager
pub struct CoopTimelineManager {
    /// Reservations
    reservations: BTreeMap<u64, TimelineReservation>,
    /// Resource timelines
    timelines: BTreeMap<(u8, u32), ResourceTimeline>,
    /// Conflicts
    conflicts: Vec<TimelineConflict>,
    /// Next reservation ID
    next_id: u64,
    /// Stats
    stats: CoopTimelineStats,
}

impl CoopTimelineManager {
    pub fn new() -> Self {
        Self {
            reservations: BTreeMap::new(),
            timelines: BTreeMap::new(),
            conflicts: Vec::new(),
            next_id: 1,
            stats: CoopTimelineStats::default(),
        }
    }

    /// Create reservation
    pub fn create_reservation(
        &mut self,
        pid: u64,
        resource: TimelineResource,
        instance: u32,
        start: u64,
        end: u64,
        reservation_type: ReservationType,
        priority: u32,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let slot = TimeSlot::new(start, end);
        let duration = slot.duration();
        let reservation = TimelineReservation::new(
            id, pid, resource, instance, slot, reservation_type, priority,
        );
        self.reservations.insert(id, reservation);

        let tl_key = (resource as u8, instance);
        let timeline = self
            .timelines
            .entry(tl_key)
            .or_insert_with(|| ResourceTimeline::new(resource, instance));
        timeline.add_reservation(id, duration);

        self.stats.total_reservations += 1;
        self.stats.total_reserved_ns += duration;
        self.stats.timeline_count = self.timelines.len();
        self.update_active();
        id
    }

    /// Activate reservation
    #[inline]
    pub fn activate(&mut self, id: u64) {
        if let Some(r) = self.reservations.get_mut(&id) {
            r.state = ReservationState::Active;
        }
        self.update_active();
    }

    /// Complete reservation
    #[inline]
    pub fn complete(&mut self, id: u64) {
        if let Some(r) = self.reservations.get_mut(&id) {
            r.state = ReservationState::Completed;
        }
        self.update_active();
    }

    /// Cancel reservation
    #[inline]
    pub fn cancel(&mut self, id: u64) {
        if let Some(r) = self.reservations.get_mut(&id) {
            r.state = ReservationState::Cancelled;
            let tl_key = (r.resource as u8, r.instance);
            let duration = r.slot.duration();
            if let Some(tl) = self.timelines.get_mut(&tl_key) {
                tl.remove_reservation(id, duration);
            }
        }
        self.update_active();
    }

    /// Detect conflicts for a resource/instance
    pub fn detect_conflicts(&mut self, resource: TimelineResource, instance: u32) -> Vec<TimelineConflict> {
        let mut active: Vec<&TimelineReservation> = self
            .reservations
            .values()
            .filter(|r| {
                r.resource == resource
                    && r.instance == instance
                    && matches!(r.state, ReservationState::Active | ReservationState::Pending)
            })
            .collect();

        active.sort_by_key(|r| r.slot.start);

        let mut conflicts = Vec::new();
        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                if active[i].slot.overlaps(&active[j].slot) {
                    let overlap_start = active[i].slot.start.max(active[j].slot.start);
                    let overlap_end = active[i].slot.end.min(active[j].slot.end);
                    conflicts.push(TimelineConflict {
                        reservation_a: active[i].id,
                        reservation_b: active[j].id,
                        overlap_start,
                        overlap_end,
                        resource,
                        instance,
                    });
                }
            }
        }

        self.stats.conflicts_detected += conflicts.len() as u64;
        self.conflicts.extend(conflicts.clone());
        conflicts
    }

    /// Get active reservations at timestamp
    #[inline]
    pub fn active_at(&self, resource: TimelineResource, instance: u32, timestamp: u64) -> Vec<&TimelineReservation> {
        self.reservations
            .values()
            .filter(|r| {
                r.resource == resource
                    && r.instance == instance
                    && r.is_active_at(timestamp)
            })
            .collect()
    }

    fn update_active(&mut self) {
        self.stats.active_reservations = self
            .reservations
            .values()
            .filter(|r| r.state == ReservationState::Active)
            .count();
    }

    /// Get reservation
    #[inline(always)]
    pub fn reservation(&self, id: u64) -> Option<&TimelineReservation> {
        self.reservations.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopTimelineStats {
        &self.stats
    }
}
