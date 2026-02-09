// SPDX-License-Identifier: GPL-2.0
//! Coop msgqueue — cooperative SysV message queue coordination

extern crate alloc;

/// Msgqueue coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgqueueCoopEvent { QueueShare, PriorityReorder, LoadBalance, BatchDequeue }

/// Msgqueue coop record
#[derive(Debug, Clone)]
pub struct MsgqueueCoopRecord {
    pub event: MsgqueueCoopEvent,
    pub msqid: i32,
    pub messages: u32,
    pub participants: u32,
}

impl MsgqueueCoopRecord {
    pub fn new(event: MsgqueueCoopEvent) -> Self { Self { event, msqid: -1, messages: 0, participants: 0 } }
}

/// Msgqueue coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MsgqueueCoopStats { pub total_events: u64, pub shares: u64, pub reorders: u64, pub batches: u64 }

/// Main coop msgqueue
#[derive(Debug)]
pub struct CoopMsgqueue { pub stats: MsgqueueCoopStats }

impl CoopMsgqueue {
    pub fn new() -> Self { Self { stats: MsgqueueCoopStats { total_events: 0, shares: 0, reorders: 0, batches: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &MsgqueueCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            MsgqueueCoopEvent::QueueShare => self.stats.shares += 1,
            MsgqueueCoopEvent::PriorityReorder => self.stats.reorders += 1,
            MsgqueueCoopEvent::BatchDequeue | MsgqueueCoopEvent::LoadBalance => self.stats.batches += 1,
        }
    }
}
