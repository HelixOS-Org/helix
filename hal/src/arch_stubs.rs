//! # Architecture Selection
//!
//! This module provides compile-time architecture selection.
//!
//! NOTE: Architecture-specific HAL implementations are provided inline
//! until the separate arch crates are implemented.

use alloc::vec::Vec;

use crate::cpu::{CpuAbstraction, CpuContext};
use crate::firmware::{BootTime, FirmwareInterface, FirmwareType, FramebufferInfo};
use crate::interrupts::{
    InterruptController, InterruptPriority, InterruptVector, IpiTarget, TriggerMode,
};
use crate::mmu::{MemoryRegion, MmuAbstraction, PageFlags, PageTable};
use crate::{HalError, HalResult, HardwareAbstractionLayer, PageSize, PhysAddr, VirtAddr};

/// Architecture name constant
pub const ARCH_NAME: &str = {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(target_arch = "riscv64")]
    {
        "riscv64"
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        "unknown"
    }
};

/// Pointer width in bits
pub const POINTER_WIDTH: usize = core::mem::size_of::<usize>() * 8;

// =============================================================================
// Stub HAL Implementations (until arch-specific crates are created)
// =============================================================================

/// Stub HAL for compilation - to be replaced by arch-specific implementations
pub struct StubHal {
    cpu: StubCpu,
    mmu: StubMmu,
    interrupts: StubInterruptController,
    firmware: StubFirmware,
}

impl StubHal {
    /// Create a new stub HAL instance
    pub const fn new() -> Self {
        Self {
            cpu: StubCpu::new(),
            mmu: StubMmu::new(),
            interrupts: StubInterruptController::new(),
            firmware: StubFirmware::new(),
        }
    }
}

impl Default for StubHal {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: StubHal contains no mutable shared state
unsafe impl Send for StubHal {}
unsafe impl Sync for StubHal {}

impl HardwareAbstractionLayer for StubHal {
    type Cpu = StubCpu;
    type Mmu = StubMmu;
    type InterruptController = StubInterruptController;
    type Firmware = StubFirmware;

    fn cpu(&self) -> &Self::Cpu {
        &self.cpu
    }

    fn mmu(&self) -> &Self::Mmu {
        &self.mmu
    }

    fn interrupt_controller(&self) -> &Self::InterruptController {
        &self.interrupts
    }

    fn firmware(&self) -> &Self::Firmware {
        &self.firmware
    }

    fn arch_name(&self) -> &'static str {
        ARCH_NAME
    }

    fn arch_version(&self) -> &'static str {
        "stub-1.0"
    }

    fn early_init(&mut self) -> HalResult<()> {
        Ok(())
    }

    fn init(&mut self) -> HalResult<()> {
        Ok(())
    }

    fn halt(&self) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }

    fn reboot(&self) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }

    fn shutdown(&self) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
}

// =============================================================================
// Stub CPU Implementation
// =============================================================================

/// Stub CPU context for context switching
#[derive(Clone, Default)]
pub struct StubCpuContext {
    ip: u64,
    sp: u64,
    ret: u64,
    args: [u64; 6],
}

// SAFETY: StubCpuContext is plain data
unsafe impl Send for StubCpuContext {}
unsafe impl Sync for StubCpuContext {}

impl CpuContext for StubCpuContext {
    fn new_kernel(entry: VirtAddr, stack: VirtAddr) -> Self {
        Self {
            ip: entry.as_u64(),
            sp: stack.as_u64(),
            ret: 0,
            args: [0; 6],
        }
    }

    fn new_user(entry: VirtAddr, stack: VirtAddr) -> Self {
        Self::new_kernel(entry, stack)
    }

    fn instruction_pointer(&self) -> VirtAddr {
        VirtAddr::new(self.ip)
    }

    fn set_instruction_pointer(&mut self, ip: VirtAddr) {
        self.ip = ip.as_u64();
    }

