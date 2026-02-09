// SPDX-License-Identifier: GPL-2.0
//! Bridge device proxy — kernel device model ↔ userspace device access translation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevClass {
    /// Character device
    Char,
    /// Block device
    Block,
    /// Network device
    Net,
    /// Input device (keyboard, mouse)
    Input,
    /// Sound device
    Sound,
    /// Video device (v4l2, DRM)
    Video,
    /// USB device
    Usb,
    /// PCI device
    Pci,
    /// Virtual device
    Virtual,
    /// Platform device
    Platform,
}

/// Device power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DevPowerState {
    /// Fully on
    D0,
    /// Light sleep
    D1,
    /// Deeper sleep
    D2,
    /// Power off (can be resumed)
    D3Hot,
    /// Fully off
    D3Cold,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOp {
    Read,
    Write,
    Ioctl,
    Mmap,
    Poll,
}

/// Major/minor device number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevNumber {
    pub major: u32,
    pub minor: u32,
}

impl DevNumber {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    #[inline(always)]
    pub fn combined(&self) -> u64 {
        ((self.major as u64) << 20) | (self.minor as u64)
    }
}

/// Device resource
#[derive(Debug, Clone)]
pub struct DevResource {
    pub base: u64,
    pub size: u64,
    pub is_io: bool,
    pub name: String,
}

impl DevResource {
    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.base + self.size
    }
}

/// Registered device descriptor
#[derive(Debug)]
pub struct DevDescriptor {
    pub dev_num: DevNumber,
    pub class: DevClass,
    pub name: String,
    pub driver: String,
    pub power_state: DevPowerState,
    pub resources: Vec<DevResource>,
    pub irq: Option<u32>,
    pub open_count: u32,
    pub io_reads: u64,
    pub io_writes: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
    pub error_count: u64,
    pub dma_capable: bool,
    pub hotpluggable: bool,
}

impl DevDescriptor {
    pub fn new(dev_num: DevNumber, class: DevClass, name: String, driver: String) -> Self {
        Self {
            dev_num,
            class,
            name,
            driver,
            power_state: DevPowerState::D0,
            resources: Vec::new(),
            irq: None,
            open_count: 0,
            io_reads: 0,
            io_writes: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
            error_count: 0,
            dma_capable: false,
            hotpluggable: false,
        }
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.power_state == DevPowerState::D0
    }

    #[inline(always)]
    pub fn total_io_ops(&self) -> u64 {
        self.io_reads.saturating_add(self.io_writes)
    }

    #[inline(always)]
    pub fn total_io_bytes(&self) -> u64 {
        self.io_bytes_read.saturating_add(self.io_bytes_written)
    }

    #[inline]
    pub fn read_write_ratio(&self) -> f64 {
        if self.io_writes == 0 {
            return if self.io_reads > 0 { f64::MAX } else { 0.0 };
        }
        self.io_reads as f64 / self.io_writes as f64
    }
}

/// Permission check for device access
#[derive(Debug, Clone)]
pub struct DevPermission {
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
}

impl DevPermission {
    pub fn new(uid: u32, gid: u32, mode: u16) -> Self {
        Self { uid, gid, mode }
    }

    #[inline]
    pub fn can_read(&self, uid: u32, gid: u32) -> bool {
        if uid == 0 { return true; }
        if uid == self.uid { return (self.mode & 0o400) != 0; }
        if gid == self.gid { return (self.mode & 0o040) != 0; }
        (self.mode & 0o004) != 0
    }

    #[inline]
    pub fn can_write(&self, uid: u32, gid: u32) -> bool {
        if uid == 0 { return true; }
        if uid == self.uid { return (self.mode & 0o200) != 0; }
        if gid == self.gid { return (self.mode & 0o020) != 0; }
        (self.mode & 0o002) != 0
    }
}

/// Hotplug event
#[derive(Debug, Clone)]
pub struct HotplugEvent {
    pub dev_num: DevNumber,
    pub action: HotplugAction,
    pub timestamp_ns: u64,
    pub subsystem: String,
}

/// Hotplug action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugAction {
    Add,
    Remove,
    Change,
    Move,
    Bind,
    Unbind,
}

/// I/O request record
#[derive(Debug, Clone)]
pub struct IoRequest {
    pub dev_id: u64,
    pub op: IoOp,
    pub offset: u64,
    pub size: u64,
    pub pid: u64,
    pub timestamp_ns: u64,
    pub duration_ns: u64,
}

/// Device proxy stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DevProxyStats {
    pub devices_registered: u64,
    pub hotplug_events: u64,
    pub io_requests: u64,
    pub io_errors: u64,
    pub permission_denied: u64,
    pub power_transitions: u64,
}

