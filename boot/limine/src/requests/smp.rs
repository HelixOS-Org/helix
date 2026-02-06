//! # SMP (Symmetric Multi-Processing) Request
//!
//! This module provides SMP support for multi-core initialization.
//! It handles CPU enumeration and startup for all supported architectures.

use core::sync::atomic::{AtomicU64, Ordering};

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::raw::RawSmpInfo;
use crate::protocol::request_ids::SMP_ID;

/// SMP request flags
pub mod smp_flags {
    /// Request x2APIC mode (`x86_64` only)
    pub const X2APIC: u64 = 1 << 0;
}

/// SMP response flags
pub mod smp_response_flags {
    /// x2APIC is enabled
    pub const X2APIC_ENABLED: u64 = 1 << 0;
}

/// SMP (Symmetric Multi-Processing) request
///
/// This request provides information about all CPUs in the system and
/// allows starting additional processor cores.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::{smp_flags, SmpRequest};
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static SMP: SmpRequest = SmpRequest::new().with_flags(smp_flags::X2APIC);
///
/// extern "C" fn ap_entry() -> ! {
///     // Application Processor entry point
///     loop {}
/// }
///
/// fn start_all_cpus() {
///     if let Some(smp) = SMP.response() {
///         for cpu in smp.cpus().filter(|c| !c.is_bsp()) {
///             cpu.start(ap_entry as u64, 0);
///         }
///     }
/// }
/// ```
#[repr(C)]
pub struct SmpRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<SmpResponse>,
    /// SMP flags
    flags: u64,
}

impl SmpRequest {
    /// Create a new SMP request
    pub const fn new() -> Self {
        Self {
            id: SMP_ID,
            revision: 0,
            response: ResponsePtr::null(),
            flags: 0,
        }
    }

    /// Create with specific flags
    #[must_use]
    pub const fn with_flags(mut self, flags: u64) -> Self {
        self.flags = flags;
        self
    }

    /// Enable `x2APIC` mode (`x86_64`)
    #[must_use]
    pub const fn enable_x2apic(mut self) -> Self {
        self.flags |= smp_flags::X2APIC;
        self
    }
}

impl Default for SmpRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for SmpRequest {
    type Response = SmpResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for SmpRequest {}

/// SMP response
#[repr(C)]
pub struct SmpResponse {
    /// Response revision
    revision: u64,
    /// Response flags
    flags: u64,
    /// BSP LAPIC ID (`x86_64`) or MPIDR (`AArch64`) or hart ID (`RISC-V`)
    bsp_lapic_id: u64,
    /// Number of CPUs
    cpu_count: u64,
    /// CPU info pointers
    cpus: *const *const RawSmpInfo,
}

impl SmpResponse {
    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the response flags
    pub fn flags(&self) -> u64 {
        self.flags
    }

    /// Check if `x2APIC` is enabled (`x86_64`)
    pub fn x2apic_enabled(&self) -> bool {
        self.flags & smp_response_flags::X2APIC_ENABLED != 0
    }

    /// Get the BSP (Bootstrap Processor) LAPIC ID
    pub fn bsp_lapic_id(&self) -> u64 {
        self.bsp_lapic_id
    }

    /// Get the number of CPUs
    pub fn cpu_count(&self) -> usize {
        self.cpu_count as usize
    }

    /// Iterate over all CPUs
    pub fn cpus(&self) -> CpuIterator<'_> {
        CpuIterator {
            response: self,
            index: 0,
        }
    }

    /// Get a specific CPU by index
    pub fn get_cpu(&self, index: usize) -> Option<CpuInfo<'_>> {
        if index >= self.cpu_count() || self.cpus.is_null() {
            return None;
        }

