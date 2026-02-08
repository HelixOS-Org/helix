// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Mremap (memory remap bridge)

extern crate alloc;
use alloc::vec::Vec;

/// Mremap flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMremapFlag { MayMove, Fixed, DontUnmap }

/// Mremap record
#[derive(Debug, Clone)]
pub struct BridgeMremapRecord { pub old_addr: u64, pub old_size: u64, pub new_addr: u64, pub new_size: u64, pub moved: bool }

/// Mremap stats
#[derive(Debug, Clone)]
pub struct BridgeMremapStats { pub total_ops: u64, pub grows: u64, pub shrinks: u64, pub moves: u64, pub in_place: u64 }

/// Manager for mremap bridge
pub struct BridgeMremapManager {
    history: Vec<BridgeMremapRecord>,
    next_addr: u64,
    stats: BridgeMremapStats,
}

impl BridgeMremapManager {
    pub fn new() -> Self {
        Self { history: Vec::new(), next_addr: 0x7f8000000000, stats: BridgeMremapStats { total_ops: 0, grows: 0, shrinks: 0, moves: 0, in_place: 0 } }
    }

    pub fn mremap(&mut self, old_addr: u64, old_size: u64, new_size: u64, may_move: bool) -> u64 {
        self.stats.total_ops += 1;
        if new_size > old_size { self.stats.grows += 1; } else { self.stats.shrinks += 1; }
        let (new_addr, moved) = if may_move && new_size > old_size {
            let a = self.next_addr; self.next_addr += (new_size + 4095) & !4095;
            self.stats.moves += 1; (a, true)
        } else {
            self.stats.in_place += 1; (old_addr, false)
        };
        self.history.push(BridgeMremapRecord { old_addr, old_size, new_addr, new_size, moved });
        new_addr
    }

    pub fn stats(&self) -> &BridgeMremapStats { &self.stats }
}
