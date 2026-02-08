// SPDX-License-Identifier: GPL-2.0
//! Apps ioctl_app — device ioctl management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Ioctl direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlDir {
    None,
    Read,
    Write,
    ReadWrite,
}

/// Ioctl command descriptor
#[derive(Debug)]
pub struct IoctlCmd {
    pub cmd: u32,
    pub direction: IoctlDir,
    pub arg_size: u32,
    pub magic: u8,
    pub number: u8,
}

impl IoctlCmd {
    pub fn from_raw(cmd: u32) -> Self {
        let dir = match (cmd >> 30) & 3 { 0 => IoctlDir::None, 1 => IoctlDir::Write, 2 => IoctlDir::Read, _ => IoctlDir::ReadWrite };
        let size = ((cmd >> 16) & 0x3FFF) as u32;
        let magic = ((cmd >> 8) & 0xFF) as u8;
        let number = (cmd & 0xFF) as u8;
        Self { cmd, direction: dir, arg_size: size, magic, number }
    }
}

/// Ioctl event
#[derive(Debug)]
pub struct IoctlEvent {
    pub fd: u64,
    pub pid: u64,
    pub cmd: IoctlCmd,
    pub timestamp: u64,
    pub success: bool,
    pub duration_ns: u64,
}

/// Ioctl frequency tracker
#[derive(Debug)]
pub struct IoctlTracker {
    pub cmd_counts: BTreeMap<u32, u64>,
    pub total_calls: u64,
    pub total_errors: u64,
    pub total_time_ns: u64,
}

impl IoctlTracker {
    pub fn new() -> Self { Self { cmd_counts: BTreeMap::new(), total_calls: 0, total_errors: 0, total_time_ns: 0 } }

    pub fn record(&mut self, cmd: u32, success: bool, dur: u64) {
        *self.cmd_counts.entry(cmd).or_insert(0) += 1;
        self.total_calls += 1;
        if !success { self.total_errors += 1; }
        self.total_time_ns += dur;
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IoctlAppStats {
    pub total_calls: u64,
    pub unique_cmds: u32,
    pub total_errors: u64,
    pub avg_duration_ns: u64,
}

/// Main ioctl app
pub struct AppIoctl {
    tracker: IoctlTracker,
}

impl AppIoctl {
    pub fn new() -> Self { Self { tracker: IoctlTracker::new() } }

    pub fn record(&mut self, cmd: u32, success: bool, dur: u64) { self.tracker.record(cmd, success, dur); }

    pub fn stats(&self) -> IoctlAppStats {
        let avg = if self.tracker.total_calls == 0 { 0 } else { self.tracker.total_time_ns / self.tracker.total_calls };
        IoctlAppStats { total_calls: self.tracker.total_calls, unique_cmds: self.tracker.cmd_counts.len() as u32, total_errors: self.tracker.total_errors, avg_duration_ns: avg }
    }
}

// ============================================================================
// Merged from ioctl_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlV2Dir {
    None,
    Read,
    Write,
    ReadWrite,
}

/// Ioctl result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlV2Result {
    Success,
    NotTty,
    InvalidArg,
    NotSupported,
    Fault,
    Busy,
    Permission,
}

/// Device class for ioctl routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlV2DevClass {
    Terminal,
    Block,
    Network,
    Scsi,
    Usb,
    Drm,
    Input,
    Sound,
    Video,
    Generic,
}

/// A decoded ioctl command.
#[derive(Debug, Clone)]
pub struct IoctlV2Command {
    pub raw: u32,
    pub direction: IoctlV2Dir,
    pub cmd_type: u8,
    pub cmd_nr: u8,
    pub arg_size: u16,
    pub name: Option<String>,
}