        unsafe {
            let cpu_ptr = *self.cpus.add(index);
            if cpu_ptr.is_null() {
                None
            } else {
                Some(CpuInfo::new(&*cpu_ptr, self.bsp_lapic_id))
            }
        }
    }

    /// Get the BSP CPU info
    pub fn bsp(&self) -> Option<CpuInfo<'_>> {
        self.cpus().find(CpuInfo::is_bsp)
    }

    /// Get all APs (Application Processors)
    pub fn aps(&self) -> impl Iterator<Item = CpuInfo<'_>> {
        self.cpus().filter(|c| !c.is_bsp())
    }

    /// Get the number of application processors
    pub fn ap_count(&self) -> usize {
        self.cpu_count().saturating_sub(1)
    }

    /// Find a CPU by LAPIC ID
    pub fn find_by_lapic_id(&self, lapic_id: u32) -> Option<CpuInfo<'_>> {
        self.cpus().find(|c| c.lapic_id() == lapic_id)
    }
}

unsafe impl SafeResponse for SmpResponse {
    fn validate(&self) -> bool {
        self.cpu_count > 0 && !self.cpus.is_null()
    }
}

impl core::fmt::Debug for SmpResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SmpResponse")
            .field("cpu_count", &self.cpu_count())
            .field("bsp_lapic_id", &self.bsp_lapic_id)
            .field("x2apic_enabled", &self.x2apic_enabled())
            .finish()
    }
}

/// Iterator over CPUs
pub struct CpuIterator<'a> {
    response: &'a SmpResponse,
    index: usize,
}

impl<'a> Iterator for CpuIterator<'a> {
    type Item = CpuInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let cpu = self.response.get_cpu(self.index)?;
        self.index += 1;
        Some(cpu)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.response.cpu_count() - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for CpuIterator<'_> {}

/// CPU information (`x86_64`)
pub struct CpuInfo<'a> {
    raw: &'a RawSmpInfo,
    bsp_lapic_id: u64,
}

impl<'a> CpuInfo<'a> {
    fn new(raw: &'a RawSmpInfo, bsp_lapic_id: u64) -> Self {
        Self { raw, bsp_lapic_id }
    }

    /// Get the ACPI processor ID
    pub fn processor_id(&self) -> u32 {
        self.raw.processor_id
    }

    /// Get the LAPIC ID
    pub fn lapic_id(&self) -> u32 {
        self.raw.lapic_id
    }

    /// Check if this is the BSP (Bootstrap Processor)
    pub fn is_bsp(&self) -> bool {
        u64::from(self.raw.lapic_id) == self.bsp_lapic_id
    }

    /// Check if this CPU has been started
    pub fn is_started(&self) -> bool {
        self.raw.goto_address.load(Ordering::Acquire) != 0
    }

    /// Get the extra argument that will be passed to the CPU
    pub fn extra_argument(&self) -> u64 {
        self.raw.extra_argument
    }

    /// Start this CPU at the given entry point
    ///
    /// The entry point function will receive the extra argument in a register.
    /// On `x86_64`, it's passed in RDI.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry point address (must be a valid function pointer)
    /// * `arg` - An extra argument to pass to the entry point
    ///
    /// # Safety
    ///
    /// This function is safe to call, but the entry point function must be
    /// carefully written to handle the early CPU state.
    pub fn start(&self, entry: u64, arg: u64) {
        // Set the extra argument first
        // Safety: The raw structure is mutable from the bootloader's perspective
        let raw_ptr = self.raw as *const RawSmpInfo as *mut RawSmpInfo;
        unsafe {
            (*raw_ptr).extra_argument = arg;
        }

        // Then set the goto address to start the CPU
        self.raw.goto_address.store(entry, Ordering::Release);
    }
}

impl core::fmt::Debug for CpuInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CpuInfo")
            .field("processor_id", &self.processor_id())
            .field("lapic_id", &self.lapic_id())
            .field("is_bsp", &self.is_bsp())
            .field("is_started", &self.is_started())
            .finish()
    }
}

// =============================================================================
// AArch64-specific CPU info
// =============================================================================

