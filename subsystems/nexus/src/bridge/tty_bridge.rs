//! # Bridge TTY Bridge
//!
//! Terminal/TTY/PTY syscall bridging:
//! - PTY master/slave pair management
//! - Terminal attribute (termios) tracking
//! - Line discipline processing
//! - Window size change (TIOCSWINSZ) handling
//! - Terminal input/output byte counting
//! - Session and controlling terminal tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TTY type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtyType {
    Console,
    Serial,
    PtyMaster,
    PtySlave,
    Virtual,
}

/// Line discipline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDiscipline {
    N_TTY,
    N_SLIP,
    N_PPP,
    N_RAW,
    N_NULL,
}

/// Terminal attributes (simplified termios)
#[derive(Debug, Clone)]
pub struct TermiosAttrs {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub ispeed: u32,
    pub ospeed: u32,
    pub vmin: u8,
    pub vtime: u8,
    pub echo: bool,
    pub canonical: bool,
    pub raw_mode: bool,
}

impl Default for TermiosAttrs {
    fn default() -> Self {
        Self {
            iflag: 0x2500, oflag: 0x0005, cflag: 0x00bf, lflag: 0x8a3b,
            ispeed: 38400, ospeed: 38400, vmin: 1, vtime: 0,
            echo: true, canonical: true, raw_mode: false,
        }
    }
}

impl TermiosAttrs {
    #[inline]
    pub fn set_raw(&mut self) {
        self.canonical = false;
        self.echo = false;
        self.raw_mode = true;
        self.vmin = 1;
        self.vtime = 0;
    }
}

/// Window size
#[derive(Debug, Clone, Copy)]
pub struct WinSize {
    pub rows: u16,
    pub cols: u16,
    pub xpixel: u16,
    pub ypixel: u16,
}

impl Default for WinSize {
    fn default() -> Self { Self { rows: 24, cols: 80, xpixel: 0, ypixel: 0 } }
}

/// TTY device
#[derive(Debug, Clone)]
pub struct TtyDevice {
    pub tty_id: u32,
    pub tty_type: TtyType,
    pub major: u32,
    pub minor: u32,
    pub ldisc: LineDiscipline,
    pub attrs: TermiosAttrs,
    pub winsize: WinSize,
    pub session_id: u64,
    pub fg_pgrp: u64,
    pub master_fd: Option<i32>,
    pub slave_fd: Option<i32>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub read_count: u64,
    pub write_count: u64,
    pub ioctl_count: u64,
    pub open_count: u32,
    pub hung_up: bool,
}

impl TtyDevice {
    pub fn new(id: u32, ttype: TtyType) -> Self {
        Self {
            tty_id: id, tty_type: ttype, major: 136, minor: id,
            ldisc: LineDiscipline::N_TTY, attrs: TermiosAttrs::default(),
            winsize: WinSize::default(), session_id: 0, fg_pgrp: 0,
            master_fd: None, slave_fd: None, bytes_in: 0, bytes_out: 0,
            read_count: 0, write_count: 0, ioctl_count: 0, open_count: 0,
            hung_up: false,
        }
    }

    #[inline(always)]
    pub fn read(&mut self, bytes: u64) { self.bytes_in += bytes; self.read_count += 1; }
    #[inline(always)]
    pub fn write(&mut self, bytes: u64) { self.bytes_out += bytes; self.write_count += 1; }
    #[inline(always)]
    pub fn ioctl(&mut self) { self.ioctl_count += 1; }
    #[inline(always)]
    pub fn hangup(&mut self) { self.hung_up = true; }
    #[inline(always)]
    pub fn set_winsize(&mut self, ws: WinSize) { self.winsize = ws; }
    #[inline(always)]
    pub fn set_session(&mut self, sid: u64, pgrp: u64) { self.session_id = sid; self.fg_pgrp = pgrp; }
}

/// PTY pair
#[derive(Debug, Clone)]
pub struct PtyPair {
    pub master_id: u32,
    pub slave_id: u32,
    pub pts_number: u32,
    pub created_ts: u64,
    pub owner_pid: u64,
}

impl PtyPair {
    pub fn new(master: u32, slave: u32, pts: u32, owner: u64, ts: u64) -> Self {
        Self { master_id: master, slave_id: slave, pts_number: pts, created_ts: ts, owner_pid: owner }
    }
}

