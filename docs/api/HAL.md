# Helix HAL API Reference

<div align="center">

🔌 **Hardware Abstraction Layer API Documentation**

*CPU, MMU, Interrupts, Firmware, and Architecture Abstractions*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [CPU Abstraction](#2-cpu-abstraction)
3. [MMU Abstraction](#3-mmu-abstraction)
4. [Interrupt Abstraction](#4-interrupt-abstraction)
5. [Firmware Interface](#5-firmware-interface)
6. [Architecture Support](#6-architecture-support)
7. [Port I/O](#7-port-io)
8. [Timing](#8-timing)

---

## 1. Overview

### 1.1 HAL Purpose

The Hardware Abstraction Layer (HAL) provides a unified interface to hardware features across different architectures. This allows the kernel core to be architecture-independent.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          HAL ARCHITECTURE                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Kernel Core / Subsystems                                                   │
│  ════════════════════════                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  helix-core    helix-memory    helix-execution    helix-modules    │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  HAL Interface                                                              │
│  ═════════════                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         helix-hal                                   │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │   │
│  │  │   CPU   │  │   MMU   │  │  IRQ    │  │Firmware │  │  Timer  │  │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  │   │
│  └───────┼────────────┼────────────┼────────────┼────────────┼───────┘   │
│          │            │            │            │            │            │
│          └────────────┴────────────┴────────────┴────────────┘            │
│                                   │                                         │
│                                   ▼                                         │
│  Architecture Implementation                                                │
│  ═══════════════════════════                                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │   │
│  │  │    x86_64     │  │    aarch64    │  │    riscv64    │           │   │
│  │  │  (default)    │  │   (planned)   │  │   (planned)   │           │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Crate Structure

```
helix-hal/
├── Cargo.toml
└── src/
    ├── lib.rs              # Crate root
    ├── cpu.rs              # CPU abstraction
    ├── mmu.rs              # Memory management unit
    ├── interrupts.rs       # Interrupt handling
    ├── firmware.rs         # BIOS/UEFI interface
    ├── arch_stubs.rs       # Architecture stubs
    └── arch/
        ├── mod.rs          # Architecture selection
        ├── x86_64/         # x86_64 implementation
        │   ├── mod.rs
        │   ├── cpu.rs
        │   ├── gdt.rs
        │   ├── idt.rs
        │   ├── apic.rs
        │   ├── paging.rs
        │   └── io.rs
        ├── aarch64/        # ARM64 (planned)
        └── riscv64/        # RISC-V (planned)
```

---

## 2. CPU Abstraction

### 2.1 CPU Trait

```rust
/// CPU abstraction trait
pub trait Cpu: Send + Sync {
    /// Get current CPU ID
    fn current_id(&self) -> CpuId;
    
    /// Get number of CPUs
    fn cpu_count(&self) -> usize;
    
    /// Halt the CPU until next interrupt
    fn halt(&self);
    
    /// Disable interrupts
    fn disable_interrupts(&self);
    
    /// Enable interrupts
    fn enable_interrupts(&self);
    
    /// Check if interrupts are enabled
    fn interrupts_enabled(&self) -> bool;
    
    /// Execute with interrupts disabled
    fn without_interrupts<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;
    
    /// Read CPU timestamp counter
    fn read_tsc(&self) -> u64;
    
    /// Get CPU features
    fn features(&self) -> CpuFeatures;
    
    /// Get CPU vendor string
    fn vendor(&self) -> &'static str;
    
    /// Get CPU model name
    fn model_name(&self) -> &'static str;
}

/// CPU identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuId(pub u32);

/// CPU features
bitflags! {
    pub struct CpuFeatures: u64 {
        /// Floating point unit
        const FPU = 1 << 0;
        /// SSE support
        const SSE = 1 << 1;
        /// SSE2 support
        const SSE2 = 1 << 2;
        /// SSE3 support
        const SSE3 = 1 << 3;
        /// SSE4.1 support
        const SSE4_1 = 1 << 4;
        /// SSE4.2 support
        const SSE4_2 = 1 << 5;
        /// AVX support
        const AVX = 1 << 6;
        /// AVX2 support
        const AVX2 = 1 << 7;
        /// AVX-512 support
        const AVX512 = 1 << 8;
        /// APIC support
        const APIC = 1 << 16;
        /// x2APIC support
        const X2APIC = 1 << 17;
        /// TSC support
        const TSC = 1 << 18;
        /// Invariant TSC
        const TSC_INVARIANT = 1 << 19;
        /// RDRAND instruction
        const RDRAND = 1 << 24;
        /// RDSEED instruction
        const RDSEED = 1 << 25;
        /// NX bit support
        const NX = 1 << 32;
        /// Page1GB support
        const PAGE_1GB = 1 << 33;
        /// PCID support
        const PCID = 1 << 34;
        /// SMAP support
        const SMAP = 1 << 35;
        /// SMEP support
        const SMEP = 1 << 36;
    }
}
```

### 2.2 CPU Implementation (x86_64)

```rust
/// x86_64 CPU implementation
pub struct X86_64Cpu {
    /// CPUID information
    cpuid: CpuidInfo,
    
    /// Number of CPUs
    cpu_count: u32,
}

impl X86_64Cpu {
    /// Create new CPU abstraction
    pub fn new() -> Self {
        let cpuid = CpuidInfo::detect();
        
        Self {
            cpuid,
            cpu_count: 1, // Updated by ACPI/MP table parsing
        }
    }
    
    /// Detect CPU features via CPUID
    fn detect_features(&self) -> CpuFeatures {
        let mut features = CpuFeatures::empty();
        
        // CPUID leaf 1
        let (_, _, ecx, edx) = cpuid(1, 0);
        
        if edx & (1 << 0) != 0 { features |= CpuFeatures::FPU; }
        if edx & (1 << 4) != 0 { features |= CpuFeatures::TSC; }
        if edx & (1 << 9) != 0 { features |= CpuFeatures::APIC; }
        if edx & (1 << 25) != 0 { features |= CpuFeatures::SSE; }
        if edx & (1 << 26) != 0 { features |= CpuFeatures::SSE2; }
        
        if ecx & (1 << 0) != 0 { features |= CpuFeatures::SSE3; }
        if ecx & (1 << 19) != 0 { features |= CpuFeatures::SSE4_1; }
        if ecx & (1 << 20) != 0 { features |= CpuFeatures::SSE4_2; }
        if ecx & (1 << 21) != 0 { features |= CpuFeatures::X2APIC; }
        if ecx & (1 << 28) != 0 { features |= CpuFeatures::AVX; }
        if ecx & (1 << 30) != 0 { features |= CpuFeatures::RDRAND; }
        
        // CPUID leaf 7
        let (_, ebx, ecx, _) = cpuid(7, 0);
        
        if ebx & (1 << 5) != 0 { features |= CpuFeatures::AVX2; }
        if ebx & (1 << 7) != 0 { features |= CpuFeatures::SMEP; }
        if ebx & (1 << 18) != 0 { features |= CpuFeatures::RDSEED; }
        if ebx & (1 << 20) != 0 { features |= CpuFeatures::SMAP; }
        if ebx & (1 << 16) != 0 { features |= CpuFeatures::AVX512; }
        
        // Extended CPUID
        let (_, _, _, edx) = cpuid(0x8000_0001, 0);
        
        if edx & (1 << 20) != 0 { features |= CpuFeatures::NX; }
        if edx & (1 << 26) != 0 { features |= CpuFeatures::PAGE_1GB; }
        
        features
    }
}

impl Cpu for X86_64Cpu {
    fn current_id(&self) -> CpuId {
        // Read APIC ID
        let id = unsafe { read_lapic_id() };
        CpuId(id)
    }
    
    fn cpu_count(&self) -> usize {
        self.cpu_count as usize
    }
    
    fn halt(&self) {
        unsafe { core::arch::asm!("hlt") };
    }
    
    fn disable_interrupts(&self) {
        unsafe { core::arch::asm!("cli") };
    }
    
    fn enable_interrupts(&self) {
        unsafe { core::arch::asm!("sti") };
    }
    
    fn interrupts_enabled(&self) -> bool {
        let rflags: u64;
        unsafe { core::arch::asm!("pushfq; pop {}", out(reg) rflags) };
        rflags & 0x200 != 0
    }
    
    fn without_interrupts<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let was_enabled = self.interrupts_enabled();
        self.disable_interrupts();
        
        let result = f();
        
        if was_enabled {
            self.enable_interrupts();
        }
        
        result
    }
    
    fn read_tsc(&self) -> u64 {
        let lo: u32;
        let hi: u32;
        unsafe {
            core::arch::asm!(
                "rdtsc",
                out("eax") lo,
                out("edx") hi,
            );
        }
        ((hi as u64) << 32) | (lo as u64)
    }
    
    fn features(&self) -> CpuFeatures {
        self.detect_features()
    }
    
    fn vendor(&self) -> &'static str {
        self.cpuid.vendor()
    }
    
    fn model_name(&self) -> &'static str {
        self.cpuid.model_name()
    }
}

/// Execute CPUID instruction
fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("ebx") ebx,
            out("edx") edx,
        );
    }
    
    (eax, ebx, ecx, edx)
}
```

### 2.3 Control Registers

```rust
/// Control register operations
pub mod cr {
    /// Read CR0 register
    pub fn read_cr0() -> u64 {
        let value: u64;
        unsafe { core::arch::asm!("mov {}, cr0", out(reg) value) };
        value
    }
    
    /// Write CR0 register
    pub unsafe fn write_cr0(value: u64) {
        core::arch::asm!("mov cr0, {}", in(reg) value);
    }
    
    /// Read CR2 register (page fault linear address)
    pub fn read_cr2() -> u64 {
        let value: u64;
        unsafe { core::arch::asm!("mov {}, cr2", out(reg) value) };
        value
    }
    
    /// Read CR3 register (page table base)
    pub fn read_cr3() -> u64 {
        let value: u64;
        unsafe { core::arch::asm!("mov {}, cr3", out(reg) value) };
        value
    }
    
    /// Write CR3 register (switches page tables)
    pub unsafe fn write_cr3(value: u64) {
        core::arch::asm!("mov cr3, {}", in(reg) value);
    }
    
    /// Read CR4 register
    pub fn read_cr4() -> u64 {
        let value: u64;
        unsafe { core::arch::asm!("mov {}, cr4", out(reg) value) };
        value
    }
    
    /// Write CR4 register
    pub unsafe fn write_cr4(value: u64) {
        core::arch::asm!("mov cr4, {}", in(reg) value);
    }
}

/// CR0 flags
bitflags! {
    pub struct Cr0Flags: u64 {
        /// Protected mode enable
        const PE = 1 << 0;
        /// Monitor coprocessor
        const MP = 1 << 1;
        /// Emulation
        const EM = 1 << 2;
        /// Task switched
        const TS = 1 << 3;
        /// Extension type
        const ET = 1 << 4;
        /// Numeric error
        const NE = 1 << 5;
        /// Write protect
        const WP = 1 << 16;
        /// Alignment mask
        const AM = 1 << 18;
        /// Not write-through
        const NW = 1 << 29;
        /// Cache disable
        const CD = 1 << 30;
        /// Paging
        const PG = 1 << 31;
    }
}

/// CR4 flags
bitflags! {
    pub struct Cr4Flags: u64 {
        /// Virtual-8086 mode extensions
        const VME = 1 << 0;
        /// Protected mode virtual interrupts
        const PVI = 1 << 1;
        /// Time stamp disable
        const TSD = 1 << 2;
        /// Debugging extensions
        const DE = 1 << 3;
        /// Page size extension
        const PSE = 1 << 4;
        /// Physical address extension
        const PAE = 1 << 5;
        /// Machine check exception
        const MCE = 1 << 6;
        /// Page global enable
        const PGE = 1 << 7;
        /// Performance monitoring counter enable
        const PCE = 1 << 8;
        /// OS FXSAVE/FXRSTOR support
        const OSFXSR = 1 << 9;
        /// OS unmasked exception support
        const OSXMMEXCPT = 1 << 10;
        /// User mode instruction prevention
        const UMIP = 1 << 11;
        /// 57-bit linear addresses
        const LA57 = 1 << 12;
        /// VMX enable
        const VMXE = 1 << 13;
        /// SMX enable
        const SMXE = 1 << 14;
        /// FSGSBASE enable
        const FSGSBASE = 1 << 16;
        /// PCID enable
        const PCIDE = 1 << 17;
        /// XSAVE/processor extended states enable
        const OSXSAVE = 1 << 18;
        /// Supervisor mode execution prevention
        const SMEP = 1 << 20;
        /// Supervisor mode access prevention
        const SMAP = 1 << 21;
        /// Protection keys for user pages
        const PKE = 1 << 22;
        /// Control-flow enforcement
        const CET = 1 << 23;
        /// Protection keys for supervisor pages
        const PKS = 1 << 24;
    }
}
```

---

## 3. MMU Abstraction

### 3.1 MMU Trait

```rust
/// Memory Management Unit abstraction
pub trait Mmu: Send + Sync {
    /// Page size type
    type PageSize: PageSize;
    
    /// Enable paging
    fn enable_paging(&self);
    
    /// Disable paging (if possible)
    fn disable_paging(&self);
    
    /// Check if paging is enabled
    fn paging_enabled(&self) -> bool;
    
    /// Get current page table base
    fn get_page_table_base(&self) -> PhysAddr;
    
    /// Set page table base
    unsafe fn set_page_table_base(&self, addr: PhysAddr);
    
    /// Flush TLB for a single page
    fn flush_tlb_page(&self, addr: VirtAddr);
    
    /// Flush entire TLB
    fn flush_tlb_all(&self);
    
    /// Flush TLB for a range
    fn flush_tlb_range(&self, start: VirtAddr, end: VirtAddr);
    
    /// Get page size
    fn page_size(&self) -> usize;
    
    /// Get supported page sizes
    fn supported_page_sizes(&self) -> &[usize];
}

/// Page size trait
pub trait PageSize: Copy + Clone {
    /// Size in bytes
    const SIZE: usize;
    
    /// Page size name
    const NAME: &'static str;
}

/// 4KB page
#[derive(Debug, Clone, Copy)]
pub struct Size4KB;

impl PageSize for Size4KB {
    const SIZE: usize = 4096;
    const NAME: &'static str = "4KB";
}

/// 2MB page
#[derive(Debug, Clone, Copy)]
pub struct Size2MB;

impl PageSize for Size2MB {
    const SIZE: usize = 2 * 1024 * 1024;
    const NAME: &'static str = "2MB";
}

/// 1GB page
#[derive(Debug, Clone, Copy)]
pub struct Size1GB;

impl PageSize for Size1GB {
    const SIZE: usize = 1024 * 1024 * 1024;
    const NAME: &'static str = "1GB";
}
```

### 3.2 Address Types

```rust
/// Physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Create new physical address
    pub const fn new(addr: u64) -> Self {
        // Mask to valid physical address bits (52 bits on x86_64)
        Self(addr & 0x000F_FFFF_FFFF_FFFF)
    }
    
    /// Create from raw value (no validation)
    pub const fn from_raw(addr: u64) -> Self {
        Self(addr)
    }
    
    /// Get raw address value
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
    
    /// Get as usize
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
    
    /// Align down to page boundary
    pub const fn align_down(&self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }
    
    /// Align up to page boundary
    pub const fn align_up(&self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }
    
    /// Check if aligned
    pub const fn is_aligned(&self, align: u64) -> bool {
        self.0 & (align - 1) == 0
    }
    
    /// Add offset
    pub fn offset(&self, offset: u64) -> Self {
        Self::new(self.0 + offset)
    }
}

/// Virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Create new virtual address (enforces canonical form)
    pub const fn new(addr: u64) -> Self {
        // Sign-extend from bit 47 for canonical form
        let extended = if addr & (1 << 47) != 0 {
            addr | 0xFFFF_0000_0000_0000
        } else {
            addr & 0x0000_FFFF_FFFF_FFFF
        };
        Self(extended)
    }
    
    /// Create from raw value
    pub const fn from_raw(addr: u64) -> Self {
        Self(addr)
    }
    
    /// Get raw address value
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
    
    /// Get as usize
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
    
    /// Get as pointer
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }
    
    /// Get as mutable pointer
    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }
    
    /// Get PML4 index (bits 39-47)
    pub const fn pml4_index(&self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }
    
    /// Get PDPT index (bits 30-38)
    pub const fn pdpt_index(&self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }
    
    /// Get PD index (bits 21-29)
    pub const fn pd_index(&self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }
    
    /// Get PT index (bits 12-20)
    pub const fn pt_index(&self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }
    
    /// Get page offset (bits 0-11)
    pub const fn page_offset(&self) -> usize {
        (self.0 & 0xFFF) as usize
    }
    
    /// Align down
    pub const fn align_down(&self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }
    
    /// Align up
    pub const fn align_up(&self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }
}

/// Convert between address types (for identity-mapped regions)
impl From<PhysAddr> for VirtAddr {
    fn from(phys: PhysAddr) -> Self {
        VirtAddr::new(phys.as_u64())
    }
}
```

### 3.3 MMU Implementation (x86_64)

```rust
/// x86_64 MMU implementation
pub struct X86_64Mmu;

impl Mmu for X86_64Mmu {
    type PageSize = Size4KB;
    
    fn enable_paging(&self) {
        unsafe {
            let mut cr0 = cr::read_cr0();
            cr0 |= Cr0Flags::PG.bits();
            cr::write_cr0(cr0);
        }
    }
    
    fn disable_paging(&self) {
        // Cannot disable paging in long mode
        panic!("Cannot disable paging in long mode");
    }
    
    fn paging_enabled(&self) -> bool {
        cr::read_cr0() & Cr0Flags::PG.bits() != 0
    }
    
    fn get_page_table_base(&self) -> PhysAddr {
        PhysAddr::new(cr::read_cr3() & !0xFFF)
    }
    
    unsafe fn set_page_table_base(&self, addr: PhysAddr) {
        let current = cr::read_cr3() & 0xFFF; // Preserve flags
        cr::write_cr3(addr.as_u64() | current);
    }
    
    fn flush_tlb_page(&self, addr: VirtAddr) {
        unsafe {
            core::arch::asm!("invlpg [{}]", in(reg) addr.as_u64());
        }
    }
    
    fn flush_tlb_all(&self) {
        unsafe {
            // Reload CR3 to flush entire TLB
            let cr3 = cr::read_cr3();
            cr::write_cr3(cr3);
        }
    }
    
    fn flush_tlb_range(&self, start: VirtAddr, end: VirtAddr) {
        let mut addr = start.align_down(4096);
        while addr < end {
            self.flush_tlb_page(addr);
            addr = VirtAddr::new(addr.as_u64() + 4096);
        }
    }
    
    fn page_size(&self) -> usize {
        Size4KB::SIZE
    }
    
    fn supported_page_sizes(&self) -> &[usize] {
        static SIZES: [usize; 3] = [
            Size4KB::SIZE,
            Size2MB::SIZE,
            Size1GB::SIZE,
        ];
        &SIZES
    }
}
```

---

## 4. Interrupt Abstraction

### 4.1 Interrupt Controller Trait

```rust
/// Interrupt controller abstraction
pub trait InterruptController: Send + Sync {
    /// Initialize the interrupt controller
    fn init(&mut self);
    
    /// Enable an interrupt line
    fn enable(&mut self, irq: u8);
    
    /// Disable an interrupt line
    fn disable(&mut self, irq: u8);
    
    /// Send End-of-Interrupt
    fn eoi(&mut self, irq: u8);
    
    /// Check if interrupt is pending
    fn is_pending(&self, irq: u8) -> bool;
    
    /// Get interrupt vector for an IRQ
    fn get_vector(&self, irq: u8) -> u8;
    
    /// Set interrupt vector for an IRQ
    fn set_vector(&mut self, irq: u8, vector: u8);
    
    /// Mask an interrupt
    fn mask(&mut self, irq: u8);
    
    /// Unmask an interrupt
    fn unmask(&mut self, irq: u8);
}

/// IDT entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    /// Offset bits 0-15
    offset_low: u16,
    /// Code segment selector
    selector: u16,
    /// IST index (bits 0-2) + reserved
    ist: u8,
    /// Type and attributes
    type_attr: u8,
    /// Offset bits 16-31
    offset_middle: u16,
    /// Offset bits 32-63
    offset_high: u32,
    /// Reserved
    reserved: u32,
}

impl IdtEntry {
    /// Create a new IDT entry
    pub fn new(handler: u64, selector: u16, type_attr: u8, ist: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist,
            type_attr,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }
    
    /// Create interrupt gate entry
    pub fn interrupt_gate(handler: u64, selector: u16) -> Self {
        Self::new(handler, selector, 0x8E, 0)
    }
    
    /// Create trap gate entry
    pub fn trap_gate(handler: u64, selector: u16) -> Self {
        Self::new(handler, selector, 0x8F, 0)
    }
    
    /// Create entry with IST
    pub fn with_ist(handler: u64, selector: u16, ist: u8) -> Self {
        Self::new(handler, selector, 0x8E, ist)
    }
}

/// IDT descriptor
#[repr(C, packed)]
pub struct IdtDescriptor {
    /// Size minus 1
    size: u16,
    /// Base address
    offset: u64,
}

impl IdtDescriptor {
    /// Create new IDT descriptor
    pub fn new(idt: &[IdtEntry; 256]) -> Self {
        Self {
            size: (core::mem::size_of_val(idt) - 1) as u16,
            offset: idt.as_ptr() as u64,
        }
    }
    
    /// Load the IDT
    pub unsafe fn load(&self) {
        core::arch::asm!("lidt [{}]", in(reg) self);
    }
}
```

### 4.2 APIC Implementation

```rust
/// Local APIC abstraction
pub struct LocalApic {
    /// Base address
    base: u64,
}

impl LocalApic {
    /// APIC register offsets
    const ID: u32 = 0x020;
    const VERSION: u32 = 0x030;
    const TPR: u32 = 0x080;
    const EOI: u32 = 0x0B0;
    const SPURIOUS: u32 = 0x0F0;
    const ICR_LOW: u32 = 0x300;
    const ICR_HIGH: u32 = 0x310;
    const LVT_TIMER: u32 = 0x320;
    const LVT_THERMAL: u32 = 0x330;
    const LVT_PMC: u32 = 0x340;
    const LVT_LINT0: u32 = 0x350;
    const LVT_LINT1: u32 = 0x360;
    const LVT_ERROR: u32 = 0x370;
    const TIMER_INIT: u32 = 0x380;
    const TIMER_CURRENT: u32 = 0x390;
    const TIMER_DIVIDE: u32 = 0x3E0;
    
    /// Create new LAPIC instance
    pub fn new() -> Self {
        // Get LAPIC base from MSR
        let base = unsafe {
            let msr = rdmsr(0x1B); // IA32_APIC_BASE
            msr & 0xFFFF_F000
        };
        
        Self { base }
    }
    
    /// Read LAPIC register
    fn read(&self, offset: u32) -> u32 {
        unsafe {
            let ptr = (self.base + offset as u64) as *const u32;
            core::ptr::read_volatile(ptr)
        }
    }
    
    /// Write LAPIC register
    fn write(&self, offset: u32, value: u32) {
        unsafe {
            let ptr = (self.base + offset as u64) as *mut u32;
            core::ptr::write_volatile(ptr, value);
        }
    }
    
    /// Initialize the LAPIC
    pub fn init(&mut self) {
        // Enable LAPIC (spurious interrupt vector)
        self.write(Self::SPURIOUS, 0x1FF);
        
        // Set task priority to 0 (accept all interrupts)
        self.write(Self::TPR, 0);
        
        serial_println!("[LAPIC] Initialized, ID: {}", self.id());
    }
    
    /// Get LAPIC ID
    pub fn id(&self) -> u32 {
        self.read(Self::ID) >> 24
    }
    
    /// Send EOI
    pub fn eoi(&mut self) {
        self.write(Self::EOI, 0);
    }
    
    /// Configure timer
    pub fn configure_timer(&mut self, vector: u8, divide: u8, initial: u32) {
        // Set divide configuration
        self.write(Self::TIMER_DIVIDE, divide as u32);
        
        // Set LVT timer entry (periodic mode)
        self.write(Self::LVT_TIMER, (vector as u32) | 0x20000);
        
        // Set initial count
        self.write(Self::TIMER_INIT, initial);
    }
    
    /// Stop timer
    pub fn stop_timer(&mut self) {
        self.write(Self::TIMER_INIT, 0);
        self.write(Self::LVT_TIMER, 0x10000); // Mask
    }
    
    /// Send IPI (Inter-Processor Interrupt)
    pub fn send_ipi(&mut self, target: u32, vector: u8) {
        self.write(Self::ICR_HIGH, target << 24);
        self.write(Self::ICR_LOW, vector as u32);
    }
    
    /// Send INIT IPI
    pub fn send_init_ipi(&mut self, target: u32) {
        self.write(Self::ICR_HIGH, target << 24);
        self.write(Self::ICR_LOW, 0x4500); // INIT
    }
    
    /// Send SIPI (Startup IPI)
    pub fn send_sipi(&mut self, target: u32, vector: u8) {
        self.write(Self::ICR_HIGH, target << 24);
        self.write(Self::ICR_LOW, 0x4600 | (vector as u32)); // SIPI
    }
}

/// I/O APIC abstraction
pub struct IoApic {
    /// Base address
    base: u64,
}

impl IoApic {
    /// Register offsets
    const IOREGSEL: u32 = 0x00;
    const IOWIN: u32 = 0x10;
    
    /// Registers
    const REG_ID: u32 = 0x00;
    const REG_VER: u32 = 0x01;
    const REG_ARB: u32 = 0x02;
    const REG_REDTBL_BASE: u32 = 0x10;
    
    /// Create new I/O APIC instance
    pub fn new(base: u64) -> Self {
        Self { base }
    }
    
    /// Read I/O APIC register
    fn read(&self, reg: u32) -> u32 {
        unsafe {
            let sel = self.base as *mut u32;
            let win = (self.base + 0x10) as *const u32;
            
            core::ptr::write_volatile(sel, reg);
            core::ptr::read_volatile(win)
        }
    }
    
    /// Write I/O APIC register
    fn write(&self, reg: u32, value: u32) {
        unsafe {
            let sel = self.base as *mut u32;
            let win = (self.base + 0x10) as *mut u32;
            
            core::ptr::write_volatile(sel, reg);
            core::ptr::write_volatile(win, value);
        }
    }
    
    /// Get I/O APIC ID
    pub fn id(&self) -> u32 {
        self.read(Self::REG_ID) >> 24
    }
    
    /// Get maximum redirection entries
    pub fn max_entries(&self) -> u32 {
        (self.read(Self::REG_VER) >> 16) & 0xFF
    }
    
    /// Set redirection entry
    pub fn set_irq(&mut self, irq: u8, vector: u8, dest: u8) {
        let reg = Self::REG_REDTBL_BASE + (irq as u32) * 2;
        
        // Low 32 bits: vector and flags
        let low = vector as u32;
        
        // High 32 bits: destination
        let high = (dest as u32) << 24;
        
        self.write(reg, low);
        self.write(reg + 1, high);
    }
    
    /// Mask an IRQ
    pub fn mask(&mut self, irq: u8) {
        let reg = Self::REG_REDTBL_BASE + (irq as u32) * 2;
        let value = self.read(reg) | 0x10000;
        self.write(reg, value);
    }
    
    /// Unmask an IRQ
    pub fn unmask(&mut self, irq: u8) {
        let reg = Self::REG_REDTBL_BASE + (irq as u32) * 2;
        let value = self.read(reg) & !0x10000;
        self.write(reg, value);
    }
}
```

---

## 5. Firmware Interface

### 5.1 Firmware Abstraction

```rust
/// Firmware interface abstraction
pub trait Firmware: Send + Sync {
    /// Get firmware type
    fn firmware_type(&self) -> FirmwareType;
    
    /// Get memory map
    fn memory_map(&self) -> &[MemoryRegion];
    
    /// Get ACPI RSDP address
    fn acpi_rsdp(&self) -> Option<PhysAddr>;
    
    /// Get framebuffer info
    fn framebuffer(&self) -> Option<FramebufferInfo>;
    
    /// Shutdown the system
    fn shutdown(&self) -> !;
    
    /// Reboot the system
    fn reboot(&self) -> !;
}

/// Firmware types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    /// Legacy BIOS
    Bios,
    /// UEFI
    Uefi,
    /// Unknown/other
    Unknown,
}

/// Memory region from firmware
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Physical start address
    pub start: PhysAddr,
    /// Length in bytes
    pub length: u64,
    /// Region type
    pub region_type: MemoryRegionType,
}

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Available for use
    Available,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Kernel code/data
    Kernel,
    /// Bootloader
    Bootloader,
    /// Unknown
    Unknown,
}

/// Framebuffer information
#[derive(Debug, Clone)]
pub struct FramebufferInfo {
    /// Framebuffer physical address
    pub address: PhysAddr,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per row)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Pixel format
    pub format: PixelFormat,
}

/// Pixel formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGB (red, green, blue)
    Rgb,
    /// BGR (blue, green, red)
    Bgr,
    /// RGB with alpha
    Rgba,
    /// BGR with alpha
    Bgra,
}
```

### 5.2 Multiboot2 Parser

```rust
/// Multiboot2 boot information
pub struct Multiboot2Info {
    /// Raw pointer to boot info
    ptr: *const u8,
    /// Total size
    total_size: u32,
}

impl Multiboot2Info {
    /// Create from Multiboot2 pointer
    pub unsafe fn from_ptr(ptr: *const u8) -> Self {
        let total_size = *(ptr as *const u32);
        Self { ptr, total_size }
    }
    
    /// Iterate over tags
    pub fn tags(&self) -> TagIterator {
        TagIterator {
            current: unsafe { self.ptr.add(8) },
            end: unsafe { self.ptr.add(self.total_size as usize) },
        }
    }
    
    /// Get memory map tag
    pub fn memory_map(&self) -> Option<&MemoryMapTag> {
        for tag in self.tags() {
            if tag.tag_type == TagType::MemoryMap {
                return Some(unsafe { &*(tag as *const Tag as *const MemoryMapTag) });
            }
        }
        None
    }
    
    /// Get framebuffer tag
    pub fn framebuffer(&self) -> Option<&FramebufferTag> {
        for tag in self.tags() {
            if tag.tag_type == TagType::Framebuffer {
                return Some(unsafe { &*(tag as *const Tag as *const FramebufferTag) });
            }
        }
        None
    }
    
    /// Get command line
    pub fn command_line(&self) -> Option<&str> {
        for tag in self.tags() {
            if tag.tag_type == TagType::CommandLine {
                let string_tag = unsafe { &*(tag as *const Tag as *const StringTag) };
                return Some(string_tag.string());
            }
        }
        None
    }
}

/// Multiboot2 tag types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TagType {
    End = 0,
    CommandLine = 1,
    BootLoaderName = 2,
    Modules = 3,
    BasicMemInfo = 4,
    BootDevice = 5,
    MemoryMap = 6,
    Vbe = 7,
    Framebuffer = 8,
    ElfSymbols = 9,
    Apm = 10,
    Efi32 = 11,
    Efi64 = 12,
    SmBios = 13,
    AcpiOld = 14,
    AcpiNew = 15,
    Network = 16,
    EfiMmap = 17,
    EfiBs = 18,
    Efi32Ih = 19,
    Efi64Ih = 20,
    LoadBaseAddr = 21,
}

/// Base tag structure
#[repr(C)]
pub struct Tag {
    pub tag_type: TagType,
    pub size: u32,
}

/// Memory map tag
#[repr(C)]
pub struct MemoryMapTag {
    pub tag_type: TagType,
    pub size: u32,
    pub entry_size: u32,
    pub entry_version: u32,
    // Entries follow
}

impl MemoryMapTag {
    /// Get memory map entries
    pub fn entries(&self) -> MemoryMapIterator {
        let start = unsafe {
            (self as *const Self as *const u8).add(16)
        };
        let end = unsafe {
            (self as *const Self as *const u8).add(self.size as usize)
        };
        
        MemoryMapIterator {
            current: start,
            end,
            entry_size: self.entry_size,
        }
    }
}

/// Memory map entry
#[repr(C)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub entry_type: u32,
    pub reserved: u32,
}

/// Memory map iterator
pub struct MemoryMapIterator {
    current: *const u8,
    end: *const u8,
    entry_size: u32,
}

impl Iterator for MemoryMapIterator {
    type Item = &'static MemoryMapEntry;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }
        
        let entry = unsafe { &*(self.current as *const MemoryMapEntry) };
        self.current = unsafe { self.current.add(self.entry_size as usize) };
        
        Some(entry)
    }
}
```

---

## 6. Architecture Support

### 6.1 Architecture Detection

```rust
/// Current architecture
#[cfg(target_arch = "x86_64")]
pub type CurrentArch = X86_64Arch;

/// Architecture abstraction
pub trait Architecture {
    /// Architecture name
    const NAME: &'static str;
    
    /// Pointer width in bits
    const POINTER_WIDTH: usize;
    
    /// Page size
    const PAGE_SIZE: usize;
    
    /// Initialize architecture
    fn init();
    
    /// Get CPU instance
    fn cpu() -> &'static dyn Cpu;
    
    /// Get MMU instance
    fn mmu() -> &'static dyn Mmu;
    
    /// Get interrupt controller
    fn interrupt_controller() -> &'static dyn InterruptController;
}

/// x86_64 architecture
pub struct X86_64Arch;

impl Architecture for X86_64Arch {
    const NAME: &'static str = "x86_64";
    const POINTER_WIDTH: usize = 64;
    const PAGE_SIZE: usize = 4096;
    
    fn init() {
        // Initialize GDT
        gdt::init();
        
        // Initialize IDT
        idt::init();
        
        // Initialize APIC
        apic::init();
        
        serial_println!("[ARCH] x86_64 initialized");
    }
    
    fn cpu() -> &'static dyn Cpu {
        static CPU: X86_64Cpu = X86_64Cpu::new();
        &CPU
    }
    
    fn mmu() -> &'static dyn Mmu {
        static MMU: X86_64Mmu = X86_64Mmu;
        &MMU
    }
    
    fn interrupt_controller() -> &'static dyn InterruptController {
        // Return APIC or PIC based on detection
        todo!()
    }
}
```

### 6.2 GDT Implementation

```rust
/// Global Descriptor Table entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    /// Null descriptor
    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }
    
    /// Code segment (64-bit)
    pub const fn code_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0x9A, // Present, DPL 0, Code, Execute/Read
            granularity: 0xAF, // 64-bit, 4KB granularity
            base_high: 0,
        }
    }
    
    /// Data segment
    pub const fn data_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0x92, // Present, DPL 0, Data, Read/Write
            granularity: 0xCF, // 32-bit, 4KB granularity
            base_high: 0,
        }
    }
    
    /// User code segment (64-bit)
    pub const fn user_code_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0xFA, // Present, DPL 3, Code, Execute/Read
            granularity: 0xAF,
            base_high: 0,
        }
    }
    
    /// User data segment
    pub const fn user_data_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            access: 0xF2, // Present, DPL 3, Data, Read/Write
            granularity: 0xCF,
            base_high: 0,
        }
    }
}

/// TSS entry (16 bytes)
#[repr(C, packed)]
pub struct TssEntry {
    length: u16,
    base_low: u16,
    base_middle: u8,
    flags1: u8,
    flags2: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

/// Task State Segment
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved1: u32,
    /// Stack pointers for privilege levels
    pub rsp: [u64; 3],
    reserved2: u64,
    /// Interrupt stack table
    pub ist: [u64; 7],
    reserved3: u64,
    reserved4: u16,
    /// I/O map base address
    pub iomap_base: u16,
}

/// GDT with TSS
#[repr(C, packed)]
pub struct Gdt {
    pub null: GdtEntry,
    pub kernel_code: GdtEntry,
    pub kernel_data: GdtEntry,
    pub user_data: GdtEntry,
    pub user_code: GdtEntry,
    pub tss: TssEntry,
}

/// GDT descriptor
#[repr(C, packed)]
pub struct GdtDescriptor {
    size: u16,
    offset: u64,
}

/// Load GDT
pub fn load_gdt(gdt: &'static Gdt) {
    let descriptor = GdtDescriptor {
        size: (core::mem::size_of::<Gdt>() - 1) as u16,
        offset: gdt as *const Gdt as u64,
    };
    
    unsafe {
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &descriptor,
        );
        
        // Reload code segment
        core::arch::asm!(
            "push 0x08",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
        );
        
        // Reload data segments
        core::arch::asm!(
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
        );
    }
}
```

---

## 7. Port I/O

### 7.1 Port I/O Functions

```rust
/// Read byte from I/O port
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
    );
    value
}

/// Write byte to I/O port
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    );
}

/// Read word from I/O port
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    core::arch::asm!(
        "in ax, dx",
        out("ax") value,
        in("dx") port,
    );
    value
}

/// Write word to I/O port
#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
    );
}

/// Read dword from I/O port
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    core::arch::asm!(
        "in eax, dx",
        out("eax") value,
        in("dx") port,
    );
    value
}

/// Write dword to I/O port
#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    core::arch::asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
    );
}

/// I/O wait (short delay)
#[inline]
pub unsafe fn io_wait() {
    outb(0x80, 0);
}

/// Port wrapper for type-safe I/O
pub struct Port<T> {
    port: u16,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Port<T> {
    /// Create new port
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl Port<u8> {
    /// Read byte
    pub unsafe fn read(&self) -> u8 {
        inb(self.port)
    }
    
    /// Write byte
    pub unsafe fn write(&self, value: u8) {
        outb(self.port, value);
    }
}

impl Port<u16> {
    /// Read word
    pub unsafe fn read(&self) -> u16 {
        inw(self.port)
    }
    
    /// Write word
    pub unsafe fn write(&self, value: u16) {
        outw(self.port, value);
    }
}

impl Port<u32> {
    /// Read dword
    pub unsafe fn read(&self) -> u32 {
        inl(self.port)
    }
    
    /// Write dword
    pub unsafe fn write(&self, value: u32) {
        outl(self.port, value);
    }
}
```

---

## 8. Timing

### 8.1 Timer Abstraction

```rust
/// Timer abstraction
pub trait Timer: Send + Sync {
    /// Get current time in nanoseconds since boot
    fn now_ns(&self) -> u64;
    
    /// Get current time in milliseconds since boot
    fn now_ms(&self) -> u64 {
        self.now_ns() / 1_000_000
    }
    
    /// Get current time in seconds since boot
    fn now_secs(&self) -> u64 {
        self.now_ns() / 1_000_000_000
    }
    
    /// Get timer frequency in Hz
    fn frequency(&self) -> u64;
    
    /// Busy-wait for given nanoseconds
    fn busy_wait_ns(&self, ns: u64);
    
    /// Busy-wait for given milliseconds
    fn busy_wait_ms(&self, ms: u64) {
        self.busy_wait_ns(ms * 1_000_000);
    }
}

/// PIT timer
pub struct PitTimer {
    frequency: u64,
    ticks: AtomicU64,
}

impl PitTimer {
    /// PIT frequency
    pub const FREQUENCY: u64 = 1_193_182;
    
    /// I/O ports
    const CHANNEL_0: u16 = 0x40;
    const COMMAND: u16 = 0x43;
    
    /// Create new PIT timer
    pub const fn new() -> Self {
        Self {
            frequency: Self::FREQUENCY,
            ticks: AtomicU64::new(0),
        }
    }
    
    /// Initialize PIT
    pub fn init(&self, hz: u32) {
        let divisor = (Self::FREQUENCY / hz as u64) as u16;
        
        unsafe {
            // Mode 3 (square wave), binary
            outb(Self::COMMAND, 0x36);
            
            // Set divisor
            outb(Self::CHANNEL_0, (divisor & 0xFF) as u8);
            outb(Self::CHANNEL_0, (divisor >> 8) as u8);
        }
    }
    
    /// Handle timer interrupt
    pub fn tick(&self) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
    }
}

impl Timer for PitTimer {
    fn now_ns(&self) -> u64 {
        let ticks = self.ticks.load(Ordering::Relaxed);
        ticks * 1_000_000_000 / self.frequency
    }
    
    fn frequency(&self) -> u64 {
        self.frequency
    }
    
    fn busy_wait_ns(&self, ns: u64) {
        let start = self.now_ns();
        while self.now_ns() - start < ns {
            core::hint::spin_loop();
        }
    }
}

/// TSC timer (timestamp counter)
pub struct TscTimer {
    frequency: u64,
    start_tsc: u64,
}

impl TscTimer {
    /// Create new TSC timer (requires calibration)
    pub fn new(frequency: u64) -> Self {
        Self {
            frequency,
            start_tsc: rdtsc(),
        }
    }
    
    /// Calibrate TSC using PIT
    pub fn calibrate() -> u64 {
        // Use PIT to calibrate TSC
        let pit = PitTimer::new();
        
        // Measure TSC ticks over 10ms
        let start_tsc = rdtsc();
        pit.busy_wait_ms(10);
        let end_tsc = rdtsc();
        
        // Calculate frequency
        (end_tsc - start_tsc) * 100 // 10ms -> 1s
    }
}

impl Timer for TscTimer {
    fn now_ns(&self) -> u64 {
        let tsc = rdtsc() - self.start_tsc;
        tsc * 1_000_000_000 / self.frequency
    }
    
    fn frequency(&self) -> u64 {
        self.frequency
    }
    
    fn busy_wait_ns(&self, ns: u64) {
        let target_tsc = rdtsc() + (ns * self.frequency / 1_000_000_000);
        while rdtsc() < target_tsc {
            core::hint::spin_loop();
        }
    }
}

/// Read TSC
#[inline]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}
```

---

## Summary

The Helix HAL provides:

1. **CPU Abstraction**: Unified interface for CPU operations
2. **MMU Abstraction**: Page table and virtual memory management
3. **Interrupt Abstraction**: IDT, APIC, and interrupt routing
4. **Firmware Interface**: Multiboot2 parsing, ACPI discovery
5. **Architecture Support**: x86_64 with plans for ARM64/RISC-V
6. **Port I/O**: Type-safe I/O port access
7. **Timing**: PIT and TSC timer support

For implementation details, see [hal/src/](../../hal/src/).

---

<div align="center">

🔌 *Abstracting hardware complexity* 🔌

</div>
