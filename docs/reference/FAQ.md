# Helix OS Frequently Asked Questions

<div align="center">

❓ **Your Questions Answered**

*Common questions about Helix OS development, design, and usage*

</div>

---

## Table of Contents

1. [General Questions](#general-questions)
2. [Getting Started](#getting-started)
3. [Architecture & Design](#architecture--design)
4. [Development](#development)
5. [Building & Running](#building--running)
6. [Debugging](#debugging)
7. [Performance](#performance)
8. [Contributing](#contributing)
9. [Comparisons](#comparisons)
10. [Troubleshooting](#troubleshooting)

---

## General Questions

### What is Helix OS?

Helix OS is a modular, research-oriented operating system kernel written entirely in Rust. It explores modern OS design principles including:

- **Intent-Based Scheduling**: Tasks declare their behavioral requirements rather than raw priorities
- **Hot-Reloadable Components**: Update kernel modules without rebooting
- **Capability-Based Security**: Fine-grained access control through capabilities
- **Modern Filesystem**: B-tree indexed, journaled, CoW filesystem

Helix is designed for learning, experimentation, and exploring novel OS concepts.

### Why another operating system?

Helix addresses several goals:

1. **Education**: A clear, well-documented codebase for learning OS internals
2. **Research**: A platform for experimenting with scheduling, memory, and FS ideas
3. **Rust Showcase**: Demonstrating Rust's viability for systems programming
4. **Modern Design**: Applying decades of OS research to a clean-slate design

### What platforms does Helix support?

Currently, Helix supports:

- **x86_64**: Primary supported architecture
- **QEMU**: Development and testing environment

Future planned support:
- ARM64 (AArch64)
- RISC-V
- Real hardware (various x86_64 systems)

### Is Helix ready for production use?

**No.** Helix is a research and educational project. It lacks:

- Complete device driver support
- Hardened security implementation
- Extensive testing
- Production workload validation

Use Helix for learning and experimentation, not production systems.

### What license is Helix under?

Helix is released under [LICENSE]. This allows:
- Personal use
- Educational use
- Modification and redistribution
- Commercial use (with attribution)

See the LICENSE file in the repository for full terms.

---

## Getting Started

### What are the system requirements?

**Development Machine:**
- OS: Linux (Ubuntu 22.04+, Arch, Fedora), macOS (Intel/ARM)
- RAM: 8 GB minimum, 16 GB recommended
- Disk: 2 GB free space
- CPU: x86_64 with virtualization support

**Software:**
- Rust nightly (specified in `rust-toolchain.toml`)
- QEMU with x86_64 support
- GRUB 2 (for ISO creation)
- GDB (optional, for debugging)

### How do I build Helix?

```bash
# Clone the repository
git clone https://github.com/helix-os/helix.git
cd helix

# Install Rust targets
rustup target add x86_64-unknown-none

# Build the kernel
./scripts/build.sh

# Run in QEMU
./scripts/run_qemu.sh
```

### I get a Rust version error. What do I do?

Helix requires a specific nightly Rust version. Install it:

```bash
# The rust-toolchain.toml handles this automatically
cd helix
rustup show  # Should install the required version

# Or manually:
rustup install nightly
rustup override set nightly
```

### How do I set up my development environment?

1. **Install prerequisites:**
   ```bash
   # Ubuntu/Debian
   sudo apt install qemu-system-x86 grub-pc-bin xorriso gdb
   
   # Arch
   sudo pacman -S qemu grub xorriso gdb
   
   # macOS
   brew install qemu xorriso x86_64-elf-gdb
   ```

2. **Configure VS Code (optional):**
   - Install rust-analyzer extension
   - Configure debugging with provided launch.json

3. **Build and test:**
   ```bash
   ./scripts/build.sh
   ./scripts/test.sh
   ```

---

## Architecture & Design

### Why is Helix written in Rust?

Rust provides unique advantages for kernel development:

1. **Memory Safety**: Prevents buffer overflows, use-after-free, null dereference
2. **Zero-Cost Abstractions**: High-level code compiles to efficient machine code
3. **No Runtime**: Minimal runtime requirements suitable for `#![no_std]`
4. **Strong Type System**: Catches errors at compile time
5. **Modern Tooling**: Cargo, rustfmt, clippy, documentation

The kernel uses `unsafe` sparingly and only where absolutely necessary.

### What is the Differentiated Intent Scheduler (DIS)?

DIS is Helix's novel scheduling system that moves beyond simple priority-based scheduling:

```
Traditional:          DIS:
┌─────────────┐      ┌────────────────────┐
│ Priority: 10│      │ Intent: Realtime   │
│ Nice: -5    │      │ Deadline: 10ms     │
└─────────────┘      │ CPU Affinity: 0-3  │
                     └────────────────────┘
```

**Intent Classes:**
- **Realtime**: Hard timing requirements (audio, control systems)
- **Interactive**: User-facing, low latency preferred
- **Batch**: CPU-intensive, throughput-oriented
- **Background**: Best-effort, runs when idle

The scheduler uses this information to optimize scheduling decisions.

### How does hot-reloading work?

Hot-reload enables updating modules without reboot:

```
┌─────────────────────────────────────────────┐
│                Hot-Reload Flow              │
├─────────────────────────────────────────────┤
│                                             │
│  1. Load new module version                 │
│              ↓                              │
│  2. Pause old module (quiesce)              │
│              ↓                              │
│  3. Transfer state                          │
│              ↓                              │
│  4. Activate new module                     │
│              ↓                              │
│  5. Unload old module                       │
│                                             │
└─────────────────────────────────────────────┘
```

If the new module fails, the system rolls back to the old version.

### Why B-trees in the filesystem?

HelixFS uses B-trees for several advantages:

1. **Disk Optimization**: B-trees minimize disk seeks with high fanout
2. **Consistent Performance**: O(log n) operations regardless of data size
3. **Efficient Range Queries**: Directory listing, extent lookup
4. **Self-Balancing**: No manual rebalancing needed
5. **Cache-Friendly**: Nodes fit in memory pages

Comparison with other structures:

| Structure | Insert | Search | Range Query | Disk Seeks |
|-----------|--------|--------|-------------|------------|
| B-tree    | O(log n) | O(log n) | O(log n + k) | Low |
| Hash Table | O(1) | O(1) | O(n) | Variable |
| Binary Tree | O(log n) | O(log n) | O(n) | High |

### What's the difference between Helix and a microkernel?

```
Microkernel:                    Helix:
┌───────────────────┐          ┌───────────────────┐
│    User Space     │          │    User Space     │
│  ┌─────┐ ┌─────┐  │          │                   │
│  │ FS  │ │ Net │  │          │                   │
│  └─────┘ └─────┘  │          │                   │
├───────────────────┤          ├───────────────────┤
│      Kernel       │          │      Kernel       │
│  ┌─────────────┐  │          │  ┌──────┐ ┌────┐  │
│  │ IPC + Sched │  │          │  │  FS  │ │Net │  │
│  └─────────────┘  │          │  └──────┘ └────┘  │
└───────────────────┘          │  ┌─────────────┐  │
                               │  │ Core + Sched│  │
                               │  └─────────────┘  │
                               └───────────────────┘
```

Helix is a **modular monolithic** kernel:
- Subsystems run in kernel space (like monolithic)
- Clear interfaces between components (like microkernel)
- Hot-reloadable modules provide flexibility
- Lower IPC overhead than microkernel

---

## Development

### How is the codebase organized?

```
helix/
├── boot/           # Boot assembly (32→64 bit transition)
├── core/           # Kernel core (IPC, interrupts, syscalls)
├── hal/            # Hardware abstraction layer
├── fs/             # HelixFS filesystem
├── subsystems/
│   ├── dis/        # Differentiated Intent Scheduler
│   ├── memory/     # Memory management
│   ├── execution/  # Task execution
│   └── userspace/  # User space support
├── modules/        # Module system
├── profiles/       # Build configurations
└── scripts/        # Build and utility scripts
```

Each directory is a Rust crate with defined interfaces.

### How do I add a new module?

1. **Create module crate:**
   ```bash
   ./scripts/module_add.sh my_module
   ```

2. **Implement the Module trait:**
   ```rust
   use helix_modules::prelude::*;
   
   pub struct MyModule;
   
   impl Module for MyModule {
       fn name(&self) -> &'static str { "my_module" }
       fn version(&self) -> Version { Version::new(1, 0, 0) }
       
       fn init(&mut self) -> ModuleResult<()> {
           log::info!("MyModule initialized");
           Ok(())
       }
       
       fn cleanup(&mut self) -> ModuleResult<()> {
           Ok(())
       }
   }
   ```

3. **Register with the module system:**
   ```rust
   register_module!(MyModule);
   ```

### How do I add a new syscall?

1. **Define syscall number** in `core/src/syscall/mod.rs`:
   ```rust
   pub const SYS_MY_CALL: usize = 100;
   ```

2. **Implement handler** in `core/src/syscall/handlers/`:
   ```rust
   pub fn sys_my_call(arg1: usize, arg2: usize) -> SyscallResult {
       // Implementation
       Ok(0)
   }
   ```

3. **Register in dispatcher:**
   ```rust
   SYS_MY_CALL => sys_my_call(args[0], args[1]),
   ```

### How do I write kernel tests?

Helix uses Rust's test framework with custom test runner:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test_case]
    fn test_my_function() {
        let result = my_function(42);
        assert_eq!(result, 42);
    }
}
```

Run tests:
```bash
./scripts/test.sh
```

---

## Building & Running

### Build fails with "linker not found"

Install the cross-compilation linker:

```bash
# Ubuntu/Debian
sudo apt install lld

# Or use gcc's linker
sudo apt install gcc
```

Update `.cargo/config.toml`:
```toml
[target.x86_64-unknown-none]
linker = "rust-lld"
```

### QEMU won't start

**Possible causes:**

1. **QEMU not installed:**
   ```bash
   sudo apt install qemu-system-x86
   ```

2. **KVM permission denied:**
   ```bash
   sudo usermod -aG kvm $USER
   # Log out and back in
   ```

3. **No kernel binary:**
   ```bash
   ./scripts/build.sh
   ```

### How do I create a bootable ISO?

```bash
./scripts/build.sh --iso

# Output: build/output/helix.iso
```

The ISO can be burned to USB or booted in VirtualBox/VMware.

### How do I run on real hardware?

**Warning**: Running on real hardware can cause data loss!

1. Build bootable ISO:
   ```bash
   ./scripts/build.sh --iso
   ```

2. Write to USB:
   ```bash
   sudo dd if=build/output/helix.iso of=/dev/sdX bs=4M status=progress
   ```

3. Boot from USB and select Helix

---

## Debugging

### How do I use GDB with Helix?

1. **Start QEMU in debug mode:**
   ```bash
   ./scripts/run_qemu.sh --debug
   ```

2. **Connect GDB:**
   ```bash
   gdb build/output/helix-kernel
   (gdb) target remote localhost:1234
   (gdb) break kernel_main
   (gdb) continue
   ```

### I get a triple fault!

Triple faults usually indicate:

1. **Invalid page tables** - Check mapping code
2. **Stack overflow** - Increase stack size
3. **GDT/IDT issues** - Verify descriptor setup
4. **Interrupt handler bugs** - Check exception handlers

Debug approach:
```bash
# Enable more QEMU logging
qemu-system-x86_64 -d int,cpu_reset -no-reboot ...
```

### How do I read serial output?

QEMU shows serial output in the terminal by default:

```bash
./scripts/run_qemu.sh
# Output appears in terminal
```

Or capture to file:
```bash
./scripts/run_qemu.sh -serial file:serial.log
```

### My breakpoints don't work

1. **Ensure debug build:**
   ```bash
   ./scripts/build.sh --debug
   ```

2. **Wait for QEMU connection:**
   ```bash
   qemu-system-x86_64 -s -S ...  # -S pauses CPU at start
   ```

3. **Load symbols correctly:**
   ```bash
   (gdb) file build/output/helix-kernel
   (gdb) target remote :1234
   ```

---

## Performance

### How fast is Helix?

Context switch benchmark (on QEMU):
- **Context switch**: ~2-5 μs
- **Syscall overhead**: ~1-2 μs
- **IPC message**: ~3-5 μs

These are development measurements; real hardware will differ.

### How do I profile the kernel?

1. **Use the built-in profiler:**
   ```rust
   use helix_core::debug::profiler::*;
   
   measure!(profile, "operation_name", {
       // Code to measure
   });
   ```

2. **Use RDTSC timing:**
   ```rust
   let start = rdtsc();
   // ... code ...
   let elapsed = rdtsc() - start;
   ```

3. **QEMU performance counters:**
   ```bash
   qemu-system-x86_64 -d perf ...
   ```

### Why is QEMU slower than real hardware?

QEMU emulates hardware, adding overhead:

- **No KVM**: Pure emulation is 10-100x slower
- **With KVM**: Near-native speed for CPU, slower I/O

Enable KVM:
```bash
qemu-system-x86_64 -enable-kvm ...
```

---

## Contributing

### How do I contribute?

1. **Fork** the repository
2. **Create** a feature branch
3. **Make** your changes
4. **Test** thoroughly
5. **Submit** a pull request

See [CONTRIBUTING.md](../development/CONTRIBUTING.md) for details.

### What needs work?

Check the GitHub issues for:
- `good first issue` - Beginner-friendly tasks
- `help wanted` - Areas needing contributors
- `enhancement` - Feature requests

High-impact areas:
- Device drivers
- ARM64 port
- Userspace applications
- Documentation

### How do I report a bug?

1. **Check existing issues** for duplicates
2. **Gather information**:
   - Helix version/commit
   - Host OS and version
   - QEMU version
   - Steps to reproduce
   - Serial output / stack trace
3. **Open issue** with this information

---

## Comparisons

### Helix vs. Linux

| Aspect | Helix | Linux |
|--------|-------|-------|
| Code size | ~75K lines | ~30M lines |
| Language | Rust | C |
| Scheduler | Intent-based | CFS |
| Filesystem | HelixFS | ext4, btrfs, ... |
| Use case | Research/Learning | Production |
| Hardware | x86_64 (QEMU) | Everything |

### Helix vs. Redox OS

| Aspect | Helix | Redox |
|--------|-------|-------|
| Architecture | Modular monolithic | Microkernel |
| Language | Rust | Rust |
| User space | Minimal | Unix-like |
| Filesystem | HelixFS | RedoxFS |
| Maturity | Research | More mature |

### Helix vs. seL4

| Aspect | Helix | seL4 |
|--------|-------|------|
| Focus | Modularity | Formal verification |
| Size | Medium | Minimal |
| Security | Capability-based | Formally proven |
| Scheduler | DIS | Simple priority |

---

## Troubleshooting

### Build Errors

**Error: `can't find crate for 'core'`**
```bash
rustup target add x86_64-unknown-none
```

**Error: `linking with 'cc' failed`**
```bash
# Use rust-lld instead
export RUSTFLAGS="-C linker=rust-lld"
```

**Error: `error[E0463]: can't find crate for 'alloc'`**
```bash
# Ensure nightly toolchain
rustup override set nightly
```

### Runtime Errors

**Screen stays blank:**
- Check serial output for panic messages
- Verify GRUB configuration
- Check VGA initialization code

**Immediate reboot:**
- Triple fault - see debugging section
- Check CPU exception handlers

**Hangs at startup:**
- Add debug prints to narrow down location
- Check for infinite loops
- Verify interrupt handling

### Common Mistakes

1. **Forgetting `#![no_std]`**
   ```rust
   #![no_std]  // Required for kernel crates
   ```

2. **Missing `extern crate alloc`**
   ```rust
   extern crate alloc;  // For Vec, Box, etc.
   ```

3. **Incorrect memory addresses**
   - Remember physical vs virtual addresses
   - Apply higher-half offset where needed

4. **Interrupt handler not returning**
   - Must use `iretq` instruction
   - Ensure stack is properly aligned

---

## Still Have Questions?

- **GitHub Discussions**: Ask the community
- **Issue Tracker**: Report bugs and request features
- **Documentation**: Read the [full docs](../README.md)

---

<div align="center">

❓ *The only bad question is the one not asked* ❓

</div>