    fn stack_pointer(&self) -> VirtAddr {
        VirtAddr::new(self.sp)
    }

    fn set_stack_pointer(&mut self, sp: VirtAddr) {
        self.sp = sp.as_u64();
    }

    fn return_value(&self) -> u64 {
        self.ret
    }

    fn set_return_value(&mut self, value: u64) {
        self.ret = value;
    }

    fn syscall_arg(&self, index: usize) -> u64 {
        self.args.get(index).copied().unwrap_or(0)
    }

    fn set_syscall_arg(&mut self, index: usize, value: u64) {
        if index < 6 {
            self.args[index] = value;
        }
    }
}

/// Stub CPU implementation
pub struct StubCpu;

impl Default for StubCpu {
    fn default() -> Self {
        Self::new()
    }
}

impl StubCpu {
    /// Create a new stub CPU
    pub const fn new() -> Self {
        Self
    }
}

// SAFETY: StubCpu contains no mutable state
unsafe impl Send for StubCpu {}
unsafe impl Sync for StubCpu {}

impl CpuAbstraction for StubCpu {
    type Context = StubCpuContext;
    type CpuId = usize;

    fn current_cpu_id(&self) -> Self::CpuId {
        0
    }

    fn cpu_count(&self) -> usize {
        1
    }

    fn is_bsp(&self) -> bool {
        true
    }

    unsafe fn enable_interrupts(&self) {
        // Stub: no-op
    }

    unsafe fn disable_interrupts(&self) {
        // Stub: no-op
    }

    fn interrupts_enabled(&self) -> bool {
        false
    }

    fn without_interrupts<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }

    fn halt(&self) {
        core::hint::spin_loop();
    }

    fn pause(&self) {
        core::hint::spin_loop();
    }

    fn memory_barrier(&self) {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }

    fn read_barrier(&self) {
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
    }

    fn write_barrier(&self) {
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
    }

    fn invalidate_icache(&self) {
        // Stub: no-op
    }

    fn stack_pointer(&self) -> VirtAddr {
        VirtAddr::new(0)
    }

    fn instruction_pointer(&self) -> VirtAddr {
        VirtAddr::new(0)
    }

    fn read_register(&self, _name: &str) -> HalResult<u64> {
        Err(HalError::NotSupported)
    }

    unsafe fn write_register(&self, _name: &str, _value: u64) -> HalResult<()> {
        Err(HalError::NotSupported)
    }
}

// =============================================================================
// Stub MMU Implementation
// =============================================================================

/// Stub page table
pub struct StubPageTable;

// SAFETY: StubPageTable contains no mutable state
unsafe impl Send for StubPageTable {}
unsafe impl Sync for StubPageTable {}

impl PageTable for StubPageTable {
    unsafe fn map(
        &mut self,
        _virt: VirtAddr,
        _phys: PhysAddr,
        _size: PageSize,
        _flags: PageFlags,
    ) -> HalResult<()> {
        Ok(())
    }

    unsafe fn map_range(
        &mut self,
        _virt_start: VirtAddr,
        _phys_start: PhysAddr,
        _size: usize,
        _page_size: PageSize,
        _flags: PageFlags,
    ) -> HalResult<()> {
        Ok(())
    }

    fn unmap(&mut self, _virt: VirtAddr, _size: PageSize) -> HalResult<PhysAddr> {
        Err(HalError::NotSupported)
    }

    fn unmap_range(
        &mut self,
        _virt_start: VirtAddr,
        _size: usize,
        _page_size: PageSize,
    ) -> HalResult<()> {
        Ok(())
    }

    fn update_flags(
        &mut self,
        _virt: VirtAddr,
        _size: PageSize,
        _flags: PageFlags,
    ) -> HalResult<()> {
        Ok(())
    }

    fn query(&self, _virt: VirtAddr) -> Option<(PhysAddr, PageSize, PageFlags)> {
        None
    }

    fn root_physical_address(&self) -> PhysAddr {
        PhysAddr::new(0)
    }

