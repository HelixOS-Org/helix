// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Brk (heap management bridge)

extern crate alloc;

/// Brk operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeBrkOp {
    Expand,
    Shrink,
    Query,
}

/// Brk state
#[derive(Debug, Clone)]
pub struct BridgeBrkState {
    pub start: u64,
    pub current: u64,
    pub max_reached: u64,
    pub page_aligned: bool,
}

/// Brk stats
#[derive(Debug, Clone)]
pub struct BridgeBrkStats {
    pub total_ops: u64,
    pub expansions: u64,
    pub shrinks: u64,
    pub queries: u64,
    pub peak_heap_bytes: u64,
    pub current_heap_bytes: u64,
}

/// Manager for brk bridge operations
pub struct BridgeBrkManager {
    state: BridgeBrkState,
    stats: BridgeBrkStats,
}

impl BridgeBrkManager {
    pub fn new(heap_start: u64) -> Self {
        Self {
            state: BridgeBrkState {
                start: heap_start,
                current: heap_start,
                max_reached: heap_start,
                page_aligned: true,
            },
            stats: BridgeBrkStats {
                total_ops: 0,
                expansions: 0,
                shrinks: 0,
                queries: 0,
                peak_heap_bytes: 0,
                current_heap_bytes: 0,
            },
        }
    }

    pub fn brk(&mut self, new_brk: u64) -> u64 {
        self.stats.total_ops += 1;
        if new_brk == 0 {
            self.stats.queries += 1;
            return self.state.current;
        }
        let aligned = (new_brk + 4095) & !4095;
        if aligned > self.state.current {
            self.stats.expansions += 1;
        } else if aligned < self.state.current {
            self.stats.shrinks += 1;
        }
        self.state.current = aligned;
        if aligned > self.state.max_reached {
            self.state.max_reached = aligned;
        }
        self.stats.current_heap_bytes = self.state.current - self.state.start;
        if self.stats.current_heap_bytes > self.stats.peak_heap_bytes {
            self.stats.peak_heap_bytes = self.stats.current_heap_bytes;
        }
        self.state.current
    }

    pub fn heap_size(&self) -> u64 { self.state.current - self.state.start }
    pub fn stats(&self) -> &BridgeBrkStats { &self.stats }
}
