// SPDX-License-Identifier: GPL-2.0
//! Bridge fanotify_bridge — filesystem-wide event notification.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fanotify event mask bits
#[derive(Debug, Clone, Copy)]
pub struct FanEventMask {
    pub bits: u64,
}

impl FanEventMask {
    pub const ACCESS: u64 = 1 << 0;
    pub const MODIFY: u64 = 1 << 1;
    pub const CLOSE_WRITE: u64 = 1 << 2;
    pub const CLOSE_NOWRITE: u64 = 1 << 3;
    pub const OPEN: u64 = 1 << 4;
    pub const OPEN_EXEC: u64 = 1 << 5;
    pub const ATTRIB: u64 = 1 << 6;
    pub const CREATE: u64 = 1 << 7;
    pub const DELETE: u64 = 1 << 8;
    pub const DELETE_SELF: u64 = 1 << 9;
    pub const MOVE_FROM: u64 = 1 << 10;
    pub const MOVE_TO: u64 = 1 << 11;
    pub const RENAME: u64 = 1 << 12;
    pub const OPEN_PERM: u64 = 1 << 16;
    pub const ACCESS_PERM: u64 = 1 << 17;
    pub const OPEN_EXEC_PERM: u64 = 1 << 18;

    pub fn new(bits: u64) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u64) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn is_permission(&self) -> bool {
        self.has(Self::OPEN_PERM) || self.has(Self::ACCESS_PERM) || self.has(Self::OPEN_EXEC_PERM)
    }
}

/// Fanotify init flags
#[derive(Debug, Clone, Copy)]
pub struct FanInitFlags {
    pub bits: u32,
}

impl FanInitFlags {
    pub const CLASS_NOTIF: u32 = 0;
    pub const CLASS_CONTENT: u32 = 1;
    pub const CLASS_PRE_CONTENT: u32 = 2;
    pub const UNLIMITED_QUEUE: u32 = 1 << 4;
    pub const UNLIMITED_MARKS: u32 = 1 << 5;
    pub const REPORT_FID: u32 = 1 << 9;
    pub const REPORT_DIR_FID: u32 = 1 << 10;
    pub const REPORT_NAME: u32 = 1 << 11;
    pub const REPORT_PIDFD: u32 = 1 << 12;

    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
}

/// Fanotify mark type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanMarkType {
    Inode,
    Mount,
    Filesystem,
}

/// Fanotify permission response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanPermResponse {
    Allow,
    Deny,
    Pending,
}

/// Mark entry (watch descriptor)
#[derive(Debug, Clone)]
pub struct FanMark {
    pub id: u64,
    pub mark_type: FanMarkType,
    pub mask: FanEventMask,
    pub target_id: u64,
    pub ignore_mask: FanEventMask,
    pub events_delivered: u64,
}

impl FanMark {
    pub fn new(id: u64, mtype: FanMarkType, mask: FanEventMask, target: u64) -> Self {
        Self {
            id, mark_type: mtype, mask, target_id: target,
            ignore_mask: FanEventMask::new(0), events_delivered: 0,
        }
    }

    #[inline(always)]
    pub fn matches(&self, event_mask: u64) -> bool {
        (self.mask.bits & event_mask) != 0 && (self.ignore_mask.bits & event_mask) == 0
    }
}

/// Fanotify group (per-fd instance)
#[derive(Debug)]
pub struct FanotifyGroup {
    pub id: u64,
    pub flags: FanInitFlags,
    pub marks: Vec<FanMark>,
    pub queue: Vec<FanotifyEvent>,
    pub queue_max: u32,
    pub total_events: u64,
    pub overflow_count: u64,
    pub permission_pending: u32,
}

impl FanotifyGroup {
    pub fn new(id: u64, flags: FanInitFlags) -> Self {
        Self {
            id, flags, marks: Vec::new(), queue: Vec::new(),
            queue_max: 16384, total_events: 0, overflow_count: 0,
            permission_pending: 0,
        }
    }

    #[inline(always)]
    pub fn add_mark(&mut self, mark: FanMark) { self.marks.push(mark); }

    #[inline]
    pub fn deliver(&mut self, event: FanotifyEvent) -> bool {
        if self.queue.len() as u32 >= self.queue_max {
            self.overflow_count += 1;
            return false;
        }
        self.total_events += 1;
        if event.needs_response { self.permission_pending += 1; }
        self.queue.push(event);
        true
    }

    #[inline(always)]
    pub fn read_events(&mut self, max: usize) -> Vec<FanotifyEvent> {
        let n = max.min(self.queue.len());
        self.queue.drain(..n).collect()
    }
}

