// SPDX-License-Identifier: GPL-2.0
//! Holistic firmware_mgr — firmware loading, versioning, and update management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    Microcode,
    DeviceBlob,
    Dtb,
    Acpi,
    Uefi,
    OptionRom,
    Fpga,
    Regulatory,
}

/// Firmware state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareState {
    NotLoaded,
    Loading,
    Loaded,
    Applied,
    Failed,
    Stale,
}

/// Firmware security level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FirmwareSecLevel {
    Unsigned,
    SelfSigned,
    VendorSigned,
    PlatformSigned,
    SecureBoot,
}

/// Firmware version
#[derive(Debug, Clone)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u32,
}

impl FirmwareVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch, build: 0 }
    }

    pub fn to_u64(&self) -> u64 {
        ((self.major as u64) << 48) | ((self.minor as u64) << 32)
            | ((self.patch as u64) << 16) | (self.build as u64)
    }

    pub fn is_newer_than(&self, other: &FirmwareVersion) -> bool {
        self.to_u64() > other.to_u64()
    }
}

/// Firmware image descriptor
#[derive(Debug, Clone)]
pub struct FirmwareImage {
    pub id: u64,
    pub name: String,
    pub fw_type: FirmwareType,
    pub state: FirmwareState,
    pub version: FirmwareVersion,
    pub sec_level: FirmwareSecLevel,
    pub size_bytes: u64,
    pub checksum: u64,
    pub load_address: u64,
    pub device_id: u64,
    pub loaded_at: u64,
    pub load_time_ns: u64,
    pub apply_count: u32,
}

impl FirmwareImage {
    pub fn new(id: u64, name: String, fw_type: FirmwareType, ver: FirmwareVersion, size: u64) -> Self {
        Self {
            id, name, fw_type, state: FirmwareState::NotLoaded,
            version: ver, sec_level: FirmwareSecLevel::Unsigned,
            size_bytes: size, checksum: 0, load_address: 0,
            device_id: 0, loaded_at: 0, load_time_ns: 0, apply_count: 0,
        }
    }

    pub fn load(&mut self, address: u64, now: u64) {
        self.load_address = address;
        self.state = FirmwareState::Loading;
        self.loaded_at = now;
    }

    pub fn mark_loaded(&mut self, duration_ns: u64) {
        self.state = FirmwareState::Loaded;
        self.load_time_ns = duration_ns;
    }

    pub fn apply(&mut self) {
        self.state = FirmwareState::Applied;
        self.apply_count += 1;
    }

    pub fn fail(&mut self) { self.state = FirmwareState::Failed; }

    pub fn fnv_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in self.name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Firmware update request
#[derive(Debug, Clone)]
pub struct FirmwareUpdateReq {
    pub target_id: u64,
    pub new_image: FirmwareImage,
    pub force: bool,
    pub rollback_on_fail: bool,
    pub submitted_at: u64,
}

/// Firmware manager stats
#[derive(Debug, Clone)]
pub struct FirmwareMgrStats {
    pub total_images: u32,
    pub loaded_images: u32,
    pub applied_images: u32,
    pub failed_images: u32,
    pub total_bytes_loaded: u64,
    pub avg_load_time_ns: u64,
    pub pending_updates: u32,
}

/// Main firmware manager
pub struct HolisticFirmwareMgr {
    images: BTreeMap<u64, FirmwareImage>,
    pending_updates: Vec<FirmwareUpdateReq>,
    next_id: u64,
}

impl HolisticFirmwareMgr {
    pub fn new() -> Self {
        Self { images: BTreeMap::new(), pending_updates: Vec::new(), next_id: 1 }
    }

    pub fn register(&mut self, name: String, fw_type: FirmwareType, ver: FirmwareVersion, size: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.images.insert(id, FirmwareImage::new(id, name, fw_type, ver, size));
        id
    }

    pub fn load(&mut self, id: u64, address: u64, now: u64) -> bool {
        if let Some(img) = self.images.get_mut(&id) { img.load(address, now); true }
        else { false }
    }

    pub fn complete_load(&mut self, id: u64, duration_ns: u64) -> bool {
        if let Some(img) = self.images.get_mut(&id) { img.mark_loaded(duration_ns); true }
        else { false }
    }

    pub fn apply(&mut self, id: u64) -> bool {
        if let Some(img) = self.images.get_mut(&id) {
            if img.state == FirmwareState::Loaded { img.apply(); true }
            else { false }
        } else { false }
    }

    pub fn submit_update(&mut self, req: FirmwareUpdateReq) { self.pending_updates.push(req); }

    pub fn images_for_device(&self, device_id: u64) -> Vec<&FirmwareImage> {
        self.images.values().filter(|i| i.device_id == device_id).collect()
    }

    pub fn stats(&self) -> FirmwareMgrStats {
        let loaded = self.images.values().filter(|i| i.state == FirmwareState::Loaded).count() as u32;
        let applied = self.images.values().filter(|i| i.state == FirmwareState::Applied).count() as u32;
        let failed = self.images.values().filter(|i| i.state == FirmwareState::Failed).count() as u32;
        let total_bytes: u64 = self.images.values()
            .filter(|i| i.state == FirmwareState::Loaded || i.state == FirmwareState::Applied)
            .map(|i| i.size_bytes).sum();
        let load_times: Vec<u64> = self.images.values()
            .filter(|i| i.load_time_ns > 0)
            .map(|i| i.load_time_ns).collect();
        let avg_load = if load_times.is_empty() { 0 } else { load_times.iter().sum::<u64>() / load_times.len() as u64 };
        FirmwareMgrStats {
            total_images: self.images.len() as u32, loaded_images: loaded,
            applied_images: applied, failed_images: failed,
            total_bytes_loaded: total_bytes, avg_load_time_ns: avg_load,
            pending_updates: self.pending_updates.len() as u32,
        }
    }
}