/// TTY bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TtyBridgeStats {
    pub total_ttys: usize,
    pub pty_pairs: usize,
    pub consoles: usize,
    pub total_bytes_in: u64,
    pub total_bytes_out: u64,
    pub total_ioctls: u64,
    pub hung_up_count: usize,
    pub raw_mode_count: usize,
}

/// Bridge TTY manager
#[repr(align(64))]
pub struct BridgeTtyBridge {
    ttys: BTreeMap<u32, TtyDevice>,
    pty_pairs: Vec<PtyPair>,
    next_tty_id: u32,
    next_pts: u32,
    stats: TtyBridgeStats,
}

impl BridgeTtyBridge {
    pub fn new() -> Self {
        Self { ttys: BTreeMap::new(), pty_pairs: Vec::new(), next_tty_id: 1, next_pts: 0, stats: TtyBridgeStats::default() }
    }

    #[inline]
    pub fn create_tty(&mut self, ttype: TtyType) -> u32 {
        let id = self.next_tty_id;
        self.next_tty_id += 1;
        self.ttys.insert(id, TtyDevice::new(id, ttype));
        id
    }

    #[inline]
    pub fn create_pty(&mut self, owner: u64, ts: u64) -> (u32, u32) {
        let master = self.create_tty(TtyType::PtyMaster);
        let slave = self.create_tty(TtyType::PtySlave);
        let pts = self.next_pts;
        self.next_pts += 1;
        self.pty_pairs.push(PtyPair::new(master, slave, pts, owner, ts));
        (master, slave)
    }

    #[inline(always)]
    pub fn read(&mut self, tty_id: u32, bytes: u64) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.read(bytes); }
    }

    #[inline(always)]
    pub fn write(&mut self, tty_id: u32, bytes: u64) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.write(bytes); }
    }

    #[inline(always)]
    pub fn ioctl(&mut self, tty_id: u32) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.ioctl(); }
    }

    #[inline(always)]
    pub fn set_attrs(&mut self, tty_id: u32, attrs: TermiosAttrs) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.attrs = attrs; }
    }

    #[inline(always)]
    pub fn set_raw(&mut self, tty_id: u32) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.attrs.set_raw(); }
    }

    #[inline(always)]
    pub fn set_winsize(&mut self, tty_id: u32, ws: WinSize) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.set_winsize(ws); }
    }

    #[inline(always)]
    pub fn hangup(&mut self, tty_id: u32) {
        if let Some(t) = self.ttys.get_mut(&tty_id) { t.hangup(); }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_ttys = self.ttys.len();
        self.stats.pty_pairs = self.pty_pairs.len();
        self.stats.consoles = self.ttys.values().filter(|t| t.tty_type == TtyType::Console).count();
        self.stats.total_bytes_in = self.ttys.values().map(|t| t.bytes_in).sum();
        self.stats.total_bytes_out = self.ttys.values().map(|t| t.bytes_out).sum();
        self.stats.total_ioctls = self.ttys.values().map(|t| t.ioctl_count).sum();
        self.stats.hung_up_count = self.ttys.values().filter(|t| t.hung_up).count();
        self.stats.raw_mode_count = self.ttys.values().filter(|t| t.attrs.raw_mode).count();
    }

    #[inline(always)]
    pub fn tty(&self, id: u32) -> Option<&TtyDevice> { self.ttys.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &TtyBridgeStats { &self.stats }
}

// ============================================================================
// Merged from tty_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtyV2Ldisc {
    N_TTY,
    N_SLIP,
    N_MOUSE,
    N_PPP,
    N_HDLC,
    N_GSM,
    N_RAW,
}

/// Terminal mode flags
#[derive(Debug, Clone)]
pub struct TtyV2Termios {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub ispeed: u32,
    pub ospeed: u32,
    pub echo: bool,
    pub canonical: bool,
    pub raw: bool,
}

impl TtyV2Termios {
    pub fn default_termios() -> Self {
        Self {
            iflag: 0x0500,  // ICRNL | IXON
            oflag: 0x0005,  // OPOST | ONLCR
            cflag: 0x00BF,  // CS8 | CREAD | HUPCL
            lflag: 0x8A3B,  // ISIG | ICANON | ECHO | ECHOE | ECHOK | ECHOCTL | ECHOKE | IEXTEN
            ispeed: 38400,
            ospeed: 38400,
            echo: true,
            canonical: true,
            raw: false,
        }
    }

