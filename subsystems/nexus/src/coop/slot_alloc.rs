//! # Coop Slot Allocator
//!
//! Distributed slot allocation for time-division cooperative scheduling:
//! - Time slot assignment across nodes
//! - Slot contention resolution
//! - Hierarchical slot subdivision
//! - Slot borrowing and lending
//! - Utilization-based reallocation
//! - Guard band management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Slot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Free,
    Assigned,
    Active,
    Guard,
    Borrowed,
    Reserved,
}

/// Slot priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotPriority {
    Background = 0,
    Normal = 1,
    High = 2,
    Realtime = 3,
    Critical = 4,
}

/// Time slot
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub id: u64,
    pub frame_id: u64,
    pub offset_ns: u64,
    pub duration_ns: u64,
    pub guard_ns: u64,
    pub state: SlotState,
    pub owner_node: Option<u64>,
    pub priority: SlotPriority,
    pub utilization: f64,
    pub used_ns: u64,
    pub wasted_ns: u64,
    pub borrowed_from: Option<u64>,
    pub borrowed_to: Option<u64>,
}

impl TimeSlot {
    pub fn new(id: u64, frame: u64, offset: u64, duration: u64) -> Self {
        Self {
            id, frame_id: frame, offset_ns: offset, duration_ns: duration,
            guard_ns: duration / 20, state: SlotState::Free, owner_node: None,
            priority: SlotPriority::Normal, utilization: 0.0, used_ns: 0,
            wasted_ns: 0, borrowed_from: None, borrowed_to: None,
        }
    }

    #[inline(always)]
    pub fn usable_ns(&self) -> u64 { self.duration_ns.saturating_sub(self.guard_ns) }

    #[inline]
    pub fn assign(&mut self, node: u64, priority: SlotPriority) {
        self.owner_node = Some(node);
        self.priority = priority;
        self.state = SlotState::Assigned;
    }

    #[inline(always)]
    pub fn activate(&mut self) { self.state = SlotState::Active; }

    #[inline]
    pub fn release(&mut self) {
        self.owner_node = None;
        self.state = SlotState::Free;
        self.borrowed_from = None;
        self.borrowed_to = None;
    }

    #[inline(always)]
    pub fn lend_to(&mut self, borrower: u64) {
        self.borrowed_to = Some(borrower);
        self.state = SlotState::Borrowed;
    }

    #[inline(always)]
    pub fn borrow_from(&mut self, lender: u64) {
        self.borrowed_from = Some(lender);
        self.state = SlotState::Borrowed;
    }

    #[inline]
    pub fn record_usage(&mut self, used: u64) {
        self.used_ns += used;
        let usable = self.usable_ns();
        if used < usable { self.wasted_ns += usable - used; }
        self.utilization = if usable == 0 { 0.0 } else { used as f64 / usable as f64 };
    }

    #[inline(always)]
    pub fn end_ns(&self) -> u64 { self.offset_ns + self.duration_ns }
}

/// Slot frame (collection of slots in one period)
#[derive(Debug, Clone)]
pub struct SlotFrame {
    pub id: u64,
    pub period_ns: u64,
    pub slots: Vec<u64>,
    pub total_slots: u32,
    pub assigned_slots: u32,
    pub epoch: u64,
}

impl SlotFrame {
    pub fn new(id: u64, period_ns: u64, num_slots: u32) -> Self {
        Self { id, period_ns, slots: Vec::new(), total_slots: num_slots, assigned_slots: 0, epoch: 0 }
    }

    #[inline(always)]
    pub fn slot_duration(&self) -> u64 {
        if self.total_slots == 0 { return 0; }
        self.period_ns / self.total_slots as u64
    }
}

/// Per-node allocation
#[derive(Debug, Clone)]
pub struct NodeAllocation {
    pub node_id: u64,
    pub assigned_slots: Vec<u64>,
    pub borrowed_slots: Vec<u64>,
    pub lent_slots: Vec<u64>,
    pub total_used_ns: u64,
    pub total_wasted_ns: u64,
    pub avg_utilization: f64,
    pub weight: u32,
}

impl NodeAllocation {
    pub fn new(node_id: u64, weight: u32) -> Self {
        Self {
            node_id, assigned_slots: Vec::new(), borrowed_slots: Vec::new(),
            lent_slots: Vec::new(), total_used_ns: 0, total_wasted_ns: 0,
            avg_utilization: 0.0, weight,
        }
    }

    #[inline(always)]
    pub fn fair_share(&self, total_weight: u32, total_slots: u32) -> u32 {
        if total_weight == 0 { return 0; }
        ((self.weight as u64 * total_slots as u64) / total_weight as u64) as u32
    }
}

/// Slot allocator stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SlotAllocStats {
    pub total_frames: usize,
    pub total_slots: usize,
    pub free_slots: usize,
    pub assigned_slots: usize,
    pub borrowed_slots: usize,
    pub total_nodes: usize,
    pub avg_utilization: f64,
    pub total_used_ns: u64,
    pub total_wasted_ns: u64,
}

