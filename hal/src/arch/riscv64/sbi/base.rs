//! # SBI Base Extension
//!
//! The base extension is mandatory and provides core SBI functionality.
//!
//! ## Functions
//!
//! - `get_spec_version`: Get SBI specification version
//! - `get_impl_id`: Get SBI implementation ID
//! - `get_impl_version`: Get SBI implementation version
//! - `probe_extension`: Check if an extension is available
//! - `get_mvendorid`: Get machine vendor ID
//! - `get_marchid`: Get machine architecture ID
//! - `get_mimpid`: Get machine implementation ID

use super::{eid, base_fid};

// ============================================================================
// SBI Return Type
// ============================================================================

/// SBI call return value
#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    /// Error code (0 = success)
    pub error: i64,
    /// Return value
    pub value: i64,
}

impl SbiRet {
    /// Check if the call succeeded
    pub fn is_success(&self) -> bool {
        self.error == 0
    }

    /// Get result as Option
    pub fn ok(self) -> Option<i64> {
        if self.is_success() {
            Some(self.value)
        } else {
            None
        }
    }

    /// Get result as Result
    pub fn into_result(self) -> Result<i64, i64> {
        if self.is_success() {
            Ok(self.value)
        } else {
            Err(self.error)
        }
    }
}

// ============================================================================
// SBI Call Wrappers
// ============================================================================

/// Make an SBI call with no arguments
#[inline]
pub fn sbi_call_0(eid: usize, fid: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 1 argument
#[inline]
pub fn sbi_call_1(eid: usize, fid: usize, arg0: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 2 arguments
#[inline]
pub fn sbi_call_2(eid: usize, fid: usize, arg0: usize, arg1: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 3 arguments
#[inline]
pub fn sbi_call_3(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 4 arguments
#[inline]
pub fn sbi_call_4(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 5 arguments
#[inline]
pub fn sbi_call_5(
    eid: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 6 arguments
#[inline]
pub fn sbi_call_6(
    eid: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Generic SBI call
#[inline]
pub fn sbi_call(eid: usize, fid: usize, args: &[usize]) -> SbiRet {
    match args.len() {
        0 => sbi_call_0(eid, fid),
        1 => sbi_call_1(eid, fid, args[0]),
        2 => sbi_call_2(eid, fid, args[0], args[1]),
        3 => sbi_call_3(eid, fid, args[0], args[1], args[2]),
        4 => sbi_call_4(eid, fid, args[0], args[1], args[2], args[3]),
        5 => sbi_call_5(eid, fid, args[0], args[1], args[2], args[3], args[4]),
        _ => sbi_call_6(eid, fid, args[0], args[1], args[2], args[3], args[4], args[5]),
    }
}

// ============================================================================
// Base Extension Functions
// ============================================================================

/// Get SBI specification version
///
/// Returns (major, minor) version tuple.
pub fn get_spec_version() -> (u32, u32) {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_SPEC_VERSION);
    let version = ret.value as u32;
    let major = (version >> 24) & 0x7F;
    let minor = version & 0xFF_FFFF;
    (major, minor)
}

/// Get SBI implementation ID
pub fn get_impl_id() -> u64 {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_IMPL_ID);
    ret.value as u64
}

/// Get SBI implementation version
pub fn get_impl_version() -> u64 {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_IMPL_VERSION);
    ret.value as u64
}

/// Probe if an extension is available
pub fn probe_extension(extension_id: usize) -> bool {
    let ret = sbi_call_1(eid::BASE, base_fid::PROBE_EXTENSION, extension_id);
    ret.value != 0
}

/// Get machine vendor ID (mvendorid CSR)
pub fn get_mvendorid() -> u64 {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_MVENDORID);
    ret.value as u64
}

/// Get machine architecture ID (marchid CSR)
pub fn get_marchid() -> u64 {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_MARCHID);
    ret.value as u64
}

/// Get machine implementation ID (mimpid CSR)
pub fn get_mimpid() -> u64 {
    let ret = sbi_call_0(eid::BASE, base_fid::GET_MIMPID);
    ret.value as u64
}

// ============================================================================
// Version Parsing
// ============================================================================

/// Parse a version number into components
pub fn parse_version(version: u64) -> (u32, u32, u32) {
    let major = ((version >> 48) & 0xFFFF) as u32;
    let minor = ((version >> 32) & 0xFFFF) as u32;
    let patch = (version & 0xFFFF_FFFF) as u32;
    (major, minor, patch)
}

/// SBI version structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbiVersion {
    pub major: u32,
    pub minor: u32,
}

impl SbiVersion {
    /// Query the current SBI version
    pub fn current() -> Self {
        let (major, minor) = get_spec_version();
        Self { major, minor }
    }

    /// Check if version is at least the given version
    pub fn is_at_least(&self, major: u32, minor: u32) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Check if version supports v1.0 features
    pub fn supports_v1(&self) -> bool {
        self.is_at_least(1, 0)
    }

    /// Check if version supports v2.0 features
    pub fn supports_v2(&self) -> bool {
        self.is_at_least(2, 0)
    }
}

impl core::fmt::Display for SbiVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// ============================================================================
// Implementation Version Parsing
// ============================================================================

/// OpenSBI version structure
#[derive(Debug, Clone, Copy)]
pub struct OpenSbiVersion {
    pub major: u32,
    pub minor: u32,
}

impl OpenSbiVersion {
    /// Parse OpenSBI version
    pub fn from_impl_version(version: u64) -> Self {
        let major = ((version >> 16) & 0xFFFF) as u32;
        let minor = (version & 0xFFFF) as u32;
        Self { major, minor }
    }
}

impl core::fmt::Display for OpenSbiVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// ============================================================================
// Machine ID Parsing
// ============================================================================

/// Known machine vendor IDs
pub mod vendor_id {
    /// SiFive
    pub const SIFIVE: u64 = 0x489;
    /// Andes
    pub const ANDES: u64 = 0x31e;
    /// T-Head
    pub const THEAD: u64 = 0x5b7;
}

/// Known machine architecture IDs
pub mod arch_id {
    /// SiFive U74
    pub const SIFIVE_U74: u64 = 0x8000000000000007;
    /// SiFive S76
    pub const SIFIVE_S76: u64 = 0x8000000000000006;
    /// QEMU virt
    pub const QEMU_VIRT: u64 = 0;
}

/// Get vendor name from ID
pub fn vendor_name(id: u64) -> &'static str {
    match id {
        vendor_id::SIFIVE => "SiFive",
        vendor_id::ANDES => "Andes",
        vendor_id::THEAD => "T-Head",
        0 => "Unknown/QEMU",
        _ => "Unknown",
    }
}
