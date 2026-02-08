// SPDX-License-Identifier: GPL-2.0
//! Bridge sigaction — signal handler registration bridge

extern crate alloc;

/// Sigaction operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigactionOp {
    Set,
    Get,
    Reset,
}

/// Sigaction handler type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigactionHandler {
    Default,
    Ignore,
    Custom,
    SigInfo,
}

/// Sigaction flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigactionFlag {
    SaRestart,
    SaNocldstop,
    SaNocldwait,
    SaSiginfo,
    SaOnstack,
    SaResethand,
    SaNodefer,
}

/// Sigaction record
#[derive(Debug, Clone)]
pub struct SigactionRecord {
    pub op: SigactionOp,
    pub signal_nr: u32,
    pub handler: SigactionHandler,
    pub flags: u32,
    pub pid: u32,
}

impl SigactionRecord {
    pub fn new(op: SigactionOp, signal_nr: u32) -> Self {
        Self { op, signal_nr, handler: SigactionHandler::Default, flags: 0, pid: 0 }
    }
}

/// Sigaction bridge stats
#[derive(Debug, Clone)]
pub struct SigactionBridgeStats {
    pub total_ops: u64,
    pub handlers_set: u64,
    pub handlers_reset: u64,
    pub custom_handlers: u64,
}

/// Main bridge sigaction
#[derive(Debug)]
pub struct BridgeSigaction {
    pub stats: SigactionBridgeStats,
}

impl BridgeSigaction {
    pub fn new() -> Self {
        Self { stats: SigactionBridgeStats { total_ops: 0, handlers_set: 0, handlers_reset: 0, custom_handlers: 0 } }
    }

    pub fn record(&mut self, rec: &SigactionRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            SigactionOp::Set => {
                self.stats.handlers_set += 1;
                if rec.handler == SigactionHandler::Custom || rec.handler == SigactionHandler::SigInfo {
                    self.stats.custom_handlers += 1;
                }
            }
            SigactionOp::Reset => self.stats.handlers_reset += 1,
            _ => {}
        }
    }
}
