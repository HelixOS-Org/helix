//! # System Call Interface
//!
//! Fast syscall/sysret support for x86_64.
//!
//! ## Calling Convention
//!
//! - Syscall number in RAX
//! - Arguments in RDI, RSI, RDX, R10, R8, R9
//! - Return value in RAX
//! - RCX and R11 are clobbered by syscall instruction
//!
//! ## Available Syscalls
//!
//! | Number | Name       | Description              |
//! |--------|------------|--------------------------|
//! | 0      | exit       | Exit current task        |
//! | 1      | write      | Write to output          |
//! | 2      | yield      | Yield CPU time           |
//! | 3      | getpid     | Get task ID              |
//! | 4      | sleep      | Sleep for N milliseconds |

use core::arch::{asm, naked_asm};

/// Syscall numbers
pub mod nr {
    pub const EXIT: u64 = 0;
    pub const WRITE: u64 = 1;
    pub const YIELD: u64 = 2;
    pub const GETPID: u64 = 3;
    pub const SLEEP: u64 = 4;
    pub const DEBUG: u64 = 255;
}

/// Model Specific Registers for syscall
mod msr {
    pub const STAR: u32 = 0xC0000081; // Segment selectors
    pub const LSTAR: u32 = 0xC0000082; // Syscall entry point (64-bit)
    pub const CSTAR: u32 = 0xC0000083; // Syscall entry point (compat mode)
    pub const SFMASK: u32 = 0xC0000084; // Flags mask
    pub const EFER: u32 = 0xC0000080; // Extended features
}

/// EFER bits
mod efer {
    pub const SCE: u64 = 1 << 0; // System Call Extensions
}

/// Initialize syscall/sysret support
///
/// # Safety
/// Must be called once during boot, after GDT is set up.
pub unsafe fn init() {
    // Enable syscall extensions in EFER
    let efer = rdmsr(msr::EFER);
    wrmsr(msr::EFER, efer | efer::SCE);

    // STAR register layout:
    // Bits 31:0  - Reserved (EIP for 32-bit SYSCALL, not used in 64-bit)
    // Bits 47:32 - Kernel CS (SYSCALL loads this into CS)
    // Bits 63:48 - User CS base (SYSRET uses this)
    //
    // For SYSRET in 64-bit mode:
    //   CS = STAR[63:48] + 16  (with RPL forced to 3)
    //   SS = STAR[63:48] + 8   (with RPL forced to 3)
    //
    // Our GDT (updated):
    //   0x00: Null
    //   0x08: Kernel Code (selector 0x08)
    //   0x10: Kernel Data (selector 0x10)
    //   0x18: User Data (selector 0x1B with RPL=3)
    //   0x20: User Code (selector 0x23 with RPL=3)
    //
    // For SYSRET we need:
    //   SS = 0x1B (user data)  => base + 8  = 0x18 => base = 0x10
    //   CS = 0x23 (user code)  => base + 16 = 0x20 => base = 0x10
    //
    // So user_base = 0x10, kernel_base = 0x08
    // Wait that's wrong because SS = 0x10 + 8 = 0x18, but we want 0x18|3 = 0x1B
    // The RPL is added automatically by SYSRET!
    //
    // So: STAR[63:48] = 0x10 => SS = 0x10 + 8 | 3 = 0x1B ✓
    //                       => CS = 0x10 + 16 | 3 = 0x23 ✓

    let kernel_base: u64 = 0x08;
    let user_base: u64 = 0x10; // SYSRET adds 8 for SS (0x18) and 16 for CS (0x20)

    let star = (user_base << 48) | (kernel_base << 32);
    wrmsr(msr::STAR, star);

    // Set syscall entry point
    wrmsr(msr::LSTAR, syscall_entry as u64);

    // Set compat mode entry (not used, but required)
    wrmsr(msr::CSTAR, 0);

    // Set flags mask (clear IF and TF on syscall entry)
    wrmsr(msr::SFMASK, 0x300); // Clear IF (0x200) and TF (0x100)

    log::info!(
        "Syscall/sysret initialized (STAR={:#x}, LSTAR={:#x})",
        star,
        syscall_entry as u64
    );
}

