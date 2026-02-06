//! # Kernel Entry Point Utilities
//!
//! This module provides utilities for defining kernel entry points and
//! setting up the initial execution environment with Limine.
//!
//! ## Features
//!
//! - Entry point macros with automatic request setup
//! - Stack guard page support
//! - Early boot console initialization
//! - Panic handler integration
//!
//! ## Example
//!
//! ```rust,no_run
//! use helix_limine::entry;
//!
//! entry::limine_entry! {
//!     fn kernel_main(boot_info: &BootInfo) -> ! {
//!         // Kernel initialization code
//!         loop {}
//!     }
//! }
//! ```

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::boot_info::{BootInfo, BootInfoBuilder};
use crate::requests::{
    BootTimeRequest, BootloaderInfoRequest, FramebufferRequest, HhdmRequest, KernelAddressRequest,
    KernelFileRequest, MemoryMapRequest, ModuleRequest, PagingModeRequest, RsdpRequest,
    SmbiosRequest, SmpRequest,
};

/// Flag indicating if early boot has completed
static EARLY_BOOT_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Flag indicating if a panic is in progress
static PANIC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Check if early boot has completed
pub fn is_early_boot_complete() -> bool {
    EARLY_BOOT_COMPLETE.load(Ordering::Acquire)
}

/// Mark early boot as complete
pub fn mark_early_boot_complete() {
    EARLY_BOOT_COMPLETE.store(true, Ordering::Release);
}

/// Check if a panic is in progress
pub fn is_panic_in_progress() -> bool {
    PANIC_IN_PROGRESS.load(Ordering::Acquire)
}

/// Halt the CPU in an infinite loop
///
/// This function disables interrupts and halts the CPU repeatedly.
/// Use this when the kernel cannot continue execution.
#[allow(clippy::inline_always)] // Critical hardware function must be inlined
#[inline(always)]
pub fn halt_loop() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            core::hint::spin_loop();
        }
    }
}

/// Standard Limine requests for a typical kernel
pub struct StandardRequests {
    /// Bootloader info
    pub bootloader: BootloaderInfoRequest,
    /// Memory map
    pub memory_map: MemoryMapRequest,
    /// HHDM
    pub hhdm: HhdmRequest,
    /// Paging mode
    pub paging_mode: PagingModeRequest,
    /// Kernel file
    pub kernel_file: KernelFileRequest,
    /// Kernel address
    pub kernel_address: KernelAddressRequest,
    /// Modules
    pub modules: ModuleRequest,
    /// SMP
    pub smp: SmpRequest,
    /// Framebuffer
    pub framebuffer: FramebufferRequest,
    /// RSDP
    pub rsdp: RsdpRequest,
    /// SMBIOS
    pub smbios: SmbiosRequest,
    /// Boot time
    pub boot_time: BootTimeRequest,
}

impl StandardRequests {
    /// Create standard requests with defaults
    pub const fn new() -> Self {
        Self {
            bootloader: BootloaderInfoRequest::new(),
            memory_map: MemoryMapRequest::new(),
            hhdm: HhdmRequest::new(),
            paging_mode: PagingModeRequest::new(),
            kernel_file: KernelFileRequest::new(),
            kernel_address: KernelAddressRequest::new(),
            modules: ModuleRequest::new(),
            smp: SmpRequest::new(),
            framebuffer: FramebufferRequest::new(),
            rsdp: RsdpRequest::new(),
            smbios: SmbiosRequest::new(),
            boot_time: BootTimeRequest::new(),
        }
    }

    /// Build boot info from responses
    ///
    /// # Errors
    ///
    /// Returns an error if required boot info components are missing.
    pub fn build_boot_info(&self) -> Result<BootInfo<'_>, crate::boot_info::BootInfoError> {
        BootInfoBuilder::new()
            .with_bootloader(&self.bootloader)
            .with_memory_map(&self.memory_map)
            .with_hhdm(&self.hhdm)
            .with_paging_mode(&self.paging_mode)
            .with_kernel_file(&self.kernel_file)
            .with_kernel_address(&self.kernel_address)
            .with_modules(&self.modules)
            .with_smp(&self.smp)
            .with_framebuffer(&self.framebuffer)
            .with_rsdp(&self.rsdp)
            .with_smbios(&self.smbios)
            .with_boot_time(&self.boot_time)
            .build()
    }
}