    fn clone_table(&self) -> HalResult<Self> {
        Ok(StubPageTable)
    }
}

/// Stub MMU implementation
pub struct StubMmu {
    page_table: StubPageTable,
}

impl Default for StubMmu {
    fn default() -> Self {
        Self::new()
    }
}

impl StubMmu {
    /// Create a new stub MMU
    pub const fn new() -> Self {
        Self {
            page_table: StubPageTable,
        }
    }
}

// SAFETY: StubMmu contains no mutable shared state
unsafe impl Send for StubMmu {}
unsafe impl Sync for StubMmu {}

impl MmuAbstraction for StubMmu {
    type PageTable = StubPageTable;
    type Asid = u16;

    fn supported_page_sizes(&self) -> &[PageSize] {
        &[PageSize::Size4KiB]
    }

    fn default_page_size(&self) -> PageSize {
        PageSize::Size4KiB
    }

    fn max_virtual_address(&self) -> VirtAddr {
        VirtAddr::new(0xFFFF_FFFF_FFFF_FFFF)
    }

    fn max_physical_address(&self) -> PhysAddr {
        PhysAddr::new(0xFFFF_FFFF_FFFF)
    }

    fn max_asid(&self) -> usize {
        65535
    }

    fn allocate_asid(&self) -> HalResult<Self::Asid> {
        Ok(0)
    }

    fn free_asid(&self, _asid: Self::Asid) {
        // Stub: no-op
    }

    fn create_page_table(&self) -> HalResult<Self::PageTable> {
        Ok(StubPageTable)
    }

    fn kernel_page_table(&self) -> &Self::PageTable {
        &self.page_table
    }

    fn current_page_table(&self) -> &Self::PageTable {
        &self.page_table
    }

    unsafe fn switch_page_table(&self, _table: &Self::PageTable, _asid: Self::Asid) {
        // Stub: no-op
    }

    fn translate(&self, _table: &Self::PageTable, _virt: VirtAddr) -> Option<PhysAddr> {
        None
    }

    fn invalidate_tlb(&self, _virt: VirtAddr) {
        // Stub: no-op
    }

    fn invalidate_tlb_all(&self) {
        // Stub: no-op
    }

    fn invalidate_tlb_asid(&self, _asid: Self::Asid) {
        // Stub: no-op
    }

    fn invalidate_tlb_broadcast(&self, _virt: VirtAddr) {
        // Stub: no-op
    }
}

// =============================================================================
// Stub Interrupt Controller
// =============================================================================

/// Stub Interrupt Controller
pub struct StubInterruptController;

impl Default for StubInterruptController {
    fn default() -> Self {
        Self::new()
    }
}

impl StubInterruptController {
    /// Create a new stub interrupt controller
    pub const fn new() -> Self {
        Self
    }
}

// SAFETY: StubInterruptController contains no mutable state
unsafe impl Send for StubInterruptController {}
unsafe impl Sync for StubInterruptController {}

impl InterruptController for StubInterruptController {
    fn init(&mut self) -> HalResult<()> {
        Ok(())
    }

    fn enable(&mut self) {
        // Stub: no-op
    }

    fn disable(&mut self) {
        // Stub: no-op
    }

    fn interrupt_count(&self) -> usize {
        256
    }

    fn enable_interrupt(&mut self, _vector: InterruptVector) -> HalResult<()> {
        Ok(())
    }

    fn disable_interrupt(&mut self, _vector: InterruptVector) -> HalResult<()> {
        Ok(())
    }

    fn is_interrupt_enabled(&self, _vector: InterruptVector) -> bool {
        false
    }

    fn set_priority(
        &mut self,
        _vector: InterruptVector,
        _priority: InterruptPriority,
    ) -> HalResult<()> {
        Ok(())
    }

    fn get_priority(&self, _vector: InterruptVector) -> HalResult<InterruptPriority> {
        Ok(0)
    }

