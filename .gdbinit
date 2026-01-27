# =============================================================================
# Helix OS - GDB Initialization Script
# =============================================================================
# Debugging configuration for kernel development
# =============================================================================

# -----------------------------------------------------------------------------
# Connection Settings
# -----------------------------------------------------------------------------

# Connect to QEMU's GDB server
set architecture i386:x86-64
target remote localhost:1234

# Load kernel symbols
symbol-file build/output/helix-kernel

# -----------------------------------------------------------------------------
# Display Settings
# -----------------------------------------------------------------------------

# Enable pretty-printing for Rust types
set print pretty on
set print array on
set print array-indexes on
set print elements 100
set print frame-arguments all
set print object on
set print static-members on
set print vtbl on
set print demangle on
set print asm-demangle on
set print sevenbit-strings off

# Rust demangling
set demangle-style gnu-v3

# Pagination off for scripting
set pagination off

# Confirm off for batch operations
set confirm off

# History
set history save on
set history size 10000
set history filename ~/.gdb_history_helix

# Logging
set logging file build/logs/gdb.log
set logging overwrite on

# -----------------------------------------------------------------------------
# Breakpoint Settings
# -----------------------------------------------------------------------------

# Don't stop on SIGINT (Ctrl+C in QEMU)
handle SIGINT nostop pass

# Hardware breakpoints (more reliable for kernel debugging)
set breakpoint always-inserted on

# -----------------------------------------------------------------------------
# Memory Settings
# -----------------------------------------------------------------------------

# Allow access to unmapped memory (kernel may have special mappings)
set mem inaccessible-by-default off

# -----------------------------------------------------------------------------
# Convenience Functions
# -----------------------------------------------------------------------------

# Print current instruction
define current
    x/i $pc
end
document current
    Print the current instruction.
end

# Print stack trace with source
define bt-full
    bt full
end
document bt-full
    Print full backtrace with local variables.
end

# Dump registers
define regs
    info registers
end
document regs
    Print all general-purpose registers.
end

# Dump segment registers
define segs
    info registers cs ss ds es fs gs
end
document segs
    Print segment registers.
end

# Dump control registers
define cregs
    monitor info registers
end
document cregs
    Print all registers via QEMU monitor.
end

# Print page tables
define pagetables
    monitor info tlb
end
document pagetables
    Print TLB entries via QEMU monitor.
end

# Print memory map
define memmap
    monitor info mem
end
document memmap
    Print memory mappings via QEMU monitor.
end

# Print interrupt state
define irqs
    monitor info irq
end
document irqs
    Print IRQ state via QEMU monitor.
end

# Step to next instruction (ni with register dump)
define nir
    ni
    regs
    current
end
document nir
    Single step instruction and show registers.
end

# Step into (si with register dump)
define sir
    si
    regs
    current
end
document sir
    Single step into and show registers.
end

# Continue until next interrupt/exception
define ci
    catch signal SIGTRAP
    continue
end
document ci
    Continue until next trap.
end

# Print kernel stack
define kstack
    set $sp = $rsp
    while $sp < $rsp + 0x1000
        x/gx $sp
        set $sp = $sp + 8
    end
end
document kstack
    Print kernel stack contents.
end

# Disassemble current function
define disf
    disassemble
end
document disf
    Disassemble current function.
end

# Disassemble N instructions from current PC
define disn
    if $argc == 1
        x/$arg0i $pc
    else
        x/20i $pc
    end
end
document disn
    Disassemble N instructions from PC (default 20).
end

# Hexdump memory region
define hexdump
    if $argc == 2
        dump binary memory /tmp/hexdump.bin $arg0 $arg0+$arg1
        shell xxd /tmp/hexdump.bin
    else
        echo Usage: hexdump ADDRESS SIZE\n
    end
end
document hexdump
    Hexdump memory region: hexdump ADDRESS SIZE
end

# Print Rust &str
define rstr
    set $ptr = (char*)($arg0.data_ptr)
    set $len = $arg0.length
    printf "len=%d: \"", $len
    set $i = 0
    while $i < $len
        printf "%c", $ptr[$i]
        set $i = $i + 1
    end
    printf "\"\n"
end
document rstr
    Print Rust &str: rstr VARIABLE
end

# Print Rust Vec
define rvec
    set $ptr = $arg0.buf.ptr.pointer
    set $len = $arg0.len
    set $cap = $arg0.buf.cap
    printf "Vec { len: %d, cap: %d, ptr: %p }\n", $len, $cap, $ptr
    set $i = 0
    while $i < $len
        printf "  [%d]: ", $i
        print $ptr[$i]
        set $i = $i + 1
    end
end
document rvec
    Print Rust Vec: rvec VARIABLE
end

# -----------------------------------------------------------------------------
# Kernel-Specific Breakpoints
# -----------------------------------------------------------------------------

# Break on kernel panic
define bp-panic
    break rust_begin_unwind
    break core::panicking::panic
    break core::panicking::panic_fmt
end
document bp-panic
    Set breakpoints on Rust panic functions.
end

# Break on page fault
define bp-pf
    break page_fault_handler
end
document bp-pf
    Set breakpoint on page fault handler.
end

# Break on double fault
define bp-df
    break double_fault_handler
end
document bp-df
    Set breakpoint on double fault handler.
end

# Break on all exceptions
define bp-exceptions
    bp-panic
    bp-pf
    bp-df
    break general_protection_fault_handler
    break invalid_opcode_handler
end
document bp-exceptions
    Set breakpoints on all exception handlers.
end

# Break on kernel entry
define bp-entry
    break kernel_main
    break _start
end
document bp-entry
    Set breakpoints on kernel entry points.
end

# -----------------------------------------------------------------------------
# QEMU Monitor Commands
# -----------------------------------------------------------------------------

# Access QEMU monitor
define qemu
    monitor $arg0
end
document qemu
    Send command to QEMU monitor: qemu COMMAND
end

# Quit QEMU
define qemu-quit
    monitor quit
end
document qemu-quit
    Quit QEMU.
end

# Reset QEMU
define qemu-reset
    monitor system_reset
end
document qemu-reset
    Reset QEMU system.
end

# Take QEMU screenshot
define qemu-screenshot
    monitor screendump /tmp/helix-screenshot.ppm
    echo Screenshot saved to /tmp/helix-screenshot.ppm\n
end
document qemu-screenshot
    Take QEMU screenshot.
end

# -----------------------------------------------------------------------------
# Initialization
# -----------------------------------------------------------------------------

# Set common breakpoints
bp-entry

# Show welcome message
echo \n
echo ========================================\n
echo   Helix OS Kernel Debugger\n
echo ========================================\n
echo \n
echo Commands:\n
echo   regs        - Print registers\n
echo   current     - Print current instruction\n
echo   disn N      - Disassemble N instructions\n
echo   bp-panic    - Break on panic\n
echo   bp-exceptions - Break on all exceptions\n
echo   qemu CMD    - QEMU monitor command\n
echo \n
echo Breakpoints set at: kernel_main, _start\n
echo \n