impl Default for StandardRequests {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to declare a Limine entry point with automatic request setup
///
/// This macro creates all necessary boilerplate for a Limine-based kernel:
/// - Request structures in the appropriate linker sections
/// - Entry point function
/// - Panic handler (optional)
///
/// # Example
///
/// ```rust,no_run
/// helix_limine::limine_entry! {
///     fn kernel_main(boot_info: &helix_limine::boot_info::BootInfo) -> ! {
///         loop {}
///     }
/// }
/// ```
#[macro_export]
macro_rules! limine_entry {
    (fn $name:ident($info:ident : &BootInfo) -> ! $body:block) => {
        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_BOOTLOADER_INFO: $crate::requests::BootloaderInfoRequest =
            $crate::requests::BootloaderInfoRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_MEMMAP: $crate::requests::MemoryMapRequest =
            $crate::requests::MemoryMapRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_HHDM: $crate::requests::HhdmRequest = $crate::requests::HhdmRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_KERNEL_ADDRESS: $crate::requests::KernelAddressRequest =
            $crate::requests::KernelAddressRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_FRAMEBUFFER: $crate::requests::FramebufferRequest =
            $crate::requests::FramebufferRequest::new();

        #[no_mangle]
        extern "C" fn _start() -> ! {
            // Build boot info from responses
            let $info = $crate::boot_info::BootInfoBuilder::new()
                .with_bootloader(&__LIMINE_BOOTLOADER_INFO)
                .with_memory_map(&__LIMINE_MEMMAP)
                .with_hhdm(&__LIMINE_HHDM)
                .with_kernel_address(&__LIMINE_KERNEL_ADDRESS)
                .with_framebuffer(&__LIMINE_FRAMEBUFFER)
                .build_unchecked();

            $crate::entry::mark_early_boot_complete();

            $body
        }
    };

    (requests: $requests:expr,fn $name:ident($info:ident : &BootInfo) -> ! $body:block) => {
        #[no_mangle]
        extern "C" fn _start() -> ! {
            let requests = $requests;
            let $info = requests
                .build_boot_info()
                .expect("Failed to build boot info");

            $crate::entry::mark_early_boot_complete();

            $body
        }
    };
}

/// Macro to declare standard Limine requests
///
/// This creates all standard request structures in the proper linker sections.
#[macro_export]
macro_rules! limine_standard_requests {
    () => {
        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_BOOTLOADER_INFO: $crate::requests::BootloaderInfoRequest =
            $crate::requests::BootloaderInfoRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_MEMMAP: $crate::requests::MemoryMapRequest =
            $crate::requests::MemoryMapRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_HHDM: $crate::requests::HhdmRequest = $crate::requests::HhdmRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_KERNEL_FILE: $crate::requests::KernelFileRequest =
            $crate::requests::KernelFileRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_KERNEL_ADDRESS: $crate::requests::KernelAddressRequest =
            $crate::requests::KernelAddressRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_FRAMEBUFFER: $crate::requests::FramebufferRequest =
            $crate::requests::FramebufferRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_RSDP: $crate::requests::RsdpRequest = $crate::requests::RsdpRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_SMP: $crate::requests::SmpRequest = $crate::requests::SmpRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_BOOT_TIME: $crate::requests::BootTimeRequest =
            $crate::requests::BootTimeRequest::new();
    };

    (minimal) => {
        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_MEMMAP: $crate::requests::MemoryMapRequest =
            $crate::requests::MemoryMapRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_HHDM: $crate::requests::HhdmRequest = $crate::requests::HhdmRequest::new();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_KERNEL_ADDRESS: $crate::requests::KernelAddressRequest =
            $crate::requests::KernelAddressRequest::new();
    };

    (graphical) => {
        $crate::limine_requests!();

        #[used]
        #[link_section = ".limine_requests"]
        static LIMINE_FRAMEBUFFER_EXTRA: $crate::requests::FramebufferRequest =
            $crate::requests::FramebufferRequest::with_revision(1);
    };
}

/// Early boot panic handler
///
/// This provides a minimal panic handler for use before the kernel's
/// own panic infrastructure is initialized.
pub fn early_panic_handler(_info: &PanicInfo<'_>) -> ! {
    // Prevent recursive panics
    if PANIC_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        loop {
            core::hint::spin_loop();
        }
    }

