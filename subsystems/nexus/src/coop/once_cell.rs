// SPDX-License-Identifier: GPL-2.0
//! Coop once_cell — lazy one-time initialization.

extern crate alloc;

use alloc::collections::BTreeMap;

/// OnceCell state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnceCellState {
    Uninitialized,
    Initializing,
    Initialized,
    Poisoned,
}

/// Once cell
#[derive(Debug)]
pub struct OnceCell {
    pub id: u64,
    pub state: OnceCellState,
    pub value_hash: u64,
    pub initialized_at: u64,
    pub initializer_tid: u64,
    pub access_count: u64,
    pub waiters: u32,
}

impl OnceCell {
    pub fn new(id: u64) -> Self {
        Self { id, state: OnceCellState::Uninitialized, value_hash: 0, initialized_at: 0, initializer_tid: 0, access_count: 0, waiters: 0 }
    }

    #[inline]
    pub fn try_init(&mut self, tid: u64) -> bool {
        if self.state != OnceCellState::Uninitialized { return false; }
        self.state = OnceCellState::Initializing;
        self.initializer_tid = tid;
        true
    }

    #[inline]
    pub fn complete_init(&mut self, value_hash: u64, now: u64) {
        self.state = OnceCellState::Initialized;
        self.value_hash = value_hash;
        self.initialized_at = now;
    }

    #[inline(always)]
    pub fn poison(&mut self) { self.state = OnceCellState::Poisoned; }

    #[inline(always)]
    pub fn get(&mut self) -> Option<u64> {
        if self.state == OnceCellState::Initialized { self.access_count += 1; Some(self.value_hash) }
        else { None }
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool { self.state == OnceCellState::Initialized }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct OnceCellStats {
    pub total_cells: u32,
    pub initialized: u32,
    pub uninitialized: u32,
    pub poisoned: u32,
    pub total_accesses: u64,
}

/// Main once cell manager
pub struct CoopOnceCell {
    cells: BTreeMap<u64, OnceCell>,
    next_id: u64,
}

impl CoopOnceCell {
    pub fn new() -> Self { Self { cells: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.cells.insert(id, OnceCell::new(id));
        id
    }

    #[inline]
    pub fn get_or_init(&mut self, id: u64, tid: u64, value_hash: u64, now: u64) -> Option<u64> {
        let cell = self.cells.get_mut(&id)?;
        if cell.is_initialized() { return cell.get(); }
        if cell.try_init(tid) { cell.complete_init(value_hash, now); }
        cell.get()
    }

    #[inline]
    pub fn stats(&self) -> OnceCellStats {
        let initialized = self.cells.values().filter(|c| c.is_initialized()).count() as u32;
        let uninitialized = self.cells.values().filter(|c| c.state == OnceCellState::Uninitialized).count() as u32;
        let poisoned = self.cells.values().filter(|c| c.state == OnceCellState::Poisoned).count() as u32;
        let accesses: u64 = self.cells.values().map(|c| c.access_count).sum();
        OnceCellStats { total_cells: self.cells.len() as u32, initialized, uninitialized, poisoned, total_accesses: accesses }
    }
}
