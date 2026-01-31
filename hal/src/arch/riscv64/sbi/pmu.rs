//! # SBI Performance Monitoring Unit (PMU) Extension
//!
//! Hardware performance counter management via SBI.

use super::{eid, pmu_fid, SbiError};
use super::base::{sbi_call_0, sbi_call_1, sbi_call_3, sbi_call_4, SbiRet};

// ============================================================================
// PMU Extension
// ============================================================================

/// Get the number of available counters
pub fn num_counters() -> Result<usize, SbiError> {
    let ret = sbi_call_0(eid::PMU, pmu_fid::NUM_COUNTERS);

    if ret.is_success() {
        Ok(ret.value as usize)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterType {
    /// Hardware counter
    Hardware,
    /// Firmware counter
    Firmware,
}

/// Counter information
#[derive(Debug, Clone, Copy)]
pub struct CounterInfo {
    /// Counter type
    pub counter_type: CounterType,
    /// CSR number (for hardware counters)
    pub csr: u16,
    /// Counter width in bits
    pub width: u8,
}

impl CounterInfo {
    /// Parse from raw SBI value
    pub fn from_raw(value: u64) -> Self {
        let counter_type = if (value >> 63) & 1 != 0 {
            CounterType::Firmware
        } else {
            CounterType::Hardware
        };

        let csr = (value & 0xFFF) as u16;
        let width = ((value >> 12) & 0x3F) as u8;

        Self {
            counter_type,
            csr,
            width,
        }
    }
}

/// Get information about a counter
pub fn counter_get_info(counter_idx: usize) -> Result<CounterInfo, SbiError> {
    let ret = sbi_call_1(eid::PMU, pmu_fid::COUNTER_GET_INFO, counter_idx);

    if ret.is_success() {
        Ok(CounterInfo::from_raw(ret.value as u64))
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EventType {
    /// Hardware general event
    HardwareGeneral = 0,
    /// Hardware cache event
    HardwareCache = 1,
    /// Hardware raw event
    HardwareRaw = 2,
    /// Firmware event
    Firmware = 15,
}

/// Hardware general events
pub mod hw_event {
    pub const CPU_CYCLES: u64 = 0x0001_0000;
    pub const INSTRUCTIONS: u64 = 0x0001_0001;
    pub const CACHE_REFERENCES: u64 = 0x0001_0002;
    pub const CACHE_MISSES: u64 = 0x0001_0003;
    pub const BRANCH_INSTRUCTIONS: u64 = 0x0001_0004;
    pub const BRANCH_MISSES: u64 = 0x0001_0005;
    pub const BUS_CYCLES: u64 = 0x0001_0006;
    pub const STALLED_CYCLES_FRONTEND: u64 = 0x0001_0007;
    pub const STALLED_CYCLES_BACKEND: u64 = 0x0001_0008;
    pub const REF_CPU_CYCLES: u64 = 0x0001_0009;
}

/// Counter configuration match
///
/// Finds counters that can track the specified event.
pub fn counter_cfg_match(
    counter_idx_base: usize,
    counter_idx_mask: usize,
    config_flags: usize,
    event_idx: usize,
    event_data: u64,
) -> Result<usize, SbiError> {
    // This requires a 5-argument call which we don't have,
    // so we use inline asm directly
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid::PMU,
            in("a6") pmu_fid::COUNTER_CFG_MATCH,
            in("a0") counter_idx_base,
            in("a1") counter_idx_mask,
            in("a2") config_flags,
            in("a3") event_idx,
            in("a4") event_data as usize,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    if error == 0 {
        Ok(value as usize)
    } else {
        Err(SbiError::from_raw(error))
    }
}

/// Start counters
pub fn counter_start(
    counter_idx_base: usize,
    counter_idx_mask: usize,
    start_flags: usize,
    initial_value: u64,
) -> Result<(), SbiError> {
    let ret = sbi_call_4(
        eid::PMU,
        pmu_fid::COUNTER_START,
        counter_idx_base,
        counter_idx_mask,
        start_flags,
        initial_value as usize,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Start flags
pub mod start_flags {
    /// Set initial value
    pub const SET_INIT_VALUE: usize = 1 << 0;
}

/// Stop counters
pub fn counter_stop(
    counter_idx_base: usize,
    counter_idx_mask: usize,
    stop_flags: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_3(
        eid::PMU,
        pmu_fid::COUNTER_STOP,
        counter_idx_base,
        counter_idx_mask,
        stop_flags,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Stop flags
pub mod stop_flags {
    /// Reset counter to 0
    pub const RESET: usize = 1 << 0;
}

/// Read a firmware counter
pub fn counter_fw_read(counter_idx: usize) -> Result<u64, SbiError> {
    let ret = sbi_call_1(eid::PMU, pmu_fid::COUNTER_FW_READ, counter_idx);

    if ret.is_success() {
        Ok(ret.value as u64)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Read high bits of a firmware counter (for 32-bit)
pub fn counter_fw_read_hi(counter_idx: usize) -> Result<u64, SbiError> {
    let ret = sbi_call_1(eid::PMU, pmu_fid::COUNTER_FW_READ_HI, counter_idx);

    if ret.is_success() {
        Ok(ret.value as u64)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Read a hardware performance counter CSR
#[inline]
pub fn read_hpm_counter(csr: u16) -> u64 {
    // Hardware counters are accessed via CSR
    // For now, we only support the standard ones
    match csr {
        0xC00 => { // cycle
            let val: u64;
            unsafe { core::arch::asm!("rdcycle {}", out(reg) val, options(nomem, nostack)); }
            val
        }
        0xC01 => { // time
            let val: u64;
            unsafe { core::arch::asm!("rdtime {}", out(reg) val, options(nomem, nostack)); }
            val
        }
        0xC02 => { // instret
            let val: u64;
            unsafe { core::arch::asm!("rdinstret {}", out(reg) val, options(nomem, nostack)); }
            val
        }
        _ => 0, // Other HPM counters would need CSR access
    }
}

/// Simple performance measurement
pub struct PerfMeasurement {
    start_cycles: u64,
    start_instret: u64,
}

impl PerfMeasurement {
    /// Start a new measurement
    pub fn start() -> Self {
        Self {
            start_cycles: read_hpm_counter(0xC00),
            start_instret: read_hpm_counter(0xC02),
        }
    }

    /// End the measurement and get results
    pub fn end(self) -> PerfResult {
        let end_cycles = read_hpm_counter(0xC00);
        let end_instret = read_hpm_counter(0xC02);

        PerfResult {
            cycles: end_cycles.saturating_sub(self.start_cycles),
            instructions: end_instret.saturating_sub(self.start_instret),
        }
    }
}

/// Performance measurement result
#[derive(Debug, Clone, Copy)]
pub struct PerfResult {
    /// CPU cycles elapsed
    pub cycles: u64,
    /// Instructions retired
    pub instructions: u64,
}

impl PerfResult {
    /// Calculate instructions per cycle
    pub fn ipc(&self) -> f64 {
        if self.cycles == 0 {
            0.0
        } else {
            self.instructions as f64 / self.cycles as f64
        }
    }
}