    // Try to print panic info if framebuffer is available
    #[cfg(feature = "framebuffer")]
    {
        // Minimal framebuffer output would go here
    }

    // Halt all CPUs
    halt_all_cpus()
}

/// Halt all CPUs
pub fn halt_all_cpus() -> ! {
    // Disable interrupts and halt
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm!("cli");
            loop {
                core::arch::asm!("hlt");
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            loop {
                core::arch::asm!("wfi");
            }
        }
    }

    #[cfg(target_arch = "riscv64")]
    {
        unsafe {
            loop {
                core::arch::asm!("wfi");
            }
        }
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        loop {
            core::hint::spin_loop();
        }
    }
}

/// Halt the current CPU
#[allow(clippy::inline_always)] // Critical hardware function must be inlined
#[inline(always)]
pub fn halt() -> ! {
    halt_all_cpus()
}

/// CPU relaxation hint for busy-wait loops
#[allow(clippy::inline_always)] // Critical hardware function must be inlined
#[inline(always)]
pub fn cpu_relax() {
    core::hint::spin_loop();
}

/// Memory fence for cross-CPU synchronization
#[allow(clippy::inline_always)] // Critical hardware function must be inlined
#[inline(always)]
pub fn memory_fence() {
    core::sync::atomic::fence(Ordering::SeqCst);
}

/// Early debug output trait
pub trait EarlyDebug {
    /// Write a string to early debug output
    fn write_str(&mut self, s: &str);

    /// Write a character to early debug output
    fn write_char(&mut self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.write_str(s);
    }

    /// Write a formatted number
    fn write_hex(&mut self, value: u64) {
        const HEX_CHARS: &[u8] = b"0123456789abcdef";
        let mut buf = [0u8; 18]; // "0x" + 16 hex digits
        buf[0] = b'0';
        buf[1] = b'x';

        for i in 0..16 {
            let nibble = ((value >> (60 - i * 4)) & 0xF) as usize;
            buf[2 + i] = HEX_CHARS[nibble];
        }

        // Safety: buf contains valid ASCII
        let s = unsafe { core::str::from_utf8_unchecked(&buf) };
        self.write_str(s);
    }
}

/// Serial port debug output (`x86_64`)
#[cfg(target_arch = "x86_64")]
pub struct SerialDebug {
    port: u16,
}

#[cfg(target_arch = "x86_64")]
impl SerialDebug {
    /// COM1 port
    pub const COM1: u16 = 0x3F8;
    /// COM2 port
    pub const COM2: u16 = 0x2F8;

    /// Create a new serial debug output
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    /// Initialize the serial port
    pub fn init(&self) {
        unsafe {
            // Disable interrupts
            Self::outb(self.port + 1, 0x00);
            // Enable DLAB
            Self::outb(self.port + 3, 0x80);
            // Set divisor to 1 (115200 baud)
            Self::outb(self.port, 0x01);
            Self::outb(self.port + 1, 0x00);
            // 8 bits, no parity, one stop bit
            Self::outb(self.port + 3, 0x03);
            // Enable FIFO
            Self::outb(self.port + 2, 0xC7);
            // Set OUT2 and RTS
            Self::outb(self.port + 4, 0x0B);
        }
    }

    #[allow(clippy::inline_always)] // Critical I/O port function must be inlined
    #[inline(always)]
    unsafe fn outb(port: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    #[allow(clippy::inline_always)] // Critical I/O port function must be inlined
    #[inline(always)]
    unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { Self::inb(self.port + 5) & 0x20 != 0 }
    }

    fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {
            cpu_relax();
        }
        unsafe {
            Self::outb(self.port, byte);
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl EarlyDebug for SerialDebug {
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_requests() {
        let requests = StandardRequests::new();
        // Just ensure it compiles
        let _ = requests;
    }
}
