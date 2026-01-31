//! # SBI Remote Fence Extension
//!
//! Remote fence operations via SBI.

use super::{eid, rfence_fid, SbiError};
use super::base::{sbi_call_2, sbi_call_4, sbi_call_5, SbiRet};

// ============================================================================
// Remote Fence Operations
// ============================================================================

/// Remote FENCE.I on specified harts
///
/// Instructs remote harts to execute FENCE.I.
pub fn remote_fence_i(hart_mask: u64, hart_mask_base: usize) -> Result<(), SbiError> {
    let ret = sbi_call_2(eid::RFENCE, rfence_fid::REMOTE_FENCE_I, hart_mask as usize, hart_mask_base);

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote SFENCE.VMA on specified harts
///
/// Instructs remote harts to execute SFENCE.VMA for the given address range.
/// If size is 0, flushes all entries.
pub fn remote_sfence_vma(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_4(
        eid::RFENCE,
        rfence_fid::REMOTE_SFENCE_VMA,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote SFENCE.VMA with ASID on specified harts
///
/// Instructs remote harts to execute SFENCE.VMA with a specific ASID.
pub fn remote_sfence_vma_asid(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    asid: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_5(
        eid::RFENCE,
        rfence_fid::REMOTE_SFENCE_VMA_ASID,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
        asid,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote HFENCE.GVMA with VMID on specified harts
///
/// For hypervisor extension: flush guest physical to host physical mappings.
pub fn remote_hfence_gvma_vmid(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    vmid: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_5(
        eid::RFENCE,
        rfence_fid::REMOTE_HFENCE_GVMA_VMID,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
        vmid,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote HFENCE.GVMA on specified harts
///
/// For hypervisor extension: flush guest physical to host physical mappings (all VMIDs).
pub fn remote_hfence_gvma(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_4(
        eid::RFENCE,
        rfence_fid::REMOTE_HFENCE_GVMA,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote HFENCE.VVMA with ASID on specified harts
///
/// For hypervisor extension: flush guest virtual address mappings.
pub fn remote_hfence_vvma_asid(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    asid: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_5(
        eid::RFENCE,
        rfence_fid::REMOTE_HFENCE_VVMA_ASID,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
        asid,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Remote HFENCE.VVMA on specified harts
///
/// For hypervisor extension: flush guest virtual address mappings (all ASIDs).
pub fn remote_hfence_vvma(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_4(
        eid::RFENCE,
        rfence_fid::REMOTE_HFENCE_VVMA,
        hart_mask as usize,
        hart_mask_base,
        start_addr,
        size,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Flush instruction cache on all harts
pub fn remote_fence_i_all(max_harts: usize) -> Result<(), SbiError> {
    let mask = if max_harts >= 64 { u64::MAX } else { (1u64 << max_harts) - 1 };
    remote_fence_i(mask, 0)
}

/// Flush TLB on all harts for entire address space
pub fn remote_sfence_vma_all(max_harts: usize) -> Result<(), SbiError> {
    let mask = if max_harts >= 64 { u64::MAX } else { (1u64 << max_harts) - 1 };
    remote_sfence_vma(mask, 0, 0, 0)
}

/// Flush TLB on all harts for a specific range
pub fn remote_sfence_vma_range_all(
    max_harts: usize,
    start_addr: usize,
    size: usize,
) -> Result<(), SbiError> {
    let mask = if max_harts >= 64 { u64::MAX } else { (1u64 << max_harts) - 1 };
    remote_sfence_vma(mask, 0, start_addr, size)
}

/// Flush TLB on all harts for a specific ASID
pub fn remote_sfence_vma_asid_all(max_harts: usize, asid: usize) -> Result<(), SbiError> {
    let mask = if max_harts >= 64 { u64::MAX } else { (1u64 << max_harts) - 1 };
    remote_sfence_vma_asid(mask, 0, 0, 0, asid)
}

/// Flush TLB on a single hart
pub fn remote_sfence_vma_single(hartid: usize) -> Result<(), SbiError> {
    let mask = 1u64 << (hartid % 64);
    let base = (hartid / 64) * 64;
    remote_sfence_vma(mask, base, 0, 0)
}

/// Flush instruction cache on a single hart
pub fn remote_fence_i_single(hartid: usize) -> Result<(), SbiError> {
    let mask = 1u64 << (hartid % 64);
    let base = (hartid / 64) * 64;
    remote_fence_i(mask, base)
}