/// Main bridge device proxy
#[repr(align(64))]
pub struct BridgeDevProxy {
    devices: BTreeMap<u64, DevDescriptor>,
    permissions: BTreeMap<u64, DevPermission>,
    hotplug_log: VecDeque<HotplugEvent>,
    io_log: VecDeque<IoRequest>,
    max_io_log: usize,
    max_hotplug_log: usize,
    stats: DevProxyStats,
}

impl BridgeDevProxy {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            permissions: BTreeMap::new(),
            hotplug_log: VecDeque::new(),
            io_log: VecDeque::new(),
            max_io_log: 4096,
            max_hotplug_log: 1024,
            stats: DevProxyStats {
                devices_registered: 0,
                hotplug_events: 0,
                io_requests: 0,
                io_errors: 0,
                permission_denied: 0,
                power_transitions: 0,
            },
        }
    }

    #[inline]
    pub fn register_device(
        &mut self,
        dev_num: DevNumber,
        class: DevClass,
        name: String,
        driver: String,
    ) -> u64 {
        let id = dev_num.combined();
        let dev = DevDescriptor::new(dev_num, class, name, driver);
        self.devices.insert(id, dev);
        self.stats.devices_registered += 1;
        self.log_hotplug(dev_num, HotplugAction::Add, String::new());
        id
    }

    #[inline]
    pub fn unregister_device(&mut self, dev_id: u64) -> bool {
        if let Some(dev) = self.devices.remove(&dev_id) {
            self.permissions.remove(&dev_id);
            self.log_hotplug(dev.dev_num, HotplugAction::Remove, String::new());
            true
        } else {
            false
        }
    }

    fn log_hotplug(&mut self, dev_num: DevNumber, action: HotplugAction, subsystem: String) {
        let event = HotplugEvent {
            dev_num,
            action,
            timestamp_ns: 0,
            subsystem,
        };
        self.hotplug_log.push_back(event);
        if self.hotplug_log.len() > self.max_hotplug_log {
            self.hotplug_log.pop_front();
        }
        self.stats.hotplug_events += 1;
    }

    #[inline]
    pub fn set_permission(&mut self, dev_id: u64, perm: DevPermission) -> bool {
        if self.devices.contains_key(&dev_id) {
            self.permissions.insert(dev_id, perm);
            true
        } else {
            false
        }
    }

    pub fn open_device(&mut self, dev_id: u64, uid: u32, gid: u32) -> bool {
        if let Some(perm) = self.permissions.get(&dev_id) {
            if !perm.can_read(uid, gid) {
                self.stats.permission_denied += 1;
                return false;
            }
        }
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            if !dev.is_active() {
                return false;
            }
            dev.open_count += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn close_device(&mut self, dev_id: u64) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            if dev.open_count > 0 {
                dev.open_count -= 1;
            }
            true
        } else {
            false
        }
    }

    pub fn record_io(
        &mut self,
        dev_id: u64,
        op: IoOp,
        offset: u64,
        size: u64,
        pid: u64,
        duration_ns: u64,
    ) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            match op {
                IoOp::Read => {
                    dev.io_reads += 1;
                    dev.io_bytes_read = dev.io_bytes_read.saturating_add(size);
                }
                IoOp::Write => {
                    dev.io_writes += 1;
                    dev.io_bytes_written = dev.io_bytes_written.saturating_add(size);
                }
                _ => {}
            }
            let req = IoRequest {
                dev_id,
                op,
                offset,
                size,
                pid,
                timestamp_ns: 0,
                duration_ns,
            };
            self.io_log.push_back(req);
            if self.io_log.len() > self.max_io_log {
                self.io_log.pop_front();
            }
            self.stats.io_requests += 1;
            true
        } else {
            false
        }
    }

    pub fn set_power_state(&mut self, dev_id: u64, state: DevPowerState) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            if dev.open_count > 0 && state != DevPowerState::D0 {
                return false; // Can't sleep while open
            }
            dev.power_state = state;
            self.stats.power_transitions += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn add_resource(&mut self, dev_id: u64, resource: DevResource) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            dev.resources.push(resource);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn find_device_by_resource(&self, addr: u64) -> Option<u64> {
        for (id, dev) in &self.devices {
            if dev.resources.iter().any(|r| r.contains(addr)) {
                return Some(*id);
            }
        }
        None
    }

    #[inline]
    pub fn devices_by_class(&self, class: DevClass) -> Vec<u64> {
        self.devices
            .iter()
            .filter(|(_, d)| d.class == class)
            .map(|(id, _)| *id)
            .collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &DevProxyStats {
        &self.stats
    }
}
