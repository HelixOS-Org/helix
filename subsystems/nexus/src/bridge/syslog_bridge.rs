// SPDX-License-Identifier: GPL-2.0
//! Bridge syslog_bridge — kernel syslog facility bridge.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Syslog facility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogFacility {
    Kern,
    User,
    Mail,
    Daemon,
    Auth,
    Syslog,
    Lpr,
    News,
    Uucp,
    Cron,
    AuthPriv,
    Ftp,
    Local0,
    Local1,
    Local2,
    Local3,
    Local4,
    Local5,
    Local6,
    Local7,
}

/// Syslog priority/severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogSeverity {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

/// Syslog message
#[derive(Debug)]
pub struct SyslogMessage {
    pub seq: u64,
    pub facility: SyslogFacility,
    pub severity: SyslogSeverity,
    pub msg_hash: u64,
    pub timestamp: u64,
    pub pid: u64,
    pub dropped: bool,
}

/// Syslog ring buffer
#[derive(Debug)]
#[repr(align(64))]
pub struct SyslogRingBuffer {
    pub messages: VecDeque<SyslogMessage>,
    pub capacity: usize,
    pub next_seq: u64,
    pub dropped_count: u64,
    pub read_position: u64,
}

impl SyslogRingBuffer {
    pub fn new(capacity: usize) -> Self { Self { messages: VecDeque::new(), capacity, next_seq: 1, dropped_count: 0, read_position: 0 } }

    #[inline]
    pub fn write(&mut self, facility: SyslogFacility, severity: SyslogSeverity, msg_hash: u64, pid: u64, now: u64) -> u64 {
        let seq = self.next_seq; self.next_seq += 1;
        if self.messages.len() >= self.capacity { self.messages.pop_front(); self.dropped_count += 1; }
        self.messages.push_back(SyslogMessage { seq, facility, severity, msg_hash, timestamp: now, pid, dropped: false });
        seq
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SyslogBridgeStats {
    pub total_messages: u64,
    pub buffer_used: u32,
    pub buffer_capacity: u32,
    pub dropped: u64,
    pub errors: u64,
    pub warnings: u64,
}

/// Main syslog bridge
pub struct BridgeSyslog {
    buffer: SyslogRingBuffer,
}

impl BridgeSyslog {
    pub fn new(capacity: usize) -> Self { Self { buffer: SyslogRingBuffer::new(capacity) } }

    #[inline(always)]
    pub fn log(&mut self, facility: SyslogFacility, severity: SyslogSeverity, msg_hash: u64, pid: u64, now: u64) -> u64 {
        self.buffer.write(facility, severity, msg_hash, pid, now)
    }

    #[inline]
    pub fn stats(&self) -> SyslogBridgeStats {
        let errors = self.buffer.messages.iter().filter(|m| matches!(m.severity, SyslogSeverity::Error | SyslogSeverity::Critical | SyslogSeverity::Alert | SyslogSeverity::Emergency)).count() as u64;
        let warnings = self.buffer.messages.iter().filter(|m| m.severity == SyslogSeverity::Warning).count() as u64;
        SyslogBridgeStats { total_messages: self.buffer.next_seq - 1, buffer_used: self.buffer.messages.len() as u32, buffer_capacity: self.buffer.capacity as u32, dropped: self.buffer.dropped_count, errors, warnings }
    }
}

// ============================================================================
// Merged from syslog_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyslogV2Level {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

/// Syslog facility codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogV2Facility {
    Kern,
    User,
    Mail,
    Daemon,
    Auth,
    Syslog,
    Lpr,
    News,
    Cron,
    AuthPriv,
    Local0,
    Local1,
    Local2,
    Local3,
    Local4,
    Local5,
    Local6,
    Local7,
}

/// A structured log entry
#[derive(Debug, Clone)]
pub struct SyslogV2Entry {
    pub seq: u64,
    pub timestamp_ns: u64,
    pub level: SyslogV2Level,
    pub facility: SyslogV2Facility,
    pub subsystem: String,
    pub message: String,
    pub pid: Option<u64>,
    pub cpu: Option<u32>,
}

/// Ring buffer for kernel log
#[derive(Debug, Clone)]
pub struct SyslogV2Ring {
    pub entries: Vec<SyslogV2Entry>,
    pub capacity: usize,
    pub write_pos: usize,
    pub total_written: u64,
    pub dropped: u64,
}

impl SyslogV2Ring {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity,
            write_pos: 0,
            total_written: 0,
            dropped: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, entry: SyslogV2Entry) {
        if self.entries.len() < self.capacity {
            self.entries.push(entry);
        } else {
            let idx = self.write_pos % self.capacity;
            self.entries[idx] = entry;
            self.dropped += 1;
        }
        self.write_pos += 1;
        self.total_written += 1;
    }

    #[inline]
    pub fn read_recent(&self, count: usize) -> Vec<&SyslogV2Entry> {
        let len = self.entries.len();
        let start = if len > count { len - count } else { 0 };
        self.entries[start..].iter().collect()
    }

    #[inline(always)]
    pub fn filter_level(&self, max_level: SyslogV2Level) -> Vec<&SyslogV2Entry> {
        self.entries.iter().filter(|e| e.level <= max_level).collect()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.write_pos = 0;
    }
}

/// Statistics for syslog V2 bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SyslogV2BridgeStats {
    pub total_messages: u64,
    pub emergency_count: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub debug_count: u64,
    pub dropped_messages: u64,
    pub readers_registered: u64,
}

/// Main syslog V2 bridge manager
#[derive(Debug)]
pub struct BridgeSyslogV2 {
    ring: SyslogV2Ring,
    next_seq: u64,
    console_level: SyslogV2Level,
    stats: SyslogV2BridgeStats,
}

impl BridgeSyslogV2 {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            ring: SyslogV2Ring::new(ring_capacity),
            next_seq: 1,
            console_level: SyslogV2Level::Warning,
            stats: SyslogV2BridgeStats {
                total_messages: 0,
                emergency_count: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                debug_count: 0,
                dropped_messages: 0,
                readers_registered: 0,
            },
        }
    }

    pub fn log(&mut self, level: SyslogV2Level, facility: SyslogV2Facility, subsystem: String, message: String, tick: u64) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let entry = SyslogV2Entry {
            seq,
            timestamp_ns: tick,
            level,
            facility,
            subsystem,
            message,
            pid: None,
            cpu: None,
        };
        self.ring.write(entry);
        self.stats.total_messages += 1;
        match level {
            SyslogV2Level::Emergency => self.stats.emergency_count += 1,
            SyslogV2Level::Error | SyslogV2Level::Alert | SyslogV2Level::Critical => self.stats.error_count += 1,
            SyslogV2Level::Warning => self.stats.warning_count += 1,
            SyslogV2Level::Info | SyslogV2Level::Notice => self.stats.info_count += 1,
            SyslogV2Level::Debug => self.stats.debug_count += 1,
        }
    }

    #[inline(always)]
    pub fn read_recent(&self, count: usize) -> Vec<&SyslogV2Entry> {
        self.ring.read_recent(count)
    }

    #[inline(always)]
    pub fn set_console_level(&mut self, level: SyslogV2Level) {
        self.console_level = level;
    }

    #[inline(always)]
    pub fn stats(&self) -> &SyslogV2BridgeStats {
        &self.stats
    }
}
