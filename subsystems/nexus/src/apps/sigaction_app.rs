// SPDX-License-Identifier: GPL-2.0
//! App sigaction — rt_sigaction signal handler interface

extern crate alloc;

/// Sigaction app handler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigactionAppHandler { Default, Ignore, Custom, SigInfo }

/// Sigaction app record
#[derive(Debug, Clone)]
pub struct SigactionAppRecord {
    pub signal_nr: u32,
    pub handler: SigactionAppHandler,
    pub flags: u32,
    pub pid: u32,
}

impl SigactionAppRecord {
    pub fn new(signal_nr: u32, handler: SigactionAppHandler) -> Self {
        Self { signal_nr, handler, flags: 0, pid: 0 }
    }
}

/// Sigaction app stats
#[derive(Debug, Clone)]
pub struct SigactionAppStats { pub total_ops: u64, pub custom_set: u64, pub defaults_restored: u64 }

/// Main app sigaction
#[derive(Debug)]
pub struct AppSigaction { pub stats: SigactionAppStats }

impl AppSigaction {
    pub fn new() -> Self { Self { stats: SigactionAppStats { total_ops: 0, custom_set: 0, defaults_restored: 0 } } }
    pub fn record(&mut self, rec: &SigactionAppRecord) {
        self.stats.total_ops += 1;
        match rec.handler {
            SigactionAppHandler::Custom | SigactionAppHandler::SigInfo => self.stats.custom_set += 1,
            SigactionAppHandler::Default => self.stats.defaults_restored += 1,
            _ => {}
        }
    }
}
