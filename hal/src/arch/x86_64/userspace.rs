//! # Userspace Execution
//!
//! This module handles the transition from kernel mode (Ring 0) to user mode (Ring 3)
//! and provides the infrastructure for running userspace programs.
//!
//! ## How it works
//!
//! 1. We set up a user stack in a designated memory region
//! 2. We use IRETQ to jump to Ring 3 with the user code/data segments
//! 3. The userspace code can use SYSCALL to request kernel services
//! 4. SYSCALL jumps back to Ring 0, we handle the request, then SYSRET back
//!
//! ## Memory Layout (simplified, identity mapped for now)
//!
//! ```text
//! 0x0000_0000_0040_0000 - User code start (4MB)
//! 0x0000_0000_0080_0000 - User stack top (8MB)
//! ```

use core::arch::{asm, naked_asm};
use super::gdt::{USER_CODE_SELECTOR, USER_DATA_SELECTOR, KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};

/// User stack size (64KB)
const USER_STACK_SIZE: usize = 64 * 1024;

/// User stack storage
#[repr(align(4096))]
struct UserStack([u8; USER_STACK_SIZE]);

static mut USER_STACK: UserStack = UserStack([0; USER_STACK_SIZE]);

/// Jump to userspace and execute code at the given entry point
///
/// This function never returns - it transfers control to Ring 3.
///
/// # Arguments
/// * `entry_point` - Address of the userspace code to execute
///
/// # Safety
/// The entry point must be valid executable code that uses syscalls properly.
pub unsafe fn jump_to_userspace(entry_point: u64) -> ! {
    let user_stack_top = unsafe { USER_STACK.0.as_ptr() as u64 + USER_STACK_SIZE as u64 };
    
    // Debug output
    let msg = b"[USERSPACE] Jumping to Ring 3...\n";
    for &c in msg {
        unsafe {
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
    }
    
    unsafe {
        jump_to_ring3(entry_point, user_stack_top);
    }
}

/// Low-level jump to Ring 3 using IRETQ
///
/// IRETQ expects on stack (from low to high):
/// - RIP (return address in userspace)
/// - CS (user code segment with RPL=3)
/// - RFLAGS (with IF set for interrupts)
/// - RSP (user stack pointer)
/// - SS (user data segment with RPL=3)
#[naked]
unsafe extern "C" fn jump_to_ring3(entry: u64, user_stack: u64) -> ! {
    unsafe { naked_asm!(
        // RDI = entry point
        // RSI = user stack
        
        // Debug: print 'I' for IRETQ about to happen
        "push rax",
        "push rdx",
        "mov dx, 0x3F8",
        "mov al, 'I'",
        "out dx, al",
        "mov al, 'R'",
        "out dx, al",
        "mov al, 'E'",
        "out dx, al",
        "mov al, 'T'",
        "out dx, al",
        "mov al, 10",
        "out dx, al",
        "pop rdx",
        "pop rax",
        
        // Push SS (user data segment)
        "mov rax, {user_data}",
        "push rax",
        
        // Push RSP (user stack pointer)
        "push rsi",
        
        // Push RFLAGS with IF (interrupt flag) set
        "pushfq",
        "pop rax",
        "or rax, 0x200",    // Set IF (bit 9)
        "push rax",
        
        // Push CS (user code segment)
        "mov rax, {user_code}",
        "push rax",
        
        // Push RIP (entry point)
        "push rdi",
        
        // Debug: print 'G' for GO
        "mov dx, 0x3F8",
        "mov al, 'G'",
        "out dx, al",
        "mov al, 'O'",
        "out dx, al",
        "mov al, 10",
        "out dx, al",
        
        // Clear all general purpose registers for security
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor rbp, rbp",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        
        // IRETQ to Ring 3!
        "iretq",
        
        user_data = const 0x1B_u64, // USER_DATA_SELECTOR (0x18 | 3)
        user_code = const 0x23_u64, // USER_CODE_SELECTOR (0x20 | 3)
    ); }
}

// =============================================================================
// Userspace Program Builder
// =============================================================================

/// A simple userspace program that uses syscalls
/// This is a minimal "Hello from userspace!" program
///
/// Syscall convention (matching our syscall.rs):
/// - RAX = syscall number
/// - RDI, RSI, RDX, R10, R8, R9 = arguments
/// - SYSCALL instruction
///
/// Available syscalls:
/// - 0: exit(code)
/// - 1: write(fd, buf, len)
/// - 255: debug(value)
#[repr(align(4096))]
pub struct UserspaceProgram {
    code: [u8; 4096],
}

impl UserspaceProgram {
    /// Create a simple "Hello World" userspace program
    pub fn hello_world() -> Self {
        let mut prog = Self { code: [0; 4096] };
        
        // Userspace program that prints "Hello from Ring 3!" and exits
        //
        // Layout:
        //   Offset 0:   Code
        //   Offset 128: Data (message string)
        //
        // Code:
        //   lea rsi, [rip + message]  ; buf = message address
        //   mov rax, 1                ; syscall: write
        //   mov rdi, 1                ; fd = stdout
        //   mov rdx, 20               ; len = 20 bytes
        //   syscall
        //
        //   mov rax, 0                ; syscall: exit
        //   mov rdi, 0                ; exit code = 0 (success)
        //   syscall
        //
        // message: "Hello from Ring 3!\n"
        
        // The message is at offset 128 (0x80)
        // From lea instruction at offset 0, we need to calculate RIP-relative offset
        // lea rsi, [rip + offset] => at execution, RIP points to next instruction
        // lea is 7 bytes, so RIP after lea = base + 7
        // message is at base + 128
        // offset = (base + 128) - (base + 7) = 121 = 0x79
        
        let code: &[u8] = &[
            // lea rsi, [rip + 0x79] (message at offset 128, lea ends at offset 7)
            // Offset from end of instruction: 128 - 7 = 121 = 0x79
            0x48, 0x8D, 0x35, 0x79, 0x00, 0x00, 0x00,  // offset 0-6 (7 bytes)
            
            // mov rax, 1 (write syscall)
            0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00,  // offset 7-13 (7 bytes)
            
            // mov rdi, 1 (fd = stdout)
            0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00,  // offset 14-20 (7 bytes)
            
            // mov rdx, 20 (len)
            0x48, 0xC7, 0xC2, 0x14, 0x00, 0x00, 0x00,  // offset 21-27 (7 bytes)
            
            // syscall
            0x0F, 0x05,                                  // offset 28-29 (2 bytes)
            
            // mov rax, 0 (exit syscall)
            0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00,  // offset 30-36 (7 bytes)
            
            // mov rdi, 0 (exit code = 0)
            0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00,  // offset 37-43 (7 bytes)
            
            // syscall  
            0x0F, 0x05,                                  // offset 44-45 (2 bytes)
            
            // jmp $ (safety loop, offset 46-47)
            0xEB, 0xFE,
        ];
        
        // Copy code to start of buffer
        prog.code[..code.len()].copy_from_slice(code);
        
        // Message at offset 128: "Hello from Ring 3!\n"
        let message = b"Hello from Ring 3!\n\0";
        prog.code[128..128 + message.len()].copy_from_slice(message);
        
        prog
    }
    
    /// Get the entry point address
    pub fn entry_point(&self) -> u64 {
        self.code.as_ptr() as u64
    }
}

/// Static storage for the userspace program
static mut USERSPACE_PROGRAM: UserspaceProgram = UserspaceProgram { code: [0; 4096] };

/// Initialize and get the userspace program entry point
pub fn init_hello_world() -> u64 {
    unsafe {
        USERSPACE_PROGRAM = UserspaceProgram::hello_world();
        USERSPACE_PROGRAM.entry_point()
    }
}

/// Run the userspace hello world program
/// 
/// # Safety
/// This transfers control to Ring 3 and never returns normally.
/// The kernel must have syscall handlers set up.
pub unsafe fn run_hello_world() -> ! {
    let entry = init_hello_world();
    
    unsafe {
        let msg = b"[USERSPACE] Program loaded at ";
        for &c in msg {
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
        
        // Print address in hex
        let hex_chars = b"0123456789abcdef";
        for i in (0..16).rev() {
            let nibble = ((entry >> (i * 4)) & 0xF) as usize;
            let c = hex_chars[nibble];
            asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
        asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n', options(nomem, nostack));
        
        // Make the code page accessible from Ring 3 (USER bit in page tables)
        let code_page_start = entry & !0xFFF;  // Page-align
        super::paging::make_user_accessible(code_page_start, 4096);
        
        // Also make the user stack accessible from Ring 3
        let stack_start = USER_STACK.0.as_ptr() as u64;
        super::paging::make_user_accessible(stack_start, USER_STACK_SIZE);
        
        jump_to_userspace(entry)
    }
}
