// SPDX-License-Identifier: GPL-2.0
//! Bridge kexec — kernel execution (kexec/kdump) management proxy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Kexec operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KexecType {
    /// Normal kexec into a new kernel
    Normal,
    /// Crash dump kernel (kdump)
    Crash,
    /// Live patching via kexec
    LivePatch,
}

/// Kexec segment — a chunk of the new kernel image
#[derive(Debug, Clone)]
pub struct KexecSegment {
    pub buf_start: u64,
    pub buf_size: u64,
    pub mem_start: u64,
    pub mem_size: u64,
    checksum: u32,
}

impl KexecSegment {
    pub fn new(buf_start: u64, buf_size: u64, mem_start: u64, mem_size: u64) -> Self {
        // FNV-1a checksum of addresses
        let mut hash: u32 = 0x811c9dc5;
        for &b in &buf_start.to_le_bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        for &b in &mem_start.to_le_bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        Self {
            buf_start,
            buf_size,
            mem_start,
            mem_size,
            checksum: hash,
        }
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.buf_size > 0 && self.mem_size >= self.buf_size
    }

    #[inline(always)]
    pub fn mem_end(&self) -> u64 {
        self.mem_start.saturating_add(self.mem_size)
    }

    #[inline(always)]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.mem_start < other.mem_end() && other.mem_start < self.mem_end()
    }
}

/// Loaded kexec image state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageState {
    /// No image loaded
    Empty,
    /// Image being loaded
    Loading,
    /// Image loaded and verified
    Ready,
    /// Image integrity check failed
    Invalid,
    /// Currently executing kexec
    Executing,
}

/// A loaded kexec image
#[derive(Debug)]
#[repr(align(64))]
pub struct KexecImage {
    pub kexec_type: KexecType,
    pub state: ImageState,
    pub segments: Vec<KexecSegment>,
    pub entry_point: u64,
    pub cmdline: String,
    pub initrd_start: u64,
    pub initrd_size: u64,
    pub total_size: u64,
    loaded_ns: u64,
    verify_count: u32,
}

impl KexecImage {
    pub fn new(kexec_type: KexecType, entry_point: u64) -> Self {
        Self {
            kexec_type,
            state: ImageState::Empty,
            segments: Vec::new(),
            entry_point,
            cmdline: String::new(),
            initrd_start: 0,
            initrd_size: 0,
            total_size: 0,
            loaded_ns: 0,
            verify_count: 0,
        }
    }

    pub fn add_segment(&mut self, segment: KexecSegment) -> bool {
        if !segment.is_valid() {
            return false;
        }
        // Check for overlaps
        for existing in &self.segments {
            if existing.overlaps(&segment) {
                return false;
            }
        }
        self.total_size = self.total_size.saturating_add(segment.mem_size);
        self.segments.push(segment);
        true
    }

    #[inline(always)]
    pub fn set_cmdline(&mut self, cmdline: String) {
        self.cmdline = cmdline;
    }

    #[inline(always)]
    pub fn set_initrd(&mut self, start: u64, size: u64) {
        self.initrd_start = start;
        self.initrd_size = size;
    }

    pub fn verify_integrity(&mut self) -> bool {
        self.verify_count += 1;
        if self.segments.is_empty() {
            self.state = ImageState::Invalid;
            return false;
        }
        if self.entry_point == 0 {
            self.state = ImageState::Invalid;
            return false;
        }
        // Verify all segments are valid and non-overlapping
        for i in 0..self.segments.len() {
            if !self.segments[i].is_valid() {
                self.state = ImageState::Invalid;
                return false;
            }
            for j in (i + 1)..self.segments.len() {
                if self.segments[i].overlaps(&self.segments[j]) {
                    self.state = ImageState::Invalid;
                    return false;
                }
            }
        }
        self.state = ImageState::Ready;
        true
    }

    #[inline(always)]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

/// Crash dump reservation region
#[derive(Debug, Clone)]
pub struct CrashReserveRegion {
    pub start: u64,
    pub size: u64,
    pub in_use: bool,
}

impl CrashReserveRegion {
    pub fn new(start: u64, size: u64) -> Self {
        Self { start, size, in_use: false }
    }

    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.start + self.size
    }
}

/// Purgatory (pre-boot code) status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgatoryState {
    NotLoaded,
    Loaded,
    Verified,
    Running,
    Failed,
}

/// Shutdown notification callback registration
#[derive(Debug, Clone)]
pub struct ShutdownNotifier {
    pub name: String,
    pub priority: u32,
    pub notified: bool,
    pub timeout_ns: u64,
}