    #[inline]
    pub fn set_raw(&mut self) {
        self.echo = false;
        self.canonical = false;
        self.raw = true;
        self.iflag = 0;
        self.oflag = 0;
        self.lflag = 0;
    }
}

/// A TTY device instance
#[derive(Debug, Clone)]
pub struct TtyV2Device {
    pub minor: u32,
    pub name: String,
    pub ldisc: TtyV2Ldisc,
    pub termios: TtyV2Termios,
    pub session_id: Option<u64>,
    pub foreground_pgrp: Option<u64>,
    pub input_buf: Vec<u8>,
    pub output_buf: Vec<u8>,
    pub columns: u16,
    pub rows: u16,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub open_count: u32,
    pub hangup: bool,
}

impl TtyV2Device {
    pub fn new(minor: u32, name: String) -> Self {
        Self {
            minor,
            name,
            ldisc: TtyV2Ldisc::N_TTY,
            termios: TtyV2Termios::default_termios(),
            session_id: None,
            foreground_pgrp: None,
            input_buf: Vec::new(),
            output_buf: Vec::new(),
            columns: 80,
            rows: 24,
            bytes_written: 0,
            bytes_read: 0,
            open_count: 0,
            hangup: false,
        }
    }

    #[inline]
    pub fn write_data(&mut self, data: &[u8]) -> usize {
        let len = data.len();
        self.output_buf.extend_from_slice(data);
        self.bytes_written += len as u64;
        len
    }

    #[inline]
    pub fn read_data(&mut self, max: usize) -> Vec<u8> {
        let count = max.min(self.input_buf.len());
        let data: Vec<u8> = self.input_buf.drain(..count).collect();
        self.bytes_read += data.len() as u64;
        data
    }

    #[inline(always)]
    pub fn set_winsize(&mut self, cols: u16, rows: u16) {
        self.columns = cols;
        self.rows = rows;
    }

    #[inline]
    pub fn hangup_device(&mut self) {
        self.hangup = true;
        self.session_id = None;
        self.foreground_pgrp = None;
    }

    #[inline(always)]
    pub fn set_session(&mut self, sid: u64, pgrp: u64) {
        self.session_id = Some(sid);
        self.foreground_pgrp = Some(pgrp);
    }
}

/// Statistics for TTY V2 bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TtyV2BridgeStats {
    pub devices_created: u64,
    pub total_opens: u64,
    pub total_closes: u64,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub hangups: u64,
    pub winsize_changes: u64,
    pub ldisc_changes: u64,
}

/// Main TTY V2 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeTtyV2 {
    devices: BTreeMap<u32, TtyV2Device>,
    next_minor: u32,
    stats: TtyV2BridgeStats,
}

impl BridgeTtyV2 {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            next_minor: 0,
            stats: TtyV2BridgeStats {
                devices_created: 0,
                total_opens: 0,
                total_closes: 0,
                bytes_written: 0,
                bytes_read: 0,
                hangups: 0,
                winsize_changes: 0,
                ldisc_changes: 0,
            },
        }
    }

    #[inline]
    pub fn create_device(&mut self, name: String) -> u32 {
        let minor = self.next_minor;
        self.next_minor += 1;
        self.devices.insert(minor, TtyV2Device::new(minor, name));
        self.stats.devices_created += 1;
        minor
    }

    #[inline]
    pub fn open_device(&mut self, minor: u32) -> bool {
        if let Some(dev) = self.devices.get_mut(&minor) {
            dev.open_count += 1;
            self.stats.total_opens += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn write(&mut self, minor: u32, data: &[u8]) -> usize {
        if let Some(dev) = self.devices.get_mut(&minor) {
            let written = dev.write_data(data);
            self.stats.bytes_written += written as u64;
            written
        } else {
            0
        }
    }

    #[inline]
    pub fn read(&mut self, minor: u32, max: usize) -> Vec<u8> {
        if let Some(dev) = self.devices.get_mut(&minor) {
            let data = dev.read_data(max);
            self.stats.bytes_read += data.len() as u64;
            data
        } else {
            Vec::new()
        }
    }

    #[inline]
    pub fn set_winsize(&mut self, minor: u32, cols: u16, rows: u16) -> bool {
        if let Some(dev) = self.devices.get_mut(&minor) {
            dev.set_winsize(cols, rows);
            self.stats.winsize_changes += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &TtyV2BridgeStats {
        &self.stats
    }
}
