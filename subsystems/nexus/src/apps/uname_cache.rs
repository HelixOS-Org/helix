//! # Apps Uname Cache
//!
//! System information caching:
//! - Uname result caching
//! - Kernel version parsing and comparison
//! - Feature detection caching
//! - Architecture capabilities
//! - Boot parameter tracking
//! - Compat layer version matching

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Kernel version parsed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl KernelVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    #[inline(always)]
    pub fn to_code(&self) -> u32 {
        (self.major as u32) * 65536 + (self.minor as u32) * 256 + self.patch as u32
    }

    #[inline(always)]
    pub fn is_at_least(&self, major: u16, minor: u16, patch: u16) -> bool {
        self.to_code() >= KernelVersion::new(major, minor, patch).to_code()
    }
}

/// Architecture type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchType {
    X86_64,
    Aarch64,
    Riscv64,
    X86,
    Arm,
    Other,
}

/// Architecture capabilities
#[derive(Debug, Clone)]
pub struct ArchCaps {
    pub arch: ArchType,
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub has_neon: bool,
    pub has_sve: bool,
    pub page_size: usize,
    pub huge_page_sizes: [usize; 4],
    pub num_huge_sizes: usize,
    pub cache_line_size: usize,
    pub physical_addr_bits: u8,
    pub virtual_addr_bits: u8,
}

impl ArchCaps {
    #[inline]
    pub fn x86_64_default() -> Self {
        Self {
            arch: ArchType::X86_64,
            has_sse: true, has_sse2: true, has_avx: false, has_avx2: false,
            has_avx512: false, has_neon: false, has_sve: false,
            page_size: 4096, huge_page_sizes: [2 * 1024 * 1024, 1024 * 1024 * 1024, 0, 0],
            num_huge_sizes: 2, cache_line_size: 64,
            physical_addr_bits: 48, virtual_addr_bits: 48,
        }
    }
}

/// Feature flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelFeature {
    Cgroups,
    CgroupsV2,
    Namespaces,
    Seccomp,
    IoUring,
    Bpf,
    Landlock,
    MemoryProtectionKeys,
    ShadowCallStack,
    Userfaultfd,
    Ktls,
}

/// Cached uname info
#[derive(Debug, Clone)]
pub struct UnameInfo {
    pub sysname: String,
    pub nodename: String,
    pub release: String,
    pub version_string: String,
    pub machine: String,
    pub parsed_version: KernelVersion,
}

impl UnameInfo {
    pub fn new(sysname: String, node: String, release: String, ver: String, machine: String, kv: KernelVersion) -> Self {
        Self { sysname, nodename: node, release, version_string: ver, machine, parsed_version: kv }
    }
}

/// Boot parameter
#[derive(Debug, Clone)]
pub struct BootParam {
    pub key: String,
    pub value: Option<String>,
}

/// Uname cache stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct UnameCacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub feature_queries: u64,
    pub version_checks: u64,
}

/// Apps uname cache
#[repr(align(64))]
pub struct AppsUnameCache {
    uname: Option<UnameInfo>,
    arch: ArchCaps,
    features: BTreeMap<u8, bool>,
    boot_params: BTreeMap<String, BootParam>,
    stats: UnameCacheStats,
}

impl AppsUnameCache {
    pub fn new() -> Self {
        Self {
            uname: None, arch: ArchCaps::x86_64_default(),
            features: BTreeMap::new(), boot_params: BTreeMap::new(),
            stats: UnameCacheStats::default(),
        }
    }

    #[inline(always)]
    pub fn set_uname(&mut self, info: UnameInfo) { self.uname = Some(info); }
    #[inline(always)]
    pub fn set_arch(&mut self, caps: ArchCaps) { self.arch = caps; }

    #[inline(always)]
    pub fn register_feature(&mut self, feature: KernelFeature, available: bool) {
        self.features.insert(feature as u8, available);
    }

    #[inline(always)]
    pub fn has_feature(&mut self, feature: KernelFeature) -> bool {
        self.stats.feature_queries += 1;
        self.features.get(&(feature as u8)).copied().unwrap_or(false)
    }

    #[inline(always)]
    pub fn kernel_version(&self) -> Option<KernelVersion> {
        self.uname.as_ref().map(|u| u.parsed_version)
    }

    #[inline(always)]
    pub fn is_at_least(&mut self, major: u16, minor: u16, patch: u16) -> bool {
        self.stats.version_checks += 1;
        self.kernel_version().map(|v| v.is_at_least(major, minor, patch)).unwrap_or(false)
    }

    #[inline(always)]
    pub fn add_boot_param(&mut self, key: String, value: Option<String>) {
        let param = BootParam { key: key.clone(), value };
        self.boot_params.insert(key, param);
    }

    #[inline(always)]
    pub fn get_boot_param(&self, key: &str) -> Option<&BootParam> { self.boot_params.get(key) }

    #[inline]
    pub fn uname(&mut self) -> Option<&UnameInfo> {
        if self.uname.is_some() { self.stats.cache_hits += 1; }
        else { self.stats.cache_misses += 1; }
        self.uname.as_ref()
    }

    #[inline(always)]
    pub fn arch(&self) -> &ArchCaps { &self.arch }
    #[inline(always)]
    pub fn stats(&self) -> &UnameCacheStats { &self.stats }
}
