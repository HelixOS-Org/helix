// SPDX-License-Identifier: GPL-2.0
//! Coop backoff — exponential backoff with jitter.

extern crate alloc;

/// Backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    Constant,
    Linear,
    Exponential,
    ExponentialWithJitter,
    Fibonacci,
    Decorrelated,
}

/// Backoff state
#[derive(Debug)]
#[repr(align(64))]
pub struct BackoffState {
    pub strategy: BackoffStrategy,
    pub base_ns: u64,
    pub max_ns: u64,
    pub current_ns: u64,
    pub attempt: u32,
    pub max_attempts: u32,
    pub prev_ns: u64,
    pub seed: u64,
}

impl BackoffState {
    pub fn new(strategy: BackoffStrategy, base: u64, max: u64, max_att: u32) -> Self {
        Self { strategy, base_ns: base, max_ns: max, current_ns: base, attempt: 0, max_attempts: max_att, prev_ns: base, seed: 0x12345678 }
    }

    fn xorshift(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    pub fn next_delay(&mut self) -> Option<u64> {
        if self.attempt >= self.max_attempts { return None; }
        self.attempt += 1;
        let delay = match self.strategy {
            BackoffStrategy::Constant => self.base_ns,
            BackoffStrategy::Linear => self.base_ns * self.attempt as u64,
            BackoffStrategy::Exponential => self.base_ns.saturating_mul(1u64 << self.attempt.min(32)),
            BackoffStrategy::ExponentialWithJitter => {
                let exp = self.base_ns.saturating_mul(1u64 << self.attempt.min(32));
                let jitter = self.xorshift() % (exp / 2 + 1);
                exp / 2 + jitter
            }
            BackoffStrategy::Fibonacci => {
                let next = self.current_ns + self.prev_ns;
                self.prev_ns = self.current_ns;
                self.current_ns = next;
                next
            }
            BackoffStrategy::Decorrelated => {
                let r = self.xorshift() % (self.current_ns * 3 + 1);
                let next = self.base_ns.max(r);
                self.current_ns = next;
                next
            }
        };
        Some(delay.min(self.max_ns))
    }

    #[inline]
    pub fn reset(&mut self) {
        self.attempt = 0;
        self.current_ns = self.base_ns;
        self.prev_ns = self.base_ns;
    }

    #[inline(always)]
    pub fn exhausted(&self) -> bool { self.attempt >= self.max_attempts }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BackoffStats {
    pub total_backoffs: u64,
    pub total_resets: u64,
    pub total_exhaustions: u64,
    pub avg_attempts: f64,
}

/// Main coop backoff manager
pub struct CoopBackoff {
    active: u32,
    total_backoffs: u64,
    total_resets: u64,
    total_exhaustions: u64,
    total_attempts: u64,
    total_sessions: u64,
}

impl CoopBackoff {
    pub fn new() -> Self { Self { active: 0, total_backoffs: 0, total_resets: 0, total_exhaustions: 0, total_attempts: 0, total_sessions: 0 } }

    #[inline]
    pub fn create_state(&mut self, strategy: BackoffStrategy, base: u64, max: u64, max_att: u32) -> BackoffState {
        self.active += 1;
        self.total_sessions += 1;
        BackoffState::new(strategy, base, max, max_att)
    }

    #[inline(always)]
    pub fn record_backoff(&mut self) { self.total_backoffs += 1; self.total_attempts += 1; }

    #[inline(always)]
    pub fn record_reset(&mut self) { self.total_resets += 1; }

    #[inline(always)]
    pub fn record_exhaustion(&mut self) { self.total_exhaustions += 1; }

    #[inline(always)]
    pub fn release(&mut self) { if self.active > 0 { self.active -= 1; } }

    #[inline(always)]
    pub fn stats(&self) -> BackoffStats {
        let avg = if self.total_sessions == 0 { 0.0 } else { self.total_attempts as f64 / self.total_sessions as f64 };
        BackoffStats { total_backoffs: self.total_backoffs, total_resets: self.total_resets, total_exhaustions: self.total_exhaustions, avg_attempts: avg }
    }
}