impl IoctlV2Command {
    pub fn decode(raw: u32) -> Self {
        let direction = match (raw >> 30) & 0x3 {
            0 => IoctlV2Dir::None,
            1 => IoctlV2Dir::Write,
            2 => IoctlV2Dir::Read,
            3 => IoctlV2Dir::ReadWrite,
            _ => IoctlV2Dir::None,
        };
        let cmd_type = ((raw >> 8) & 0xFF) as u8;
        let cmd_nr = (raw & 0xFF) as u8;
        let arg_size = ((raw >> 16) & 0x3FFF) as u16;
        Self {
            raw,
            direction,
            cmd_type,
            cmd_nr,
            arg_size,
            name: None,
        }
    }
}

/// An ioctl call record.
#[derive(Debug, Clone)]
pub struct IoctlV2Record {
    pub record_id: u64,
    pub pid: u64,
    pub fd: i32,
    pub command: IoctlV2Command,
    pub dev_class: IoctlV2DevClass,
    pub result: IoctlV2Result,
    pub is_compat: bool,
    pub timestamp: u64,
}

impl IoctlV2Record {
    pub fn new(record_id: u64, pid: u64, fd: i32, raw_cmd: u32) -> Self {
        Self {
            record_id,
            pid,
            fd,
            command: IoctlV2Command::decode(raw_cmd),
            dev_class: IoctlV2DevClass::Generic,
            result: IoctlV2Result::Success,
            is_compat: false,
            timestamp: 0,
        }
    }
}

/// Per-device routing entry.
#[derive(Debug, Clone)]
pub struct IoctlV2Route {
    pub dev_class: IoctlV2DevClass,
    pub cmd_type: u8,
    pub handler_name: String,
    pub call_count: u64,
    pub error_count: u64,
}

impl IoctlV2Route {
    pub fn new(dev_class: IoctlV2DevClass, cmd_type: u8, handler_name: String) -> Self {
        Self {
            dev_class,
            cmd_type,
            handler_name,
            call_count: 0,
            error_count: 0,
        }
    }
}

/// Statistics for ioctl V2 app.
#[derive(Debug, Clone)]
pub struct IoctlV2AppStats {
    pub total_calls: u64,
    pub total_errors: u64,
    pub compat_calls: u64,
    pub read_ioctls: u64,
    pub write_ioctls: u64,
    pub rw_ioctls: u64,
    pub routes_registered: u64,
}

/// Main apps ioctl V2 manager.
pub struct AppIoctlV2 {
    pub routes: BTreeMap<u16, IoctlV2Route>, // (type << 8 | nr) → route
    pub recent_records: Vec<IoctlV2Record>,
    pub next_record_id: u64,
    pub max_history: usize,
    pub stats: IoctlV2AppStats,
}

impl AppIoctlV2 {
    pub fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            max_history: 1024,
            stats: IoctlV2AppStats {
                total_calls: 0,
                total_errors: 0,
                compat_calls: 0,
                read_ioctls: 0,
                write_ioctls: 0,
                rw_ioctls: 0,
                routes_registered: 0,
            },
        }
    }

    pub fn register_route(&mut self, dev_class: IoctlV2DevClass, cmd_type: u8, handler: String) {
        let key = (cmd_type as u16) << 8;
        let route = IoctlV2Route::new(dev_class, cmd_type, handler);
        self.routes.insert(key, route);
        self.stats.routes_registered += 1;
    }

    pub fn record_call(&mut self, pid: u64, fd: i32, raw_cmd: u32, is_compat: bool) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let mut rec = IoctlV2Record::new(id, pid, fd, raw_cmd);
        rec.is_compat = is_compat;
        match rec.command.direction {
            IoctlV2Dir::Read => self.stats.read_ioctls += 1,
            IoctlV2Dir::Write => self.stats.write_ioctls += 1,
            IoctlV2Dir::ReadWrite => self.stats.rw_ioctls += 1,
            _ => {}
        }
        if is_compat {
            self.stats.compat_calls += 1;
        }
        self.stats.total_calls += 1;
        if self.recent_records.len() >= self.max_history {
            self.recent_records.remove(0);
        }
        self.recent_records.push(rec);
        id
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}
