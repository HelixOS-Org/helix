# Getting Started with Helix

<div align="center">

🚀 **From Zero to Running Kernel in 10 Minutes**

*Complete setup guide for Helix OS Framework*

</div>

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Installation](#2-installation)
3. [Building Helix](#3-building-helix)
4. [Running in QEMU](#4-running-in-qemu)
5. [Understanding the Output](#5-understanding-the-output)
6. [Your First Modification](#6-your-first-modification)
7. [Project Structure](#7-project-structure)
8. [Development Workflow](#8-development-workflow)
9. [Common Issues](#9-common-issues)
10. [Next Steps](#10-next-steps)

---

## 1. Prerequisites

### 1.1 Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | x86_64 compatible | Modern Intel/AMD |
| **RAM** | 4 GB | 8+ GB |
| **Disk** | 2 GB free | 10+ GB free |
| **OS** | Linux (any modern distro) | Ubuntu 22.04+ / Fedora 38+ |

> **Note**: Helix can also be developed on macOS and Windows (via WSL2), but Linux provides the best experience.

### 1.2 Software Requirements

| Software | Version | Purpose |
|----------|---------|---------|
| **Rust** | Nightly | Primary development language |
| **QEMU** | 5.0+ | Hardware emulation |
| **GNU Make** | 4.0+ | Build automation |
| **Git** | 2.0+ | Version control |
| **xorriso** | Any | ISO creation |
| **GRUB** | 2.0+ | Bootloader |

### 1.3 Check Your System

Run these commands to verify your system is ready:

```bash
# Check if Rust is installed
rustc --version
# Expected: rustc 1.XX.X-nightly (...)

# Check QEMU
qemu-system-x86_64 --version
# Expected: QEMU emulator version X.X.X

# Check Make
make --version
# Expected: GNU Make X.X

# Check Git
git --version
# Expected: git version X.X.X

# Check xorriso
xorriso --version 2>&1 | head -1
# Expected: xorriso X.X.X

# Check GRUB
grub-mkrescue --version
# Expected: grub-mkrescue (GRUB) X.XX
```

---

## 2. Installation

### 2.1 Install Rust

If you don't have Rust installed:

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then reload your shell
source ~/.bashrc  # or ~/.zshrc

# Verify installation
rustc --version
cargo --version
```

### 2.2 Configure Rust for Helix

Helix requires Rust nightly with specific components:

```bash
# Switch to nightly Rust
rustup default nightly

# Add the bare-metal target
rustup target add x86_64-unknown-none

# Add required components
rustup component add rust-src
rustup component add llvm-tools-preview

# Verify target is installed
rustup target list | grep x86_64-unknown-none
# Should show: x86_64-unknown-none (installed)
```

### 2.3 Install System Dependencies

#### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
    qemu-system-x86 \
    xorriso \
    grub-pc-bin \
    grub-common \
    make \
    git \
    build-essential
```

#### Fedora

```bash
sudo dnf install -y \
    qemu-system-x86 \
    xorriso \
    grub2-tools \
    grub2-pc-modules \
    make \
    git \
    gcc
```

#### Arch Linux

```bash
sudo pacman -S \
    qemu-system-x86 \
    xorriso \
    grub \
    make \
    git \
    base-devel
```

#### macOS (via Homebrew)

```bash
brew install \
    qemu \
    xorriso \
    x86_64-elf-grub \
    make \
    git

# Note: macOS requires additional setup for cross-compilation
# Consider using Docker for full compatibility
```

### 2.4 Clone the Repository

```bash
# Clone Helix
git clone https://github.com/helix-os/helix.git
cd helix

# Verify the structure
ls -la
# Should show: Cargo.toml, scripts/, docs/, core/, hal/, fs/, etc.
```

---

## 3. Building Helix

### 3.1 Quick Build (Recommended)

The simplest way to build Helix:

```bash
# Build everything
bash scripts/build.sh all
```

This runs through 12 build steps:

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                           HELIX BUILD SYSTEM                                 ║
╠══════════════════════════════════════════════════════════════════════════════╣
Step 1/12: prepare_env ............................ [OK] 0.1s
Step 2/12: build_bootloader ....................... [OK] 0.3s
Step 3/12: build_core_kernel ...................... [OK] 2.1s
Step 4/12: build_hal .............................. [OK] 0.8s
Step 5/12: build_modules .......................... [OK] 1.2s
Step 6/12: build_fs ............................... [OK] 3.4s
Step 7/12: build_subsystems ....................... [OK] 1.8s
Step 8/12: build_profile .......................... [OK] 2.6s
Step 9/12: link_kernel ............................ [OK] 0.5s
Step 10/12: create_iso ............................ [OK] 0.8s
Step 11/12: verify_build .......................... [OK] 0.2s
Step 12/12: generate_symbols ...................... [OK] 0.3s
═══════════════════════════════════════════════════════════════════════════════
                              BUILD COMPLETE!
                          Time: 14.1s | Artifacts: 3
═══════════════════════════════════════════════════════════════════════════════
```

### 3.2 Individual Build Steps

You can also run specific build steps:

```bash
# Just prepare the environment
bash scripts/build.sh prepare_env

# Build only the core kernel
bash scripts/build.sh build_core_kernel

# Build the filesystem
bash scripts/build.sh build_fs

# Create the ISO
bash scripts/build.sh create_iso
```

### 3.3 Manual Build (Advanced)

For more control, use Cargo directly:

```bash
# Build the minimal OS profile
cargo build \
    --package helix-minimal-os \
    --target x86_64-unknown-none \
    --release

# The binary will be at:
# target/x86_64-unknown-none/release/helix-minimal-os
```

### 3.4 Build Configuration

The build system uses `profiles/minimal/helix.toml`:

```toml
[profile]
name = "minimal"
description = "Minimal Helix OS for testing and development"
target = "x86_64-unknown-none"

[features]
scheduler = "round-robin"
allocator = "bump"
filesystem = "helixfs"
ipc = "channels"
shell = true
benchmarks = true
demos = true

[memory]
heap_size = "1MB"
stack_size = "16KB"

[boot]
protocol = "multiboot2"
serial = true
vga = true

[debug]
symbols = true
serial_log = true
log_level = "info"
```

### 3.5 Build Outputs

After a successful build:

```
build/
├── output/
│   ├── helix-kernel        # ELF kernel binary (~2 MB)
│   ├── helix.iso           # Bootable ISO (~32 MB)
│   └── boot/
│       └── grub/
│           └── grub.cfg    # GRUB configuration
├── iso/
│   └── boot/
│       ├── helix-kernel    # Kernel copy for ISO
│       └── grub/
│           └── grub.cfg
└── logs/
    └── build_YYYYMMDD.log  # Build log
```

### 3.6 Clean Build

To start fresh:

```bash
# Clean all build artifacts
bash scripts/build.sh clean

# Or use Cargo
cargo clean

# Then rebuild
bash scripts/build.sh all
```

---

## 4. Running in QEMU

### 4.1 Quick Run

The easiest way to run Helix:

```bash
bash scripts/run_qemu.sh
```

### 4.2 Manual QEMU Command

For more control:

```bash
qemu-system-x86_64 \
    -cdrom build/output/helix.iso \
    -m 256M \
    -serial stdio \
    -display none
```

### 4.3 QEMU Options Explained

| Option | Description |
|--------|-------------|
| `-cdrom` | Boot from ISO as CD-ROM |
| `-m 256M` | Allocate 256 MB RAM |
| `-serial stdio` | Redirect serial output to terminal |
| `-display none` | No graphical display |

### 4.4 Running with Display

To see VGA output:

```bash
qemu-system-x86_64 \
    -cdrom build/output/helix.iso \
    -m 256M \
    -serial stdio \
    -display gtk
```

### 4.5 Running with Debugging

For GDB debugging:

```bash
# Terminal 1: Start QEMU with debug
qemu-system-x86_64 \
    -cdrom build/output/helix.iso \
    -m 256M \
    -serial stdio \
    -s -S

# Terminal 2: Connect GDB
gdb build/output/helix-kernel
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```

### 4.6 QEMU with Logging

For detailed CPU/interrupt logging:

```bash
qemu-system-x86_64 \
    -cdrom build/output/helix.iso \
    -m 256M \
    -serial stdio \
    -display none \
    -d int,cpu_reset \
    -D /tmp/qemu.log
```

---

## 5. Understanding the Output

### 5.1 Boot Sequence

When Helix boots, you'll see:

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                          HELIX OS FRAMEWORK v0.1.0                           ║
║                     A Framework for Building Operating Systems               ║
╚══════════════════════════════════════════════════════════════════════════════╝

[BOOT] Multiboot2 boot successful
[BOOT] Command line: (none)
[BOOT] Boot loader: GRUB 2.06

[INIT] ================================================================
[INIT]   PHASE 1: EARLY INITIALIZATION
[INIT] ================================================================

[SERIAL] Serial port initialized at 0x3F8 (115200 baud)
[VGA] VGA text mode console initialized (80x25)

[INIT] ================================================================
[INIT]   PHASE 2: MEMORY SETUP
[INIT] ================================================================

[HEAP] Initializing kernel heap allocator...
[HEAP] Heap start: 0x200000
[HEAP] Heap size: 1048576 bytes (1 MB)
[HEAP] Allocator: BumpAllocator
[HEAP] Kernel heap ready!

[MEM] Memory subsystem initializing...
[MEM] Physical memory: 256 MB
[MEM] Kernel size: 2.1 MB
[MEM] Available: 253.9 MB
[MEM] Memory subsystem ready!
```

### 5.2 Subsystem Initialization

```
[INIT] ================================================================
[INIT]   PHASE 3: SUBSYSTEM INITIALIZATION
[INIT] ================================================================

[INT] Installing interrupt handlers...
[INT] IDT loaded at 0x100000
[INT] Exception handlers: 32
[INT] IRQ handlers: 16
[INT] Interrupts ready!

[SCHED] Scheduler framework initializing...
[SCHED] Default scheduler: RoundRobin v0.1.0
[SCHED] CPU count: 1
[SCHED] Scheduler ready!

[MOD] Module registry initializing...
[MOD] Hot-reload engine: enabled
[MOD] Self-healing: enabled
[MOD] Module system ready!
```

### 5.3 Filesystem Initialization

```
[INIT] ================================================================
[INIT]   PHASE 4: FILESYSTEM
[INIT] ================================================================

[FS] HelixFS initializing...
[FS] Backend: RamDisk (4096 KB)
[FS] Block size: 4096 bytes
[FS] Features: CoW, Journal, Snapshots
[FS] Formatting filesystem...
[FS] Creating root directory...
[FS] HelixFS ready!
[FS]   Total: 4096 KB
[FS]   Free: 4032 KB
[FS]   Used: 64 KB
```

### 5.4 Demos and Shell

```
[INIT] ================================================================
[INIT]   PHASE 5: DEMOS AND SHELL
[INIT] ================================================================

[DEMO] Running filesystem demo...
  Creating /hello.txt ... OK
  Creating /kernel.rs ... OK
  Creating /src/ ... OK
  Creating /docs/ ... OK
  Listing root directory:
    📁 .
    📁 ..
    📄 hello.txt
    📄 kernel.rs
    📁 src
    📁 docs

[DEMO] Running hot-reload demo...
  Current scheduler: RoundRobin v0.1.0
  Loading new scheduler: Priority v0.2.0
  Hot-swap in progress...
  State migration: 3 threads
  Hot-swap complete! Time: 1.2ms
  New scheduler: Priority v0.2.0

[DEMO] Running benchmarks...
  Context switch (minimal): 21 cycles (8.4 ns)
  Context switch (full): 180 cycles (72 ns)
  Syscall latency: 95 cycles (38 ns)
  DIS intent eval: 18 cycles (7.2 ns)

[SHELL] Helix Shell v0.1.0
[SHELL] Type 'help' for commands

helix> _
```

### 5.5 Shell Commands

The shell supports these commands:

```
helix> help

Available commands:
  help      - Show this help message
  version   - Show kernel version
  uptime    - Show system uptime
  ps        - List processes/threads
  mem       - Show memory statistics
  uname     - System information
  clear     - Clear screen
  history   - Command history
  echo      - Echo text
  env       - Show environment
  export    - Set environment variable
  pwd       - Print working directory
  date      - Show current date/time
  demo      - Run demos (dis, hotreload, selfheal)
  bench     - Run benchmarks (quick, full)
  exit      - Exit shell (halt system)

helix> mem

Memory Statistics:
  Physical Memory:
    Total:     256 MB
    Used:      4.2 MB
    Free:      251.8 MB
    Usage:     1.6%

  Kernel Heap:
    Total:     1024 KB
    Used:      128 KB
    Free:      896 KB
    Allocs:    342
    Deallocs:  156

helix> uname -a

Helix OS Framework 0.1.0-alpha
Architecture: x86_64
Compiler: rustc 1.76.0-nightly
Build date: 2026-01-27
```

---

## 6. Your First Modification

Let's make a simple change to verify your development setup works.

### 6.1 Modify the Boot Message

Open `profiles/minimal/src/main.rs`:

```rust
// Find this function:
fn print_banner() {
    serial_println!("");
    serial_println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    serial_println!("║                          HELIX OS FRAMEWORK v0.1.0                           ║");
    serial_println!("║                     A Framework for Building Operating Systems               ║");
    serial_println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    serial_println!("");
}

// Change it to:
fn print_banner() {
    serial_println!("");
    serial_println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    serial_println!("║                          MY CUSTOM HELIX BUILD                               ║");
    serial_println!("║                              Hello, World!                                   ║");
    serial_println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    serial_println!("");
}
```

### 6.2 Rebuild and Run

```bash
# Rebuild
bash scripts/build.sh all

# Run
bash scripts/run_qemu.sh
```

You should see your custom banner!

### 6.3 Add a Shell Command

Let's add a new shell command. Open `subsystems/userspace/src/shell.rs`:

```rust
// Find the command match statement and add:
"hello" => {
    serial_println!("Hello from my custom command!");
    serial_println!("This is my first Helix modification.");
}
```

Rebuild and run, then try:

```
helix> hello
Hello from my custom command!
This is my first Helix modification.
```

---

## 7. Project Structure

### 7.1 Top-Level Overview

```
helix/
├── Cargo.toml              # Workspace root, defines all crates
├── rust-toolchain.toml     # Rust version configuration
├── Makefile                # Alternative build system
│
├── boot/                   # Boot protocols and assembly
├── hal/                    # Hardware Abstraction Layer
├── core/                   # Kernel core (orchestrator, IPC)
├── subsystems/             # Execution, memory, DIS, userspace
├── modules/                # Module system
├── fs/                     # HelixFS filesystem
├── benchmarks/             # Performance benchmarks
├── profiles/               # OS profiles (minimal, desktop, etc.)
├── scripts/                # Build and run scripts
└── docs/                   # Documentation (you are here)
```

### 7.2 Key Crates

| Crate | Path | Purpose | Lines |
|-------|------|---------|-------|
| `helix-core` | `core/` | Kernel orchestrator, IPC, syscalls | ~3,000 |
| `helix-hal` | `hal/` | Hardware abstraction | ~2,000 |
| `helix-modules` | `modules/` | Module system | ~2,500 |
| `helix-fs` | `fs/` | HelixFS filesystem | ~42,500 |
| `helix-execution` | `subsystems/execution/` | Thread/process | ~1,500 |
| `helix-memory` | `subsystems/memory/` | Memory management | ~2,000 |
| `helix-dis` | `subsystems/dis/` | Intent scheduling | ~11,000 |
| `helix-userspace` | `subsystems/userspace/` | Shell, ELF loader | ~1,800 |
| `helix-benchmarks` | `benchmarks/` | Benchmarking | ~1,200 |
| `helix-minimal-os` | `profiles/minimal/` | Minimal OS profile | ~800 |

### 7.3 Dependency Graph

```
                        ┌─────────────────────┐
                        │  helix-minimal-os   │
                        │  (Profile/Binary)   │
                        └──────────┬──────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
    ┌─────────────┐         ┌─────────────┐         ┌─────────────┐
    │  helix-fs   │         │ helix-core  │         │ helix-dis   │
    └──────┬──────┘         └──────┬──────┘         └──────┬──────┘
           │                       │                       │
           │               ┌───────┼───────┐               │
           │               │       │       │               │
           ▼               ▼       ▼       ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │helix-modules│ │helix-memory │ │helix-execut.│ │helix-usersp.│
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └─────────────┘
           │               │               │
           └───────────────┼───────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  helix-hal  │
                    └─────────────┘
```

---

## 8. Development Workflow

### 8.1 Typical Development Cycle

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       HELIX DEVELOPMENT WORKFLOW                           │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐ │
│   │  Edit   │───▶│  Build  │───▶│  Test   │───▶│   Run   │───▶│ Commit  │ │
│   │  Code   │    │         │    │         │    │  QEMU   │    │         │ │
│   └─────────┘    └─────────┘    └─────────┘    └─────────┘    └─────────┘ │
│       │              │              │              │              │       │
│       │              │              │              │              │       │
│       ▼              ▼              ▼              ▼              ▼       │
│   Your IDE      build.sh        cargo test    run_qemu.sh    git commit  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Quick Iteration Script

Create `dev.sh` for rapid development:

```bash
#!/bin/bash
# dev.sh - Quick build and run

set -e

echo "Building..."
bash scripts/build.sh all

echo "Running..."
timeout 30 bash scripts/run_qemu.sh || true

echo "Done!"
```

Run with:

```bash
chmod +x dev.sh
./dev.sh
```

### 8.3 Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package helix-core
cargo test --package helix-fs
cargo test --package helix-modules

# Run tests with output
cargo test --workspace -- --nocapture

# Run a specific test
cargo test --package helix-fs test_create_file
```

### 8.4 Code Formatting and Linting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Lint with Clippy
cargo clippy --workspace --all-targets

# Lint with strict settings
cargo clippy --workspace --all-targets -- -D warnings
```

### 8.5 Documentation Generation

```bash
# Generate API documentation
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

---

## 9. Common Issues

### 9.1 Build Errors

#### Error: "can't find crate for `core`"

**Solution**: You need the `rust-src` component:

```bash
rustup component add rust-src
```

#### Error: "linker `rust-lld` not found"

**Solution**: Install LLVM tools:

```bash
rustup component add llvm-tools-preview
```

#### Error: "target may not be installed"

**Solution**: Add the bare-metal target:

```bash
rustup target add x86_64-unknown-none
```

### 9.2 QEMU Errors

#### Error: "qemu-system-x86_64: command not found"

**Solution**: Install QEMU:

```bash
# Ubuntu/Debian
sudo apt install qemu-system-x86

# Fedora
sudo dnf install qemu-system-x86
```

#### Error: "Could not read from CDROM"

**Solution**: Build the ISO first:

```bash
bash scripts/build.sh create_iso
```

#### System hangs or triple faults

**Solution**: Enable debug logging:

```bash
qemu-system-x86_64 \
    -cdrom build/output/helix.iso \
    -m 256M \
    -d int,cpu_reset \
    -D /tmp/qemu.log \
    -serial stdio

# Check the log
tail -50 /tmp/qemu.log
```

### 9.3 ISO Creation Errors

#### Error: "grub-mkrescue: command not found"

**Solution**: Install GRUB tools:

```bash
# Ubuntu/Debian
sudo apt install grub-pc-bin grub-common

# Fedora
sudo dnf install grub2-tools grub2-pc-modules
```

#### Error: "xorriso: command not found"

**Solution**: Install xorriso:

```bash
sudo apt install xorriso  # or dnf/pacman equivalent
```

### 9.4 Debugging Tips

#### Enable Serial Logging

All output goes to serial by default. Make sure you use `-serial stdio` with QEMU.

#### Add Debug Prints

```rust
// In your code
serial_println!("[DEBUG] Variable x = {}", x);
serial_println!("[DEBUG] Reached checkpoint 1");
```

#### Use GDB

```bash
# Terminal 1
qemu-system-x86_64 -s -S -cdrom build/output/helix.iso -serial stdio

# Terminal 2
gdb build/output/helix-kernel
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```

---

## 10. Next Steps

Congratulations! You now have Helix running. Here's where to go next:

### 10.1 Learn the Architecture

📖 Read [Architecture Overview](../architecture/OVERVIEW.md) to understand how Helix is designed.

### 10.2 Explore the APIs

📖 Dive into the API documentation:
- [Core API](../api/CORE.md) - Kernel core functionality
- [HAL API](../api/HAL.md) - Hardware abstraction
- [Module System](../api/MODULES.md) - Module development
- [Filesystem API](../api/FILESYSTEM.md) - HelixFS
- [Scheduler API](../api/SCHEDULER.md) - DIS and scheduling

### 10.3 Make Changes

🛠️ Try these exercises:
1. Add a new shell command
2. Modify the boot banner
3. Create a simple kernel module
4. Add a new syscall
5. Implement a simple driver

### 10.4 Contribute

🤝 Ready to contribute? See [Contributing Guide](../development/CONTRIBUTING.md).

### 10.5 Get Help

💬 If you're stuck:
- Check the [FAQ](../reference/FAQ.md)
- Search [GitHub Issues](https://github.com/helix-os/helix/issues)
- Ask on [Discord](https://discord.gg/helix-os)
- Read existing code for examples

---

<div align="center">

**You're ready to build with Helix!**

🧬 *Happy hacking!* 🧬

</div>