/// Coop slot allocator
pub struct CoopSlotAllocator {
    frames: BTreeMap<u64, SlotFrame>,
    slots: BTreeMap<u64, TimeSlot>,
    nodes: BTreeMap<u64, NodeAllocation>,
    stats: SlotAllocStats,
    next_id: u64,
}

impl CoopSlotAllocator {
    pub fn new() -> Self {
        Self { frames: BTreeMap::new(), slots: BTreeMap::new(), nodes: BTreeMap::new(), stats: SlotAllocStats::default(), next_id: 1 }
    }

    pub fn create_frame(&mut self, period_ns: u64, num_slots: u32) -> u64 {
        let frame_id = self.next_id; self.next_id += 1;
        let mut frame = SlotFrame::new(frame_id, period_ns, num_slots);
        let slot_dur = frame.slot_duration();
        for i in 0..num_slots {
            let slot_id = self.next_id; self.next_id += 1;
            let offset = i as u64 * slot_dur;
            self.slots.insert(slot_id, TimeSlot::new(slot_id, frame_id, offset, slot_dur));
            frame.slots.push(slot_id);
        }
        self.frames.insert(frame_id, frame);
        frame_id
    }

    #[inline(always)]
    pub fn add_node(&mut self, node_id: u64, weight: u32) {
        self.nodes.entry(node_id).or_insert_with(|| NodeAllocation::new(node_id, weight));
    }

    #[inline]
    pub fn assign_slot(&mut self, slot_id: u64, node_id: u64, priority: SlotPriority) -> bool {
        if let Some(slot) = self.slots.get_mut(&slot_id) {
            if slot.state != SlotState::Free { return false; }
            slot.assign(node_id, priority);
            if let Some(n) = self.nodes.get_mut(&node_id) { n.assigned_slots.push(slot_id); }
            return true;
        }
        false
    }

    #[inline]
    pub fn release_slot(&mut self, slot_id: u64) {
        if let Some(slot) = self.slots.get_mut(&slot_id) {
            let owner = slot.owner_node;
            slot.release();
            if let Some(nid) = owner {
                if let Some(n) = self.nodes.get_mut(&nid) { n.assigned_slots.retain(|&s| s != slot_id); }
            }
        }
    }

    #[inline]
    pub fn lend_slot(&mut self, slot_id: u64, borrower: u64) -> bool {
        if let Some(slot) = self.slots.get_mut(&slot_id) {
            if slot.state != SlotState::Assigned { return false; }
            let lender = slot.owner_node.unwrap_or(0);
            slot.lend_to(borrower);
            if let Some(n) = self.nodes.get_mut(&lender) { n.lent_slots.push(slot_id); }
            if let Some(n) = self.nodes.get_mut(&borrower) { n.borrowed_slots.push(slot_id); }
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn record_usage(&mut self, slot_id: u64, used_ns: u64) {
        if let Some(slot) = self.slots.get_mut(&slot_id) { slot.record_usage(used_ns); }
    }

    pub fn fair_allocate(&mut self, frame_id: u64) {
        let total_weight: u32 = self.nodes.values().map(|n| n.weight).sum();
        let frame_slots: Vec<u64> = self.frames.get(&frame_id).map(|f| f.slots.clone()).unwrap_or_default();
        let free: Vec<u64> = frame_slots.iter().filter(|&&s| self.slots.get(&s).map_or(false, |sl| sl.state == SlotState::Free)).copied().collect();
        let total_free = free.len() as u32;
        let mut idx = 0;
        let mut sorted_nodes: Vec<(&u64, &NodeAllocation)> = self.nodes.iter().collect();
        sorted_nodes.sort_by(|a, b| b.1.weight.cmp(&a.1.weight));
        for (&nid, na) in &sorted_nodes {
            let share = na.fair_share(total_weight, total_free);
            for _ in 0..share {
                if idx >= free.len() { break; }
                if let Some(slot) = self.slots.get_mut(&free[idx]) {
                    slot.assign(nid, SlotPriority::Normal);
                }
                idx += 1;
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_frames = self.frames.len();
        self.stats.total_slots = self.slots.len();
        self.stats.free_slots = self.slots.values().filter(|s| s.state == SlotState::Free).count();
        self.stats.assigned_slots = self.slots.values().filter(|s| s.state == SlotState::Assigned || s.state == SlotState::Active).count();
        self.stats.borrowed_slots = self.slots.values().filter(|s| s.state == SlotState::Borrowed).count();
        self.stats.total_nodes = self.nodes.len();
        let utils: Vec<f64> = self.slots.values().filter(|s| s.state != SlotState::Free).map(|s| s.utilization).collect();
        self.stats.avg_utilization = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        self.stats.total_used_ns = self.slots.values().map(|s| s.used_ns).sum();
        self.stats.total_wasted_ns = self.slots.values().map(|s| s.wasted_ns).sum();
    }

    #[inline(always)]
    pub fn slot(&self, id: u64) -> Option<&TimeSlot> { self.slots.get(&id) }
    #[inline(always)]
    pub fn frame(&self, id: u64) -> Option<&SlotFrame> { self.frames.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &SlotAllocStats { &self.stats }
}