/// Syscall entry point
///
/// RCX = user RIP (return address)
/// R11 = user RFLAGS
/// RAX = syscall number
#[naked]
pub unsafe extern "C" fn syscall_entry() {
    unsafe {
        naked_asm!(
            // We're now in kernel mode (Ring 0)
            // RCX = return RIP, R11 = return RFLAGS
            // RAX = syscall number
            // RDI, RSI, RDX, R10, R8, R9 = arguments

            // Save user state on current stack
            // Note: In production, we'd switch to a kernel stack via TSS
            "push rcx",      // User RIP
            "push r11",      // User RFLAGS
            "push rbp",
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "push rdi",      // Save arg0
            "push rsi",      // Save arg1
            "push rdx",      // Save arg2

            // Set up arguments for dispatcher
            // dispatcher(syscall_nr, arg0, arg1, arg2, arg3, arg4)
            // RDI = syscall number (was in RAX)
            // RSI = arg0 (was in RDI)
            // RDX = arg1 (was in RSI)
            // RCX = arg2 (was in RDX)
            // R8 = arg3 (was in R10)
            // R9 = arg4 (was in R8)
            "mov r9, r8",    // arg4
            "mov r8, r10",   // arg3
            "pop rcx",       // arg2 (restore RDX)
            "pop rdx",       // arg1 (restore RSI)
            "pop rsi",       // arg0 (restore RDI)
            "mov rdi, rax",  // syscall_nr

            // Call the dispatcher
            "call {dispatcher}",

            // Result is in RAX, keep it

            // Restore user state
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",
            "pop r11",      // User RFLAGS
            "pop rcx",      // User RIP

            // Return to userspace (Ring 3)
            "sysretq",

            dispatcher = sym syscall_dispatcher,
        );
    }
}

/// Syscall dispatcher (called from assembly)
///
/// Arguments are already set up by the assembly stub.
#[no_mangle]
pub extern "C" fn syscall_dispatcher(
    syscall_nr: u64, // RDI - syscall number
    arg0: u64,       // RSI - first argument
    arg1: u64,       // RDX - second argument
    arg2: u64,       // RCX - third argument
    arg3: u64,       // R8 - fourth argument
    _arg4: u64,      // R9 - fifth argument
) -> u64 {
    match syscall_nr {
        nr::EXIT => {
            // Print exit message directly to serial
            let msg = b"\n[SYSCALL] exit(";
            for &c in msg {
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
                }
            }
            // Print exit code
            let code = arg0 as i32;
            if code >= 0 && code < 100 {
                let tens = ((code / 10) as u8) + b'0';
                let ones = ((code % 10) as u8) + b'0';
                if tens > b'0' {
                    unsafe {
                        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") tens);
                    }
                }
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") ones);
                }
            }
            let msg2 = b") - Task exiting from Ring 3!\n";
            for &c in msg2 {
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
                }
            }

            // Exit the task
            super::task::scheduler().exit(code);

            // Should never return, but halt just in case
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        },
        nr::WRITE => {
            // write(fd, buf, len)
            let _fd = arg0;
            let buf = arg1 as *const u8;
            let len = arg2 as usize;

            // Write to serial port
            unsafe {
                for i in 0..len {
                    let c = *buf.add(i);
                    if c == 0 {
                        break;
                    } // Stop at null
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3F8u16,
                        in("al") c,
                        options(nomem, nostack)
                    );
                }
            }

            len as u64
        },
        nr::YIELD => {
            super::task::yield_now();
            0
        },
        nr::GETPID => super::task::scheduler().current_task_id().as_u64(),
        nr::DEBUG => {
            // Debug syscall - print a value
            let msg = b"[DEBUG] value=";
            for &c in msg {
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
                }
            }
            // Print hex value
            let hex = b"0123456789abcdef";
            for i in (0..16).rev() {
                let nibble = ((arg0 >> (i * 4)) & 0xF) as usize;
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
                }
            }
            unsafe {
                core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
            }
            arg0
        },
        _ => {
            // Unknown syscall
            let msg = b"[SYSCALL] Unknown syscall: ";
            for &c in msg {
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
                }
            }
            let hex = b"0123456789abcdef";
            for i in (0..4).rev() {
                let nibble = ((syscall_nr >> (i * 4)) & 0xF) as usize;
                unsafe {
                    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
                }
            }
            unsafe {
                core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
            }
            u64::MAX // Error
        },
    }
}

// =============================================================================
// MSR Helpers
// =============================================================================

/// Read from a Model Specific Register
#[inline]
fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Write to a Model Specific Register
#[inline]
fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
}

// =============================================================================
// User-space syscall wrappers (for testing)
// =============================================================================

/// Exit the current task (from userspace)
#[inline]
pub fn sys_exit(code: i32) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") nr::EXIT,
            in("rdi") code,
            options(noreturn)
        );
    }
}

/// Write data (from userspace)
#[inline]
pub fn sys_write(fd: u64, buf: *const u8, len: usize) -> i64 {
    let ret: i64;
    unsafe {
        asm!(
            "syscall",
            inout("rax") nr::WRITE => ret,
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") len,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

/// Yield CPU (from userspace)
#[inline]
pub fn sys_yield() {
    unsafe {
        asm!(
            "syscall",
            in("rax") nr::YIELD,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
}

/// Get task ID (from userspace)
#[inline]
pub fn sys_getpid() -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inout("rax") nr::GETPID => ret,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}
