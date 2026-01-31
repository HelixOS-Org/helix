//! # SBI IPI Extension
//!
//! Inter-Processor Interrupt support via SBI.

use super::{eid, SbiError};
use super::base::{sbi_call_2, SbiRet};

// ============================================================================
// IPI Extension
// ============================================================================

/// Send IPI to a set of harts
///
/// # Arguments
/// * `hart_mask` - Bitmask of harts to send IPI to
/// * `hart_mask_base` - Base hart ID for the mask
///
/// Each bit in hart_mask represents a hart with ID (hart_mask_base + bit_position).
pub fn send_ipi(hart_mask: u64, hart_mask_base: usize) -> Result<(), SbiError> {
    let ret = sbi_call_2(eid::IPI, 0, hart_mask as usize, hart_mask_base);

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Send IPI to a single hart
pub fn send_ipi_single(hartid: usize) -> Result<(), SbiError> {
    let mask = 1u64 << (hartid % 64);
    let base = (hartid / 64) * 64;
    send_ipi(mask, base)
}

/// Send IPI to all harts except self
pub fn send_ipi_all_except(self_hartid: usize, max_harts: usize) -> Result<(), SbiError> {
    let mut mask = 0u64;

    for i in 0..max_harts.min(64) {
        if i != self_hartid {
            mask |= 1 << i;
        }
    }

    if mask != 0 {
        send_ipi(mask, 0)
    } else {
        Ok(())
    }
}

/// Send IPI to all harts
pub fn send_ipi_all(max_harts: usize) -> Result<(), SbiError> {
    let mask = if max_harts >= 64 {
        u64::MAX
    } else {
        (1u64 << max_harts) - 1
    };

    send_ipi(mask, 0)
}