/// Kexec bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KexecBridgeStats {
    pub images_loaded: u64,
    pub kexec_executions: u64,
    pub crash_dumps: u64,
    pub verification_failures: u64,
    pub segment_errors: u64,
    pub notifiers_called: u64,
}

/// Main kexec bridge manager
pub struct BridgeKexec {
    normal_image: Option<KexecImage>,
    crash_image: Option<KexecImage>,
    crash_regions: Vec<CrashReserveRegion>,
    purgatory_state: PurgatoryState,
    shutdown_notifiers: Vec<ShutdownNotifier>,
    max_segments: usize,
    stats: KexecBridgeStats,
}

impl BridgeKexec {
    pub fn new() -> Self {
        Self {
            normal_image: None,
            crash_image: None,
            crash_regions: Vec::new(),
            purgatory_state: PurgatoryState::NotLoaded,
            shutdown_notifiers: Vec::new(),
            max_segments: 128,
            stats: KexecBridgeStats {
                images_loaded: 0,
                kexec_executions: 0,
                crash_dumps: 0,
                verification_failures: 0,
                segment_errors: 0,
                notifiers_called: 0,
            },
        }
    }

    pub fn load_image(
        &mut self,
        kexec_type: KexecType,
        entry_point: u64,
        segments: Vec<KexecSegment>,
        cmdline: String,
    ) -> bool {
        if segments.len() > self.max_segments {
            return false;
        }
        let mut image = KexecImage::new(kexec_type, entry_point);
        for seg in segments {
            if !image.add_segment(seg) {
                self.stats.segment_errors += 1;
                return false;
            }
        }
        image.set_cmdline(cmdline);
        image.state = ImageState::Loading;
        if !image.verify_integrity() {
            self.stats.verification_failures += 1;
            return false;
        }
        match kexec_type {
            KexecType::Crash => self.crash_image = Some(image),
            _ => self.normal_image = Some(image),
        }
        self.stats.images_loaded += 1;
        true
    }

    pub fn unload_image(&mut self, kexec_type: KexecType) -> bool {
        match kexec_type {
            KexecType::Crash => {
                if self.crash_image.is_some() {
                    self.crash_image = None;
                    true
                } else {
                    false
                }
            }
            _ => {
                if self.normal_image.is_some() {
                    self.normal_image = None;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn execute_kexec(&mut self) -> bool {
        if let Some(ref mut image) = self.normal_image {
            if image.state != ImageState::Ready {
                return false;
            }
            // Run shutdown notifiers
            for notifier in &mut self.shutdown_notifiers {
                notifier.notified = true;
                self.stats.notifiers_called += 1;
            }
            image.state = ImageState::Executing;
            self.purgatory_state = PurgatoryState::Running;
            self.stats.kexec_executions += 1;
            true
        } else {
            false
        }
    }

    pub fn trigger_crash_dump(&mut self) -> bool {
        if let Some(ref mut image) = self.crash_image {
            if image.state != ImageState::Ready {
                return false;
            }
            image.state = ImageState::Executing;
            self.stats.crash_dumps += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn reserve_crash_region(&mut self, start: u64, size: u64) -> bool {
        // Check for overlaps with existing reservations
        for region in &self.crash_regions {
            if region.start < start + size && start < region.start + region.size {
                return false;
            }
        }
        self.crash_regions.push(CrashReserveRegion::new(start, size));
        true
    }

    #[inline]
    pub fn register_shutdown_notifier(&mut self, name: String, priority: u32, timeout_ns: u64) {
        self.shutdown_notifiers.push(ShutdownNotifier {
            name,
            priority,
            notified: false,
            timeout_ns,
        });
        self.shutdown_notifiers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    #[inline(always)]
    pub fn set_purgatory_state(&mut self, state: PurgatoryState) {
        self.purgatory_state = state;
    }

    #[inline]
    pub fn image_info(&self, kexec_type: KexecType) -> Option<(ImageState, u64, usize)> {
        let image = match kexec_type {
            KexecType::Crash => self.crash_image.as_ref(),
            _ => self.normal_image.as_ref(),
        };
        image.map(|img| (img.state, img.total_size, img.segment_count()))
    }

    #[inline(always)]
    pub fn total_crash_reserved(&self) -> u64 {
        self.crash_regions.iter().map(|r| r.size).sum()
    }

    #[inline(always)]
    pub fn stats(&self) -> &KexecBridgeStats {
        &self.stats
    }
}
