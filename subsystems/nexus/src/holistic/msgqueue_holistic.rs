// SPDX-License-Identifier: GPL-2.0
//! Holistic msgqueue — holistic SysV message queue depth analysis

extern crate alloc;

/// Msgqueue depth state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgqueueDepthState { Empty, Normal, NearFull, Overflowing }

/// Msgqueue holistic record
#[derive(Debug, Clone)]
pub struct MsgqueueHolisticRecord {
    pub state: MsgqueueDepthState,
    pub msqid: i32,
    pub depth: u32,
    pub max_depth: u32,
    pub msg_rate: u64,
}

impl MsgqueueHolisticRecord {
    pub fn new(state: MsgqueueDepthState) -> Self { Self { state, msqid: -1, depth: 0, max_depth: 0, msg_rate: 0 } }
}

/// Msgqueue holistic stats
#[derive(Debug, Clone)]
pub struct MsgqueueHolisticStats { pub total_samples: u64, pub overflows: u64, pub near_full: u64, pub peak_depth: u32 }

/// Main holistic msgqueue
#[derive(Debug)]
pub struct HolisticMsgqueue { pub stats: MsgqueueHolisticStats }

impl HolisticMsgqueue {
    pub fn new() -> Self { Self { stats: MsgqueueHolisticStats { total_samples: 0, overflows: 0, near_full: 0, peak_depth: 0 } } }
    pub fn record(&mut self, rec: &MsgqueueHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.state {
            MsgqueueDepthState::Overflowing => self.stats.overflows += 1,
            MsgqueueDepthState::NearFull => self.stats.near_full += 1,
            _ => {}
        }
        if rec.depth > self.stats.peak_depth { self.stats.peak_depth = rec.depth; }
    }
}