/// CPU information (AArch64)
#[cfg(target_arch = "aarch64")]
pub struct CpuInfoAarch64<'a> {
    raw: &'a RawSmpInfoAarch64,
    bsp_mpidr: u64,
}

#[cfg(target_arch = "aarch64")]
impl<'a> CpuInfoAarch64<'a> {
    /// Get the processor ID
    pub fn processor_id(&self) -> u32 {
        self.raw.processor_id
    }

    /// Get the GIC CPU interface number
    pub fn gic_interface_number(&self) -> u32 {
        self.raw.gic_iface_no
    }

    /// Get the MPIDR
    pub fn mpidr(&self) -> u64 {
        self.raw.mpidr
    }

    /// Check if this is the BSP
    pub fn is_bsp(&self) -> bool {
        self.raw.mpidr == self.bsp_mpidr
    }

    /// Start this CPU
    pub fn start(&self, entry: u64, arg: u64) {
        let raw_ptr = self.raw as *const RawSmpInfoAarch64 as *mut RawSmpInfoAarch64;
        unsafe {
            (*raw_ptr).extra_argument = arg;
        }
        self.raw.goto_address.store(entry, Ordering::Release);
    }
}

// =============================================================================
// RISC-V-specific CPU info
// =============================================================================

/// CPU information (RISC-V)
#[cfg(target_arch = "riscv64")]
pub struct CpuInfoRiscv<'a> {
    raw: &'a RawSmpInfoRiscv,
    bsp_hartid: u64,
}

#[cfg(target_arch = "riscv64")]
impl<'a> CpuInfoRiscv<'a> {
    /// Get the processor ID
    pub fn processor_id(&self) -> u64 {
        self.raw.processor_id
    }

    /// Get the hart ID
    pub fn hart_id(&self) -> u64 {
        self.raw.hartid
    }

    /// Check if this is the BSP
    pub fn is_bsp(&self) -> bool {
        self.raw.hartid == self.bsp_hartid
    }

    /// Start this CPU
    pub fn start(&self, entry: u64, arg: u64) {
        let raw_ptr = self.raw as *const RawSmpInfoRiscv as *mut RawSmpInfoRiscv;
        unsafe {
            (*raw_ptr).extra_argument = arg;
        }
        self.raw.goto_address.store(entry, Ordering::Release);
    }
}

// =============================================================================
// CPU Startup Helpers
// =============================================================================

/// Entry point type for AP startup
pub type ApEntryPoint = extern "C" fn(smp_info: &RawSmpInfo) -> !;

/// Macro to create an AP entry point
#[macro_export]
macro_rules! ap_entry {
    ($name:ident, $body:block) => {
        extern "C" fn $name(_smp_info: &$crate::protocol::raw::RawSmpInfo) -> ! {
            $body
        }
    };
}

/// Wait for all APs to reach a barrier
///
/// This is a simple spin-wait barrier for AP synchronization.
pub struct ApBarrier {
    count: AtomicU64,
    target: u64,
}

impl ApBarrier {
    /// Create a new barrier for the given number of CPUs
    pub const fn new(cpu_count: u64) -> Self {
        Self {
            count: AtomicU64::new(0),
            target: cpu_count,
        }
    }

    /// Wait at the barrier
    pub fn wait(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
        while self.count.load(Ordering::Acquire) < self.target {
            core::hint::spin_loop();
        }
    }

    /// Reset the barrier
    pub fn reset(&self) {
        self.count.store(0, Ordering::Release);
    }

    /// Get the current count
    pub fn current(&self) -> u64 {
        self.count.load(Ordering::Acquire)
    }

    /// Check if all CPUs have reached the barrier
    pub fn is_complete(&self) -> bool {
        self.count.load(Ordering::Acquire) >= self.target
    }
}

// Safety: Barrier is designed for concurrent access
unsafe impl Sync for ApBarrier {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier() {
        let barrier = ApBarrier::new(1);
        assert!(!barrier.is_complete());
        barrier.wait();
        assert!(barrier.is_complete());
    }
}