/// Fanotify event
#[derive(Debug, Clone)]
pub struct FanotifyEvent {
    pub mask: u64,
    pub fd: i32,
    pub pid: u64,
    pub inode: u64,
    pub timestamp: u64,
    pub needs_response: bool,
    pub response: FanPermResponse,
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FanotifyBridgeStats {
    pub total_groups: u32,
    pub total_marks: u32,
    pub total_events: u64,
    pub overflow_events: u64,
    pub permission_pending: u32,
}

/// Main fanotify bridge
#[repr(align(64))]
pub struct BridgeFanotify {
    groups: BTreeMap<u64, FanotifyGroup>,
    next_id: u64,
    next_mark_id: u64,
}

impl BridgeFanotify {
    pub fn new() -> Self {
        Self { groups: BTreeMap::new(), next_id: 1, next_mark_id: 1 }
    }

    #[inline]
    pub fn create_group(&mut self, flags: FanInitFlags) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.groups.insert(id, FanotifyGroup::new(id, flags));
        id
    }

    #[inline]
    pub fn add_mark(&mut self, group_id: u64, mtype: FanMarkType, mask: u64, target: u64) -> Option<u64> {
        let mid = self.next_mark_id;
        self.next_mark_id += 1;
        let mark = FanMark::new(mid, mtype, FanEventMask::new(mask), target);
        self.groups.get_mut(&group_id)?.add_mark(mark);
        Some(mid)
    }

    #[inline]
    pub fn stats(&self) -> FanotifyBridgeStats {
        let marks: u32 = self.groups.values().map(|g| g.marks.len() as u32).sum();
        let events: u64 = self.groups.values().map(|g| g.total_events).sum();
        let overflow: u64 = self.groups.values().map(|g| g.overflow_count).sum();
        let perm: u32 = self.groups.values().map(|g| g.permission_pending).sum();
        FanotifyBridgeStats {
            total_groups: self.groups.len() as u32, total_marks: marks,
            total_events: events, overflow_events: overflow, permission_pending: perm,
        }
    }
}

// ============================================================================
// Merged from fanotify_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FanotifyV2Mask(pub u64);

impl FanotifyV2Mask {
    pub const ACCESS: u64 = 1 << 0;
    pub const MODIFY: u64 = 1 << 1;
    pub const CLOSE_WRITE: u64 = 1 << 2;
    pub const CLOSE_NOWRITE: u64 = 1 << 3;
    pub const OPEN: u64 = 1 << 4;
    pub const MOVED_FROM: u64 = 1 << 5;
    pub const MOVED_TO: u64 = 1 << 6;
    pub const CREATE: u64 = 1 << 7;
    pub const DELETE: u64 = 1 << 8;
    pub const DELETE_SELF: u64 = 1 << 9;
    pub const MOVE_SELF: u64 = 1 << 10;
    pub const OPEN_EXEC: u64 = 1 << 11;
    pub const OPEN_PERM: u64 = 1 << 12;
    pub const ACCESS_PERM: u64 = 1 << 13;
    pub const OPEN_EXEC_PERM: u64 = 1 << 14;

    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u64) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
}

/// Fanotify v2 event
#[derive(Debug, Clone)]
pub struct FanotifyV2Event {
    pub mask: FanotifyV2Mask,
    pub pid: u64,
    pub fd: i32,
    pub inode: u64,
    pub timestamp: u64,
}

/// Fanotify v2 mark
#[derive(Debug)]
pub struct FanotifyV2Mark {
    pub id: u64,
    pub mark_type: FanMarkType,
    pub mask: FanotifyV2Mask,
    pub target_inode: u64,
    pub ignored_mask: FanotifyV2Mask,
}

/// Fanotify v2 group
#[derive(Debug)]
pub struct FanotifyV2Group {
    pub id: u64,
    pub flags: u32,
    pub marks: Vec<u64>,
    pub events: Vec<FanotifyV2Event>,
    pub max_events: usize,
    pub overflow_count: u64,
}

impl FanotifyV2Group {
    pub fn new(id: u64, max: usize) -> Self {
        Self { id, flags: 0, marks: Vec::new(), events: Vec::new(), max_events: max, overflow_count: 0 }
    }

