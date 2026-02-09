// SPDX-License-Identifier: GPL-2.0
//! Holistic sigaction — holistic signal handler pattern analysis

extern crate alloc;

/// Sigaction pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigactionPattern { DefaultHandler, CustomHandler, IgnoredSignal, ChainedHandler }

/// Sigaction holistic record
#[derive(Debug, Clone)]
pub struct SigactionHolisticRecord {
    pub pattern: SigactionPattern,
    pub signal_nr: u32,
    pub pid: u32,
    pub handler_changes: u32,
}

impl SigactionHolisticRecord {
    pub fn new(pattern: SigactionPattern, signal_nr: u32) -> Self { Self { pattern, signal_nr, pid: 0, handler_changes: 0 } }
}

/// Sigaction holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SigactionHolisticStats { pub total_samples: u64, pub custom_handlers: u64, pub ignored: u64, pub chained: u64 }

/// Main holistic sigaction
#[derive(Debug)]
pub struct HolisticSigaction { pub stats: SigactionHolisticStats }

impl HolisticSigaction {
    pub fn new() -> Self { Self { stats: SigactionHolisticStats { total_samples: 0, custom_handlers: 0, ignored: 0, chained: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SigactionHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.pattern {
            SigactionPattern::CustomHandler => self.stats.custom_handlers += 1,
            SigactionPattern::IgnoredSignal => self.stats.ignored += 1,
            SigactionPattern::ChainedHandler => self.stats.chained += 1,
            _ => {}
        }
    }
}
