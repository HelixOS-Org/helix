// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic XFS log — XFS write-ahead log tracking
//!
//! Models the XFS log with log item lifecycle, intent/done log items,
//! log space reservation, and log recovery state.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// XFS log item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfsLogItemType {
    InodeFork,
    ExtentFree,
    BufData,
    BufLog,
    InodeCore,
    Dquot,
    QuotaOff,
    AttrFork,
    ExtentFreeIntent,
    ExtentFreeDone,
    ReverseMapIntent,
    ReverseMapDone,
    RefcountIntent,
    RefcountDone,
    BmapIntent,
    BmapDone,
}

/// Log item state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfsLogItemState {
    Pinned,
    InLog,
    Committed,
    Checkpointed,
    Freed,
}

/// A log item.
#[derive(Debug, Clone)]
pub struct XfsLogItem {
    pub item_id: u64,
    pub item_type: XfsLogItemType,
    pub state: XfsLogItemState,
    pub lsn: u64,
    pub size_bytes: u32,
    pub transaction_id: u64,
    pub intent_done: Option<u64>, // paired intent/done
}

impl XfsLogItem {
    pub fn new(item_id: u64, item_type: XfsLogItemType, tx_id: u64) -> Self {
        Self {
            item_id,
            item_type,
            state: XfsLogItemState::Pinned,
            lsn: 0,
            size_bytes: 0,
            transaction_id: tx_id,
            intent_done: None,
        }
    }
}

/// Log space reservation.
#[derive(Debug, Clone)]
pub struct XfsLogReservation {
    pub log_size: u64,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub grant_head: u64,
    pub write_head: u64,
    pub tail_lsn: u64,
}

impl XfsLogReservation {
    pub fn new(log_size: u64) -> Self {
        Self {
            log_size,
            used_bytes: 0,
            reserved_bytes: 0,
            grant_head: 0,
            write_head: 0,
            tail_lsn: 0,
        }
    }

    pub fn free_space(&self) -> u64 {
        self.log_size
            .saturating_sub(self.used_bytes)
            .saturating_sub(self.reserved_bytes)
    }

    pub fn utilization(&self) -> f64 {
        if self.log_size == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.log_size as f64
    }
}

/// Statistics for XFS log.
#[derive(Debug, Clone)]
pub struct XfsLogStats {
    pub total_items: u64,
    pub total_transactions: u64,
    pub total_checkpoints: u64,
    pub intent_done_pairs: u64,
    pub log_forces: u64,
    pub tail_pushes: u64,
    pub log_wraps: u64,
}

/// Main holistic XFS log manager.
pub struct HolisticXfsLog {
    pub items: BTreeMap<u64, XfsLogItem>,
    pub reservation: XfsLogReservation,
    pub next_item_id: u64,
    pub next_lsn: u64,
    pub stats: XfsLogStats,
}

impl HolisticXfsLog {
    pub fn new(log_size: u64) -> Self {
        Self {
            items: BTreeMap::new(),
            reservation: XfsLogReservation::new(log_size),
            next_item_id: 1,
            next_lsn: 1,
            stats: XfsLogStats {
                total_items: 0,
                total_transactions: 0,
                total_checkpoints: 0,
                intent_done_pairs: 0,
                log_forces: 0,
                tail_pushes: 0,
                log_wraps: 0,
            },
        }
    }

    pub fn add_item(&mut self, item_type: XfsLogItemType, tx_id: u64) -> u64 {
        let id = self.next_item_id;
        self.next_item_id += 1;
        let mut item = XfsLogItem::new(id, item_type, tx_id);
        item.lsn = self.next_lsn;
        self.next_lsn += 1;
        self.items.insert(id, item);
        self.stats.total_items += 1;
        id
    }

    pub fn checkpoint(&mut self, up_to_lsn: u64) {
        let to_remove: Vec<u64> = self
            .items
            .iter()
            .filter(|(_, item)| item.lsn <= up_to_lsn)
            .map(|(&id, _)| id)
            .collect();
        for id in to_remove {
            if let Some(item) = self.items.get_mut(&id) {
                item.state = XfsLogItemState::Checkpointed;
            }
        }
        self.stats.total_checkpoints += 1;
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}