    fn set_priority_threshold(&mut self, _threshold: InterruptPriority) {
        // Stub: no-op
    }

    fn acknowledge(&mut self, _vector: InterruptVector) {
        // Stub: no-op
    }

    fn send_ipi(&mut self, _target: IpiTarget, _vector: InterruptVector) -> HalResult<()> {
        Ok(())
    }

    fn pending_interrupt(&self) -> Option<InterruptVector> {
        None
    }

    fn is_interrupt_pending(&self, _vector: InterruptVector) -> bool {
        false
    }

    fn clear_pending(&mut self, _vector: InterruptVector) {
        // Stub: no-op
    }

    fn set_trigger_mode(&mut self, _vector: InterruptVector, _mode: TriggerMode) -> HalResult<()> {
        Ok(())
    }
}

// =============================================================================
// Stub Firmware Interface
// =============================================================================

/// Stub Firmware Interface
pub struct StubFirmware;

impl Default for StubFirmware {
    fn default() -> Self {
        Self::new()
    }
}

impl StubFirmware {
    /// Create a new stub firmware interface
    pub const fn new() -> Self {
        Self
    }
}

// SAFETY: StubFirmware contains no mutable state
unsafe impl Send for StubFirmware {}
unsafe impl Sync for StubFirmware {}

impl FirmwareInterface for StubFirmware {
    fn firmware_type(&self) -> FirmwareType {
        FirmwareType::Unknown
    }

    fn firmware_version(&self) -> Option<&str> {
        Some("stub-1.0")
    }

    fn memory_map(&self) -> Vec<MemoryRegion> {
        Vec::new()
    }

    fn acpi_rsdp(&self) -> Option<PhysAddr> {
        None
    }

    fn device_tree_blob(&self) -> Option<&[u8]> {
        None
    }

    fn command_line(&self) -> Option<&str> {
        None
    }

    fn framebuffer(&self) -> Option<FramebufferInfo> {
        None
    }

    fn boot_time(&self) -> Option<BootTime> {
        None
    }

    fn request_reboot(&self) -> HalResult<()> {
        Err(HalError::NotSupported)
    }

    fn request_shutdown(&self) -> HalResult<()> {
        Err(HalError::NotSupported)
    }

    fn efi_runtime_services(&self) -> Option<PhysAddr> {
        None
    }
}

// =============================================================================
// Type Aliases and Global HAL
// =============================================================================

/// Current architecture HAL type alias
pub type CurrentHal = StubHal;

/// Global HAL instance using thread-safe initialization
///
/// Uses `spin::Once` for safe one-time initialization without data races.
static HAL: spin::Once<StubHal> = spin::Once::new();

/// Initialize the global HAL instance
///
/// # Safety
/// This must be called exactly once during early boot, before any other HAL usage.
/// Multiple calls are safe (will be no-ops) but the first call must happen
/// in a single-threaded context before SMP is enabled.
pub unsafe fn init_hal() -> HalResult<()> {
    let mut result = Ok(());

    HAL.call_once(|| {
        let mut hal = StubHal::new();
        // Note: early_init and init are safe methods, we're in an unsafe fn
        // because the caller must ensure single-threaded boot context
        if let Err(e) = hal.early_init() {
            result = Err(e);
        }
        if result.is_ok() {
            if let Err(e) = hal.init() {
                result = Err(e);
            }
        }
        hal
    });

    result
}

/// Get a reference to the global HAL
///
/// Returns `None` if HAL hasn't been initialized yet.
pub fn try_hal() -> Option<&'static StubHal> {
    HAL.get()
}

/// Get a reference to the global HAL
///
/// # Panics
/// Panics if HAL hasn't been initialized. For kernel code that runs after
/// initialization, prefer using `hal()`. For code that may run before
/// initialization, use `try_hal()`.
pub fn hal() -> &'static StubHal {
    HAL.get()
        .expect("HAL not initialized - call init_hal() first during early boot")
}
