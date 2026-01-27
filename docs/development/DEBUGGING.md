# Debugging Helix OS

<div align="center">

🔍 **Complete Debugging Guide**

*Tools and techniques for kernel debugging*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [Serial Console](#2-serial-console)
3. [QEMU Debugging](#3-qemu-debugging)
4. [GDB Integration](#4-gdb-integration)
5. [Crash Analysis](#5-crash-analysis)
6. [Memory Debugging](#6-memory-debugging)
7. [Performance Profiling](#7-performance-profiling)
8. [Common Issues](#8-common-issues)
9. [Debug Macros](#9-debug-macros)
10. [Tooling](#10-tooling)

---

## 1. Overview

### 1.1 Debugging Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       DEBUGGING ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        HOST SYSTEM                                  │   │
│  │                                                                     │   │
│  │  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐       │   │
│  │  │     GDB       │    │  Serial Term  │    │   Log Files   │       │   │
│  │  │  (debugger)   │    │   (minicom)   │    │               │       │   │
│  │  └───────┬───────┘    └───────┬───────┘    └───────────────┘       │   │
│  │          │                    │                    ▲                │   │
│  │          │ TCP:1234           │ stdio/pty          │                │   │
│  │          │                    │                    │                │   │
│  └──────────┼────────────────────┼────────────────────┼────────────────┘   │
│             │                    │                    │                     │
│             ▼                    ▼                    │                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          QEMU                                       │   │
│  │                                                                     │   │
│  │  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐       │   │
│  │  │  GDB Server   │    │  Serial Port  │    │   Log Output  │       │   │
│  │  │  (gdbstub)    │    │   COM1        │    │               │       │   │
│  │  └───────┬───────┘    └───────┬───────┘    └───────┬───────┘       │   │
│  │          │                    │                    │                │   │
│  └──────────┼────────────────────┼────────────────────┼────────────────┘   │
│             │                    │                    │                     │
│             ▼                    ▼                    ▼                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       HELIX KERNEL                                  │   │
│  │                                                                     │   │
│  │  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐       │   │
│  │  │  Breakpoints  │    │ serial_print! │    │  Debug Logs   │       │   │
│  │  │  Watchpoints  │    │ serial_println│    │               │       │   │
│  │  └───────────────┘    └───────────────┘    └───────────────┘       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Debug Build Configuration

```toml
# Cargo.toml debug profile
[profile.dev]
opt-level = 0
debug = true
debug-assertions = true
overflow-checks = true
lto = false
panic = "abort"

# Release with debug info
[profile.release-with-debug]
inherits = "release"
debug = true

# Features for debugging
[features]
debug = ["debug-console", "verbose-logging"]
debug-console = []
verbose-logging = []
trace-scheduling = []
trace-memory = []
trace-interrupts = []
```

---

## 2. Serial Console

### 2.1 Serial Output Implementation

```rust
// core/src/debug/console.rs

use spin::Mutex;
use core::fmt::{self, Write};

/// Serial port base address (COM1)
const SERIAL_PORT: u16 = 0x3F8;

/// Global serial console
pub static SERIAL: Mutex<SerialConsole> = Mutex::new(SerialConsole::new());

/// Serial console for debug output
pub struct SerialConsole {
    initialized: bool,
}

impl SerialConsole {
    /// Create new serial console
    pub const fn new() -> Self {
        Self { initialized: false }
    }
    
    /// Initialize the serial port
    pub fn init(&mut self) {
        // Disable interrupts
        outb(SERIAL_PORT + 1, 0x00);
        
        // Enable DLAB (set baud rate divisor)
        outb(SERIAL_PORT + 3, 0x80);
        
        // Set divisor to 1 (115200 baud)
        outb(SERIAL_PORT + 0, 0x01);  // Low byte
        outb(SERIAL_PORT + 1, 0x00);  // High byte
        
        // 8 bits, no parity, one stop bit
        outb(SERIAL_PORT + 3, 0x03);
        
        // Enable FIFO, clear buffers, 14-byte threshold
        outb(SERIAL_PORT + 2, 0xC7);
        
        // Enable IRQs, RTS/DSR set
        outb(SERIAL_PORT + 4, 0x0B);
        
        self.initialized = true;
    }
    
    /// Check if transmit buffer is empty
    fn is_transmit_empty(&self) -> bool {
        inb(SERIAL_PORT + 5) & 0x20 != 0
    }
    
    /// Write a byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        if !self.initialized {
            self.init();
        }
        
        // Wait for transmit buffer to be empty
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        
        outb(SERIAL_PORT, byte);
    }
    
    /// Write a string to the serial port
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl Write for SerialConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// Port I/O functions
fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
        );
    }
}

fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
        );
    }
    value
}
```

### 2.2 Serial Print Macros

```rust
// core/src/debug/mod.rs

/// Print to serial console
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::debug::_serial_print(format_args!($($arg)*))
    };
}

/// Print to serial console with newline
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

/// Debug print (only in debug builds)
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        $crate::serial_println!("[DEBUG] {}", format_args!($($arg)*))
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_print {
    ($($arg:tt)*) => {};
}

/// Trace macro for function entry/exit
#[macro_export]
macro_rules! trace_enter {
    () => {
        #[cfg(feature = "trace")]
        $crate::serial_println!("[TRACE] >>> {}", core::any::type_name::<fn()>());
    };
    ($($arg:tt)*) => {
        #[cfg(feature = "trace")]
        $crate::serial_println!("[TRACE] >>> {} ({})", 
            core::any::type_name::<fn()>(),
            format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace_exit {
    () => {
        #[cfg(feature = "trace")]
        $crate::serial_println!("[TRACE] <<< {}", core::any::type_name::<fn()>());
    };
}

#[doc(hidden)]
pub fn _serial_print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL.lock().write_fmt(args).unwrap();
}
```

### 2.3 Viewing Serial Output

```bash
# Run QEMU with serial output to terminal
qemu-system-x86_64 \
    -kernel build/output/helix-kernel \
    -serial stdio \
    -display none

# Run with serial output to file
qemu-system-x86_64 \
    -kernel build/output/helix-kernel \
    -serial file:serial.log \
    -display none

# Run with serial over TCP
qemu-system-x86_64 \
    -kernel build/output/helix-kernel \
    -serial tcp::4444,server,nowait

# Connect with netcat
nc localhost 4444

# Or use minicom
minicom -D /dev/pts/X  # Replace X with actual pts number
```

---

## 3. QEMU Debugging

### 3.1 QEMU Debug Options

```bash
#!/bin/bash
# scripts/debug_qemu.sh

KERNEL="build/output/helix-kernel"

qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -serial stdio \
    -no-reboot \
    -no-shutdown \
    -d int,cpu_reset \
    -D qemu.log \
    -monitor telnet:127.0.0.1:55555,server,nowait \
    -s -S  # Wait for GDB

# Options explained:
# -serial stdio        : Serial output to terminal
# -no-reboot          : Don't reboot on triple fault
# -no-shutdown        : Don't exit on shutdown
# -d int,cpu_reset    : Log interrupts and CPU resets
# -D qemu.log         : Write debug output to file
# -monitor telnet:... : QEMU monitor on telnet
# -s                  : Open GDB server on port 1234
# -S                  : Wait for GDB before starting
```

### 3.2 QEMU Monitor Commands

```bash
# Connect to QEMU monitor
telnet localhost 55555

# Useful commands:
info registers          # Show CPU registers
info mem               # Show memory mappings
info cpus              # Show CPU info
info irq               # Show IRQ statistics
info pic               # Show PIC state
info lapic             # Show Local APIC state

x/10i $pc              # Disassemble at current PC
x/20x 0x100000         # Examine memory (hex)
x/s 0x200000           # Examine as string

stop                   # Pause execution
cont                   # Continue execution
quit                   # Exit QEMU

# Memory dump
dump-guest-memory dump.elf  # Dump memory to ELF file
pmemsave 0 0x1000000 mem.bin  # Save memory range

# Snapshots
savevm snap1           # Save VM state
loadvm snap1           # Restore VM state
info snapshots         # List snapshots
```

### 3.3 Debugging Triple Faults

```bash
# Run with detailed interrupt logging
qemu-system-x86_64 \
    -kernel build/output/helix-kernel \
    -d int,cpu_reset,guest_errors \
    -D triple_fault.log \
    -no-reboot \
    -no-shutdown

# Analyze the log
cat triple_fault.log | grep -E "(Triple|Exception|Interrupt)"
```

Common triple fault causes:
1. **Invalid GDT**: Corrupted or wrong GDT pointer
2. **Stack overflow**: Stack pointer went outside valid range
3. **Page fault handler faults**: Double fault escalates
4. **Invalid IDT**: Missing or corrupt IDT entries

---

## 4. GDB Integration

### 4.1 GDB Configuration

```gdb
# .gdbinit file for Helix debugging

# Connect to QEMU
target remote localhost:1234

# Load symbols
symbol-file build/output/helix-kernel

# Set architecture
set architecture i386:x86-64

# Don't stop on SIGSEGV (kernel handles page faults)
handle SIGSEGV noprint nostop

# Useful aliases
define regs
    info registers
end

define stack
    x/20gx $rsp
end

define code
    x/10i $rip
end

define pte
    # Print page table entry for address
    set $addr = $arg0
    set $pml4 = $cr3 & ~0xFFF
    set $pml4e = *(unsigned long*)($pml4 + (($addr >> 39) & 0x1FF) * 8)
    printf "PML4E: 0x%lx\n", $pml4e
    # Continue for PDPT, PD, PT...
end

# Break at common points
break kernel_main
break panic
break page_fault_handler
break double_fault_handler

# Continue to kernel_main
continue
```

### 4.2 Starting a Debug Session

```bash
# Terminal 1: Start QEMU with GDB server
./scripts/run_qemu.sh --debug

# Terminal 2: Start GDB
rust-gdb \
    -ex "target remote localhost:1234" \
    -ex "symbol-file build/output/helix-kernel" \
    build/output/helix-kernel
```

### 4.3 Common GDB Commands

```gdb
# ═══════════════════════════════════════════════════════════════════════════
#                          GDB COMMANDS REFERENCE
# ═══════════════════════════════════════════════════════════════════════════

# ─── EXECUTION CONTROL ───────────────────────────────────────────────────────

continue (c)              # Continue execution
step (s)                  # Step one source line
next (n)                  # Step over function calls
stepi (si)               # Step one instruction
nexti (ni)               # Next instruction (over calls)
finish                   # Run until function returns
until <line>             # Run until line reached

# ─── BREAKPOINTS ─────────────────────────────────────────────────────────────

break <function>         # Break at function
break <file>:<line>      # Break at file:line
break *0x100000          # Break at address
watch <expr>             # Break when expr changes
rwatch <expr>            # Break on read
awatch <expr>            # Break on access
info breakpoints         # List breakpoints
delete <n>               # Delete breakpoint n
disable <n>              # Disable breakpoint
enable <n>               # Enable breakpoint

# ─── EXAMINATION ─────────────────────────────────────────────────────────────

print <expr>             # Print expression value
print/x <expr>           # Print in hex
x/Nfu <addr>             # Examine memory
                        # N = count, f = format, u = unit
                        # f: x=hex, d=dec, i=instruction, s=string
                        # u: b=byte, h=half, w=word, g=giant

info registers           # All registers
info registers rax       # Specific register
info frame              # Stack frame info
info locals             # Local variables
info args               # Function arguments

# ─── MEMORY/REGISTERS ────────────────────────────────────────────────────────

set $rax = 0x1234        # Set register
set {int}0x1234 = 100    # Write to memory
dump binary memory f.bin 0x1000 0x2000  # Dump memory

# ─── DISASSEMBLY ─────────────────────────────────────────────────────────────

disassemble              # Disassemble current function
disassemble $rip, +50    # Disassemble from RIP
x/10i $rip               # 10 instructions at RIP
set disassembly-flavor intel  # Intel syntax

# ─── BACKTRACE ───────────────────────────────────────────────────────────────

backtrace (bt)           # Show call stack
frame <n>                # Select frame n
up                       # Go up one frame
down                     # Go down one frame

# ─── SOURCE ──────────────────────────────────────────────────────────────────

list                     # Show source
list <function>          # Show function source
list <file>:<line>       # Show specific location
```

### 4.4 Advanced GDB Techniques

```gdb
# ─── CONDITIONAL BREAKPOINTS ─────────────────────────────────────────────────

# Break only when condition is true
break page_fault_handler if fault_addr == 0x0

# Break after N hits
break scheduler::schedule
ignore 1 100  # Ignore first 100 hits

# ─── CATCHPOINTS ─────────────────────────────────────────────────────────────

# Catch system calls (if applicable)
catch syscall

# ─── SCRIPTING ───────────────────────────────────────────────────────────────

# Define a command
define trace_alloc
    break allocate
    commands
        silent
        printf "alloc: size=%d, ptr=%p\n", size, $rax
        continue
    end
end

# Python scripting
python
class PrintTaskCmd(gdb.Command):
    def __init__(self):
        super().__init__("print_task", gdb.COMMAND_USER)
    
    def invoke(self, arg, from_tty):
        task = gdb.parse_and_eval(arg)
        print(f"Task: {task['name']}, State: {task['state']}")

PrintTaskCmd()
end

# ─── REVERSE DEBUGGING ───────────────────────────────────────────────────────

# (Requires QEMU with reverse debugging support)
record                   # Start recording
reverse-continue (rc)    # Run backwards
reverse-step (rs)        # Step backwards
reverse-next (rn)        # Next backwards
```

---

## 5. Crash Analysis

### 5.1 Panic Handler

```rust
// core/src/orchestrator/panic_handler.rs

use core::panic::PanicInfo;

/// Custom panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts to prevent further issues
    unsafe { core::arch::asm!("cli") };
    
    serial_println!("\n");
    serial_println!("╔══════════════════════════════════════════════════════════════╗");
    serial_println!("║                     KERNEL PANIC                             ║");
    serial_println!("╚══════════════════════════════════════════════════════════════╝");
    
    // Print panic message
    if let Some(message) = info.message() {
        serial_println!("Message: {}", message);
    }
    
    // Print location
    if let Some(location) = info.location() {
        serial_println!("Location: {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column());
    }
    
    // Print stack trace
    serial_println!("\nStack trace:");
    print_stack_trace();
    
    // Print registers
    serial_println!("\nRegisters:");
    print_registers();
    
    // Halt
    serial_println!("\nSystem halted. Debug with QEMU monitor or GDB.");
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

/// Print stack trace
fn print_stack_trace() {
    let mut rbp: u64;
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp);
    }
    
    serial_println!("  Frame chain:");
    for i in 0..20 {
        if rbp == 0 || rbp < 0x1000 {
            break;
        }
        
        // Read return address
        let ret_addr = unsafe { *((rbp + 8) as *const u64) };
        serial_println!("    #{}: 0x{:016x}", i, ret_addr);
        
        // Move to next frame
        rbp = unsafe { *(rbp as *const u64) };
    }
}

/// Print CPU registers
fn print_registers() {
    let rax: u64;
    let rbx: u64;
    let rcx: u64;
    let rdx: u64;
    let rsi: u64;
    let rdi: u64;
    let rsp: u64;
    let rbp: u64;
    let rip: u64;
    
    unsafe {
        core::arch::asm!(
            "mov {rax}, rax",
            "mov {rbx}, rbx",
            "mov {rcx}, rcx",
            "mov {rdx}, rdx",
            "mov {rsi}, rsi",
            "mov {rdi}, rdi",
            "mov {rsp}, rsp",
            "mov {rbp}, rbp",
            "lea {rip}, [rip]",
            rax = out(reg) rax,
            rbx = out(reg) rbx,
            rcx = out(reg) rcx,
            rdx = out(reg) rdx,
            rsi = out(reg) rsi,
            rdi = out(reg) rdi,
            rsp = out(reg) rsp,
            rbp = out(reg) rbp,
            rip = out(reg) rip,
        );
    }
    
    serial_println!("  RAX: 0x{:016x}  RBX: 0x{:016x}", rax, rbx);
    serial_println!("  RCX: 0x{:016x}  RDX: 0x{:016x}", rcx, rdx);
    serial_println!("  RSI: 0x{:016x}  RDI: 0x{:016x}", rsi, rdi);
    serial_println!("  RSP: 0x{:016x}  RBP: 0x{:016x}", rsp, rbp);
    serial_println!("  RIP: 0x{:016x}", rip);
}
```

### 5.2 Exception Handlers with Debug Info

```rust
// core/src/interrupts/exceptions.rs

/// Page fault handler with debug output
pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let fault_addr: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) fault_addr);
    }
    
    serial_println!("\n╔══════════════════════════════════════════════════════════════╗");
    serial_println!("║                      PAGE FAULT                              ║");
    serial_println!("╚══════════════════════════════════════════════════════════════╝");
    serial_println!("Fault Address:  0x{:016x}", fault_addr);
    serial_println!("Error Code:     {:?}", error_code);
    serial_println!();
    serial_println!("Error Details:");
    serial_println!("  Present:      {}", error_code.contains(PageFaultErrorCode::PRESENT));
    serial_println!("  Write:        {}", error_code.contains(PageFaultErrorCode::WRITE));
    serial_println!("  User Mode:    {}", error_code.contains(PageFaultErrorCode::USER));
    serial_println!("  Reserved:     {}", error_code.contains(PageFaultErrorCode::RESERVED));
    serial_println!("  Instr Fetch:  {}", error_code.contains(PageFaultErrorCode::INSTRUCTION));
    serial_println!();
    serial_println!("Stack Frame:");
    serial_println!("  RIP: 0x{:016x}", stack_frame.instruction_pointer.as_u64());
    serial_println!("  RSP: 0x{:016x}", stack_frame.stack_pointer.as_u64());
    serial_println!("  CS:  0x{:04x}", stack_frame.code_segment);
    serial_println!("  SS:  0x{:04x}", stack_frame.stack_segment);
    serial_println!("  RFLAGS: 0x{:016x}", stack_frame.cpu_flags);
    
    // Check if this is recoverable
    if can_recover_page_fault(fault_addr, &error_code) {
        // Handle recoverable fault (e.g., lazy allocation)
        handle_page_fault(fault_addr, &error_code);
        return;
    }
    
    // Unrecoverable - panic
    panic!("Unrecoverable page fault at 0x{:x}", fault_addr);
}

bitflags! {
    pub struct PageFaultErrorCode: u64 {
        const PRESENT = 1 << 0;
        const WRITE = 1 << 1;
        const USER = 1 << 2;
        const RESERVED = 1 << 3;
        const INSTRUCTION = 1 << 4;
    }
}
```

---

## 6. Memory Debugging

### 6.1 Memory Dump Utilities

```rust
/// Dump memory in hex format
pub fn hexdump(addr: usize, len: usize) {
    serial_println!("Memory dump at 0x{:x} ({} bytes):", addr, len);
    
    let ptr = addr as *const u8;
    
    for offset in (0..len).step_by(16) {
        // Print address
        serial_print!("{:08x}:  ", addr + offset);
        
        // Print hex bytes
        for i in 0..16 {
            if offset + i < len {
                let byte = unsafe { *ptr.add(offset + i) };
                serial_print!("{:02x} ", byte);
            } else {
                serial_print!("   ");
            }
            if i == 7 {
                serial_print!(" ");
            }
        }
        
        serial_print!(" |");
        
        // Print ASCII
        for i in 0..16 {
            if offset + i < len {
                let byte = unsafe { *ptr.add(offset + i) };
                if byte >= 0x20 && byte < 0x7F {
                    serial_print!("{}", byte as char);
                } else {
                    serial_print!(".");
                }
            }
        }
        
        serial_println!("|");
    }
}

/// Dump page tables for an address
pub fn dump_page_tables(virt_addr: u64) {
    serial_println!("Page table walk for 0x{:016x}:", virt_addr);
    
    let cr3: u64;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) cr3); }
    
    let pml4_base = cr3 & !0xFFF;
    let pml4_idx = (virt_addr >> 39) & 0x1FF;
    let pdpt_idx = (virt_addr >> 30) & 0x1FF;
    let pd_idx = (virt_addr >> 21) & 0x1FF;
    let pt_idx = (virt_addr >> 12) & 0x1FF;
    let offset = virt_addr & 0xFFF;
    
    serial_println!("  Indices: PML4={}, PDPT={}, PD={}, PT={}, Offset=0x{:x}",
        pml4_idx, pdpt_idx, pd_idx, pt_idx, offset);
    
    // Read PML4E
    let pml4e = unsafe { *((pml4_base + pml4_idx * 8) as *const u64) };
    serial_println!("  PML4E: 0x{:016x} (P={}, RW={}, US={})",
        pml4e,
        pml4e & 1,
        (pml4e >> 1) & 1,
        (pml4e >> 2) & 1);
    
    if pml4e & 1 == 0 {
        serial_println!("  -> NOT PRESENT");
        return;
    }
    
    // Continue with PDPT, PD, PT...
}
```

### 6.2 Allocator Debugging

```rust
/// Allocator with debug tracking
pub struct DebugAllocator<A: Allocator> {
    inner: A,
    allocations: Mutex<HashMap<usize, AllocationInfo>>,
    stats: AtomicStats,
}

struct AllocationInfo {
    size: usize,
    align: usize,
    location: &'static str,
    time: u64,
}

impl<A: Allocator> DebugAllocator<A> {
    /// Dump all active allocations
    pub fn dump_allocations(&self) {
        let allocs = self.allocations.lock();
        
        serial_println!("Active allocations: {}", allocs.len());
        serial_println!("{:<18} {:>10} {:>6} {}", "Address", "Size", "Align", "Location");
        serial_println!("{:-<60}", "");
        
        for (addr, info) in allocs.iter() {
            serial_println!("0x{:016x} {:>10} {:>6} {}", 
                addr, info.size, info.align, info.location);
        }
        
        serial_println!();
        serial_println!("Statistics:");
        serial_println!("  Total allocated: {} bytes", self.stats.total_allocated.load(Ordering::Relaxed));
        serial_println!("  Total freed: {} bytes", self.stats.total_freed.load(Ordering::Relaxed));
        serial_println!("  Peak usage: {} bytes", self.stats.peak_usage.load(Ordering::Relaxed));
    }
    
    /// Check for memory leaks
    pub fn check_leaks(&self) -> bool {
        let allocs = self.allocations.lock();
        if !allocs.is_empty() {
            serial_println!("WARNING: {} allocations not freed!", allocs.len());
            return true;
        }
        false
    }
}
```

---

## 7. Performance Profiling

### 7.1 CPU Cycle Counting

```rust
/// Read Time Stamp Counter
#[inline(always)]
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

/// Measure function execution time
#[macro_export]
macro_rules! measure {
    ($name:expr, $block:expr) => {{
        let start = $crate::debug::rdtsc();
        let result = $block;
        let end = $crate::debug::rdtsc();
        serial_println!("[PERF] {}: {} cycles", $name, end - start);
        result
    }};
}

// Usage:
let result = measure!("page_allocation", {
    allocator.allocate_page()
});
```

### 7.2 Profiling Infrastructure

```rust
/// Simple profiler
pub struct Profiler {
    samples: Mutex<HashMap<&'static str, ProfileData>>,
}

struct ProfileData {
    call_count: u64,
    total_cycles: u64,
    min_cycles: u64,
    max_cycles: u64,
}

impl Profiler {
    pub fn record(&self, name: &'static str, cycles: u64) {
        let mut samples = self.samples.lock();
        let data = samples.entry(name).or_insert(ProfileData {
            call_count: 0,
            total_cycles: 0,
            min_cycles: u64::MAX,
            max_cycles: 0,
        });
        
        data.call_count += 1;
        data.total_cycles += cycles;
        data.min_cycles = data.min_cycles.min(cycles);
        data.max_cycles = data.max_cycles.max(cycles);
    }
    
    pub fn report(&self) {
        let samples = self.samples.lock();
        
        serial_println!("\n╔══════════════════════════════════════════════════════════════╗");
        serial_println!("║                    PROFILER REPORT                           ║");
        serial_println!("╚══════════════════════════════════════════════════════════════╝");
        serial_println!("{:<30} {:>10} {:>12} {:>12} {:>12}", 
            "Function", "Calls", "Total", "Avg", "Max");
        serial_println!("{:-<80}", "");
        
        let mut entries: Vec<_> = samples.iter().collect();
        entries.sort_by(|a, b| b.1.total_cycles.cmp(&a.1.total_cycles));
        
        for (name, data) in entries {
            let avg = data.total_cycles / data.call_count.max(1);
            serial_println!("{:<30} {:>10} {:>12} {:>12} {:>12}",
                name,
                data.call_count,
                data.total_cycles,
                avg,
                data.max_cycles);
        }
    }
}

static PROFILER: Profiler = Profiler::new();

/// Profile a block of code
#[macro_export]
macro_rules! profile {
    ($name:expr, $block:expr) => {{
        let start = $crate::debug::rdtsc();
        let result = $block;
        let cycles = $crate::debug::rdtsc() - start;
        $crate::debug::PROFILER.record($name, cycles);
        result
    }};
}
```

---

## 8. Common Issues

### 8.1 Issue Reference Table

| Issue | Symptoms | Common Causes | Solution |
|-------|----------|---------------|----------|
| Triple Fault | QEMU resets immediately | Bad GDT/IDT, stack overflow | Check GDT/IDT, increase stack size |
| Page Fault | Crash with fault info | Null pointer, unmapped memory | Check address, verify mapping |
| Double Fault | No handler output | Stack overflow in handler | Use separate IST stack |
| No Serial Output | Empty console | Serial not initialized | Check serial init order |
| Hang on Boot | Stuck after GRUB | Infinite loop, missing `hlt` | Add debug prints, check flow |
| Interrupt Storm | 100% CPU, no progress | Missing EOI, bad PIC config | Send EOI, check PIC setup |
| Memory Corruption | Random crashes | Buffer overflow, use-after-free | Use debug allocator |

### 8.2 Debugging Checklist

```markdown
## Pre-Debug Checklist

### Build
- [ ] Clean build (`cargo clean && ./scripts/build.sh`)
- [ ] Debug symbols included (`debug = true`)
- [ ] Optimization disabled for debugging (`opt-level = 0`)

### Environment
- [ ] QEMU version is recent
- [ ] GDB/rust-gdb available
- [ ] Serial output working

### Issue Reproduction
- [ ] Can reproduce consistently
- [ ] Minimal test case identified
- [ ] Previous working commit known

## During Debug

### Information Gathering
- [ ] Serial output captured
- [ ] QEMU logs saved (`-d` option)
- [ ] Stack trace available
- [ ] Register values recorded

### Analysis
- [ ] Fault address meaningful?
- [ ] Stack trace makes sense?
- [ ] Error code decoded?
- [ ] Recent changes reviewed?

## Post-Debug

### Verification
- [ ] Fix actually resolves issue
- [ ] No regression introduced
- [ ] Test added for issue
- [ ] Documentation updated
```

### 8.3 Quick Debug Commands

```bash
# Quick serial debug
qemu-system-x86_64 -kernel build/output/helix-kernel -serial stdio -no-reboot 2>&1 | tee boot.log

# Quick GDB attach
rust-gdb -ex "target remote :1234" -ex "symbol-file build/output/helix-kernel"

# Watch QEMU logs
qemu-system-x86_64 -kernel build/output/helix-kernel -d int,guest_errors 2>&1 | grep -v "^$"

# Memory dump
echo "pmemsave 0 0x10000000 memory.bin" | nc localhost 55555
```

---

## 9. Debug Macros

### 9.1 Assertion Macros

```rust
/// Debug assertion with message
#[macro_export]
macro_rules! debug_assert_msg {
    ($cond:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        if !$cond {
            panic!("Assertion failed: {}\n  {}", stringify!($cond), format_args!($($arg)*));
        }
    };
}

/// Unreachable code marker
#[macro_export]
macro_rules! debug_unreachable {
    () => {
        #[cfg(debug_assertions)]
        panic!("Entered unreachable code");
        
        #[cfg(not(debug_assertions))]
        unsafe { core::hint::unreachable_unchecked() }
    };
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        panic!("Unreachable: {}", format_args!($($arg)*));
        
        #[cfg(not(debug_assertions))]
        unsafe { core::hint::unreachable_unchecked() }
    };
}

/// Log and return error
#[macro_export]
macro_rules! bail {
    ($err:expr) => {{
        let e = $err;
        serial_println!("[ERROR] {}:{}: {:?}", file!(), line!(), e);
        return Err(e.into());
    }};
}
```

### 9.2 Tracing Macros

```rust
/// Trace function entry/exit with timing
#[macro_export]
macro_rules! trace_fn {
    ($($arg:tt)*) => {
        let _guard = $crate::debug::TraceGuard::new(
            concat!(module_path!(), "::", stringify!($($arg)*))
        );
    };
}

pub struct TraceGuard {
    name: &'static str,
    start: u64,
}

impl TraceGuard {
    pub fn new(name: &'static str) -> Self {
        serial_println!("[TRACE] >>> {}", name);
        Self {
            name,
            start: rdtsc(),
        }
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        let elapsed = rdtsc() - self.start;
        serial_println!("[TRACE] <<< {} ({} cycles)", self.name, elapsed);
    }
}
```

---

## 10. Tooling

### 10.1 Useful Scripts

```bash
#!/bin/bash
# scripts/debug.sh - Complete debug setup

set -e

KERNEL="build/output/helix-kernel"

# Parse arguments
DEBUG_MODE="gdb"
while [[ $# -gt 0 ]]; do
    case $1 in
        --serial)
            DEBUG_MODE="serial"
            shift
            ;;
        --monitor)
            DEBUG_MODE="monitor"
            shift
            ;;
        --gdb)
            DEBUG_MODE="gdb"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

case $DEBUG_MODE in
    serial)
        echo "Starting with serial output..."
        qemu-system-x86_64 \
            -kernel "$KERNEL" \
            -serial stdio \
            -no-reboot \
            -no-shutdown
        ;;
    
    monitor)
        echo "Starting with QEMU monitor..."
        echo "Connect with: telnet localhost 55555"
        qemu-system-x86_64 \
            -kernel "$KERNEL" \
            -serial stdio \
            -monitor telnet:127.0.0.1:55555,server,nowait \
            -no-reboot
        ;;
    
    gdb)
        echo "Starting with GDB server..."
        echo "In another terminal, run: rust-gdb -x .gdbinit"
        qemu-system-x86_64 \
            -kernel "$KERNEL" \
            -serial stdio \
            -s -S \
            -no-reboot
        ;;
esac
```

### 10.2 VS Code Debug Configuration

```json
// .vscode/launch.json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "Debug Helix Kernel",
            "type": "cppdbg",
            "request": "launch",
            "program": "${workspaceFolder}/build/output/helix-kernel",
            "miDebuggerServerAddress": "localhost:1234",
            "miDebuggerPath": "rust-gdb",
            "cwd": "${workspaceFolder}",
            "preLaunchTask": "Start QEMU Debug",
            "setupCommands": [
                {
                    "text": "set architecture i386:x86-64"
                },
                {
                    "text": "file ${workspaceFolder}/build/output/helix-kernel"
                }
            ]
        }
    ]
}
```

```json
// .vscode/tasks.json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Build Kernel",
            "type": "shell",
            "command": "./scripts/build.sh",
            "group": {
                "kind": "build",
                "isDefault": true
            }
        },
        {
            "label": "Start QEMU Debug",
            "type": "shell",
            "command": "qemu-system-x86_64 -kernel build/output/helix-kernel -serial stdio -s -S -no-reboot &",
            "isBackground": true,
            "problemMatcher": []
        }
    ]
}
```

---

## Summary

Effective kernel debugging requires:

1. **Serial Output**: Primary debug channel
2. **QEMU Tools**: Monitor, logging, snapshots
3. **GDB Integration**: Breakpoints, watchpoints, stepping
4. **Crash Analysis**: Panic handlers, stack traces
5. **Memory Debugging**: Dumps, tracking, verification
6. **Performance Profiling**: Cycle counting, profilers
7. **Good Practices**: Assertions, tracing, tooling

Remember: **Print early, print often!**

---

<div align="center">

🔍 *Debugging is twice as hard as writing code. If you write code as cleverly as possible, you are not smart enough to debug it.* 🔍

</div>
