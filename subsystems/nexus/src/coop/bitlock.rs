// SPDX-License-Identifier: GPL-2.0
//! Coop bitlock — single-bit spinlock for compact locking.

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

/// Bitlock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitlockState {
    Unlocked,
    Locked,
    Contended,
}

/// Bitlock — uses a single bit in a u64 word for locking
#[derive(Debug)]
pub struct Bitlock {
    word: AtomicU64,
    bit: u32,
    lock_count: u64,
    contention_count: u64,
    spin_total: u64,
}

impl Bitlock {
    pub fn new(bit: u32) -> Self {
        Self { word: AtomicU64::new(0), bit: bit & 63, lock_count: 0, contention_count: 0, spin_total: 0 }
    }

    #[inline]
    pub fn try_lock(&mut self) -> bool {
        let mask = 1u64 << self.bit;
        let old = self.word.fetch_or(mask, Ordering::Acquire);
        if old & mask == 0 {
            self.lock_count += 1;
            true
        } else {
            self.contention_count += 1;
            false
        }
    }

    #[inline(always)]
    pub fn unlock(&self) {
        let mask = 1u64 << self.bit;
        self.word.fetch_and(!mask, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        let mask = 1u64 << self.bit;
        self.word.load(Ordering::Relaxed) & mask != 0
    }
}

/// Bitlock array — manages many bitlocks in a compact array
pub struct BitlockArray {
    words: alloc::vec::Vec<AtomicU64>,
    total_bits: u32,
    lock_count: u64,
}

impl BitlockArray {
    pub fn new(bits: u32) -> Self {
        let words = (bits as usize + 63) / 64;
        let mut v = alloc::vec::Vec::with_capacity(words);
        for _ in 0..words { v.push(AtomicU64::new(0)); }
        Self { words: v, total_bits: bits, lock_count: 0 }
    }

    #[inline]
    pub fn try_lock(&mut self, idx: u32) -> bool {
        if idx >= self.total_bits { return false; }
        let word = (idx / 64) as usize;
        let bit = idx % 64;
        let mask = 1u64 << bit;
        let old = self.words[word].fetch_or(mask, Ordering::Acquire);
        if old & mask == 0 { self.lock_count += 1; true } else { false }
    }

    #[inline]
    pub fn unlock(&self, idx: u32) {
        if idx >= self.total_bits { return; }
        let word = (idx / 64) as usize;
        let bit = idx % 64;
        let mask = 1u64 << bit;
        self.words[word].fetch_and(!mask, Ordering::Release);
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BitlockStats {
    pub total_locks: u64,
    pub total_contentions: u64,
}

/// Main coop bitlock
pub struct CoopBitlock {
    locks: alloc::vec::Vec<Bitlock>,
}

impl CoopBitlock {
    pub fn new() -> Self { Self { locks: alloc::vec::Vec::new() } }

    #[inline]
    pub fn create(&mut self, bit: u32) -> usize {
        let idx = self.locks.len();
        self.locks.push(Bitlock::new(bit));
        idx
    }

    #[inline]
    pub fn stats(&self) -> BitlockStats {
        let locks: u64 = self.locks.iter().map(|l| l.lock_count).sum();
        let cont: u64 = self.locks.iter().map(|l| l.contention_count).sum();
        BitlockStats { total_locks: locks, total_contentions: cont }
    }
}