    #[inline(always)]
    pub fn push_event(&mut self, event: FanotifyV2Event) {
        if self.events.len() >= self.max_events { self.overflow_count += 1; return; }
        self.events.push(event);
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FanotifyV2BridgeStats {
    pub groups: u32,
    pub marks: u32,
    pub total_events: u64,
    pub overflow_events: u64,
}

/// Main fanotify v2 bridge
#[repr(align(64))]
pub struct BridgeFanotifyV2 {
    groups: BTreeMap<u64, FanotifyV2Group>,
    marks: BTreeMap<u64, FanotifyV2Mark>,
    next_group_id: u64,
    next_mark_id: u64,
}

impl BridgeFanotifyV2 {
    pub fn new() -> Self { Self { groups: BTreeMap::new(), marks: BTreeMap::new(), next_group_id: 1, next_mark_id: 1 } }

    #[inline]
    pub fn create_group(&mut self, max_events: usize) -> u64 {
        let id = self.next_group_id; self.next_group_id += 1;
        self.groups.insert(id, FanotifyV2Group::new(id, max_events));
        id
    }

    #[inline]
    pub fn add_mark(&mut self, group: u64, mark_type: FanMarkType, mask: FanotifyV2Mask, inode: u64) -> u64 {
        let id = self.next_mark_id; self.next_mark_id += 1;
        self.marks.insert(id, FanotifyV2Mark { id, mark_type, mask, target_inode: inode, ignored_mask: FanotifyV2Mask::new() });
        if let Some(g) = self.groups.get_mut(&group) { g.marks.push(id); }
        id
    }

    #[inline]
    pub fn stats(&self) -> FanotifyV2BridgeStats {
        let events: u64 = self.groups.values().map(|g| g.events.len() as u64).sum();
        let overflows: u64 = self.groups.values().map(|g| g.overflow_count).sum();
        FanotifyV2BridgeStats { groups: self.groups.len() as u32, marks: self.marks.len() as u32, total_events: events, overflow_events: overflows }
    }
}

// ============================================================================
// Merged from fanotify_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanotifyV3Mask {
    Access,
    Modify,
    CloseWrite,
    CloseNoWrite,
    Open,
    OpenExec,
    Attrib,
    Create,
    Delete,
    DeleteSelf,
    MovedFrom,
    MovedTo,
    Rename,
    OpenPerm,
    AccessPerm,
    OpenExecPerm,
}

/// Fanotify v3 response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanResponseV3 {
    Allow,
    Deny,
    AuditAllow,
    AuditDeny,
}

/// Fanotify v3 event
#[derive(Debug)]
pub struct FanEventV3 {
    pub fd: i64,
    pub mask: u64,
    pub pid: u64,
    pub timestamp: u64,
    pub info_hash: u64,
}

/// Fanotify v3 group
#[derive(Debug)]
pub struct FanGroupV3 {
    pub id: u64,
    pub flags: u32,
    pub event_mask: u64,
    pub events: Vec<FanEventV3>,
    pub marks: u32,
    pub overflow_count: u64,
    pub total_events: u64,
    pub permission_events: u64,
}

impl FanGroupV3 {
    pub fn new(id: u64, flags: u32) -> Self {
        Self { id, flags, event_mask: 0, events: Vec::new(), marks: 0, overflow_count: 0, total_events: 0, permission_events: 0 }
    }

    #[inline(always)]
    pub fn queue_event(&mut self, evt: FanEventV3) {
        self.total_events += 1;
        self.events.push(evt);
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FanotifyV3BridgeStats {
    pub total_groups: u32,
    pub total_marks: u32,
    pub total_events: u64,
    pub total_overflows: u64,
}

/// Main bridge fanotify v3
#[repr(align(64))]
pub struct BridgeFanotifyV3 {
    groups: BTreeMap<u64, FanGroupV3>,
    next_id: u64,
}

impl BridgeFanotifyV3 {
    pub fn new() -> Self { Self { groups: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn init(&mut self, flags: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.groups.insert(id, FanGroupV3::new(id, flags));
        id
    }

    #[inline(always)]
    pub fn mark(&mut self, group: u64, mask: u64) {
        if let Some(g) = self.groups.get_mut(&group) { g.event_mask |= mask; g.marks += 1; }
    }

    #[inline(always)]
    pub fn queue(&mut self, group: u64, evt: FanEventV3) {
        if let Some(g) = self.groups.get_mut(&group) { g.queue_event(evt); }
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) { self.groups.remove(&id); }

    #[inline]
    pub fn stats(&self) -> FanotifyV3BridgeStats {
        let marks: u32 = self.groups.values().map(|g| g.marks).sum();
        let evts: u64 = self.groups.values().map(|g| g.total_events).sum();
        let overflows: u64 = self.groups.values().map(|g| g.overflow_count).sum();
        FanotifyV3BridgeStats { total_groups: self.groups.len() as u32, total_marks: marks, total_events: evts, total_overflows: overflows }
    }
}
