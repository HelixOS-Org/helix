// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic F2FS GC — Flash-friendly filesystem garbage collection
//!
//! Models F2FS garbage collection with segment cleaning, valid block
//! counting, foreground/background GC modes, and victim selection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// F2FS segment type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F2fsSegmentType {
    HotData,
    WarmData,
    ColdData,
    HotNode,
    WarmNode,
    ColdNode,
}

/// Segment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F2fsSegmentState {
    Free,
    InUse,
    Full,
    Dirty,
    Prefree,
}

/// GC mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F2fsGcMode {
    Background,
    Foreground,
    Urgent,
    Idle,
}

/// Victim selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F2fsVictimPolicy {
    Greedy,
    CostBenefit,
    AgeThreshold,
}

/// A F2FS segment.
#[derive(Debug, Clone)]
pub struct F2fsSegment {
    pub segment_id: u64,
    pub seg_type: F2fsSegmentType,
    pub state: F2fsSegmentState,
    pub valid_blocks: u32,
    pub total_blocks: u32,
    pub age: u64,
    pub last_gc_time: u64,
}

impl F2fsSegment {
    pub fn new(segment_id: u64, seg_type: F2fsSegmentType, total_blocks: u32) -> Self {
        Self {
            segment_id,
            seg_type,
            state: F2fsSegmentState::Free,
            valid_blocks: 0,
            total_blocks,
            age: 0,
            last_gc_time: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.total_blocks == 0 {
            return 0.0;
        }
        self.valid_blocks as f64 / self.total_blocks as f64
    }

    pub fn gc_benefit(&self) -> f64 {
        // Cost-benefit: (1 - u) * age / (2 * u)
        let u = self.utilization();
        if u < 0.001 {
            return f64::MAX;
        }
        (1.0 - u) * self.age as f64 / (2.0 * u)
    }
}

/// GC round record.
#[derive(Debug, Clone)]
pub struct F2fsGcRound {
    pub round_id: u64,
    pub mode: F2fsGcMode,
    pub policy: F2fsVictimPolicy,
    pub segments_cleaned: u32,
    pub blocks_moved: u64,
    pub valid_blocks_copied: u64,
    pub start_time: u64,
    pub duration_ns: u64,
}

impl F2fsGcRound {
    pub fn new(round_id: u64, mode: F2fsGcMode) -> Self {
        Self {
            round_id,
            mode,
            policy: F2fsVictimPolicy::Greedy,
            segments_cleaned: 0,
            blocks_moved: 0,
            valid_blocks_copied: 0,
            start_time: 0,
            duration_ns: 0,
        }
    }
}

/// Statistics for F2FS GC.
#[derive(Debug, Clone)]
pub struct F2fsGcStats {
    pub total_gc_rounds: u64,
    pub foreground_gcs: u64,
    pub background_gcs: u64,
    pub segments_cleaned: u64,
    pub blocks_moved: u64,
    pub free_segments: u64,
    pub dirty_segments: u64,
}

/// Main holistic F2FS GC manager.
pub struct HolisticF2fsGc {
    pub segments: BTreeMap<u64, F2fsSegment>,
    pub gc_history: Vec<F2fsGcRound>,
    pub next_segment_id: u64,
    pub next_round_id: u64,
    pub gc_mode: F2fsGcMode,
    pub victim_policy: F2fsVictimPolicy,
    pub stats: F2fsGcStats,
}

impl HolisticF2fsGc {
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            gc_history: Vec::new(),
            next_segment_id: 1,
            next_round_id: 1,
            gc_mode: F2fsGcMode::Background,
            victim_policy: F2fsVictimPolicy::Greedy,
            stats: F2fsGcStats {
                total_gc_rounds: 0,
                foreground_gcs: 0,
                background_gcs: 0,
                segments_cleaned: 0,
                blocks_moved: 0,
                free_segments: 0,
                dirty_segments: 0,
            },
        }
    }

    pub fn add_segment(&mut self, seg_type: F2fsSegmentType, blocks: u32) -> u64 {
        let id = self.next_segment_id;
        self.next_segment_id += 1;
        let seg = F2fsSegment::new(id, seg_type, blocks);
        self.segments.insert(id, seg);
        self.stats.free_segments += 1;
        id
    }

    pub fn select_victim(&self) -> Option<u64> {
        let mut best_id = None;
        let mut best_score = f64::MIN;
        for (id, seg) in &self.segments {
            if seg.state != F2fsSegmentState::Dirty {
                continue;
            }
            let score = match self.victim_policy {
                F2fsVictimPolicy::Greedy => 1.0 - seg.utilization(),
                F2fsVictimPolicy::CostBenefit => seg.gc_benefit(),
                F2fsVictimPolicy::AgeThreshold => seg.age as f64,
            };
            if score > best_score {
                best_score = score;
                best_id = Some(*id);
            }
        }
        best_id
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}
