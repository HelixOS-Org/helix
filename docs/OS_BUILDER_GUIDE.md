# Helix OS Builder Guide

This guide explains how to create custom operating systems using the Helix Framework.

## Overview

Helix provides a **framework** for building operating systems, not a single OS.
You create an OS by:

1. Choosing a profile (minimal, desktop, server, embedded, secure)
2. Selecting modules (scheduler, allocator, filesystems, drivers)
3. Configuring policies (security, resource limits)
4. Building your custom kernel

## Quick Start

### 1. Create a New Profile

```bash
mkdir profiles/myos
cd profiles/myos
```

### 2. Create Configuration (helix.toml)

```toml
[profile]
name = "myos"
version = "1.0.0"
description = "My custom operating system"
target = "desktop"

[profile.arch]
primary = "x86_64"
supported = ["x86_64"]

[profile.features]
multicore = true
hot_reload = true
userspace = true
networking = true
filesystem = true
graphics = false

[memory]
min_ram_mb = 64
max_ram_mb = 4096
heap_size_kb = 4096
virtual_memory = true

[scheduler]
module = "cfs"
time_slice_ms = 4
priority_levels = 140
load_balancing = true

[allocator]
physical = "buddy"
heap = "slab"

[modules.static]
# Modules linked into the kernel
modules = [
    "helix-scheduler-cfs",
    "helix-allocator-buddy",
    "helix-fs-ramfs",
]

[modules.dynamic]
# Modules loaded at runtime
modules = [
    "helix-driver-virtio",
]

[boot]
cmdline = "console=serial loglevel=4"

[security]
capabilities = true
mac = false
sandbox = true

[debug]
level = "normal"
symbols = true
stack_traces = true
```

### 3. Create Entry Point (src/main.rs)

```rust
#![no_std]
#![no_main]

use helix_core::Kernel;

#[no_mangle]
pub extern "C" fn kernel_main(boot_info: *const BootInfo) -> ! {
    // Initialize the kernel
    let mut kernel = Kernel::new();
    
    kernel.init_early(boot_info);
    kernel.init_memory();
    kernel.init_interrupts();
    kernel.init_scheduler();
    kernel.init_modules();
    
    // Start the system
    kernel.start();
}
```

### 4. Build

```bash
cargo build --release --target x86_64-unknown-none
```

## Profile Configuration

### Target Types

| Target | Description | Typical Use |
|--------|-------------|-------------|
| `minimal` | Bare minimum, no userspace | Testing, embedded |
| `embedded` | Small footprint, limited features | IoT, microcontrollers |
| `server` | Networking, no graphics | Cloud, servers |
| `desktop` | Full-featured with graphics | Workstations |
| `secure` | Hardened, minimal attack surface | Security-critical |

### Memory Configuration

```toml
[memory]
# Minimum RAM required
min_ram_mb = 64

# Maximum RAM to use (0 = all available)
max_ram_mb = 0

# Kernel heap size
heap_size_kb = 4096

# Stack size for kernel threads
stack_size_kb = 32

# Enable virtual memory
virtual_memory = true

# Enable huge pages
huge_pages = true
```

### Scheduler Selection

Available schedulers:

| Module | Description | Best For |
|--------|-------------|----------|
| `round_robin` | Simple time-sharing | Embedded, simple systems |
| `cfs` | Completely Fair Scheduler | General purpose |
| `realtime` | FIFO/RR real-time | Real-time systems |
| `cooperative` | Non-preemptive | Single-purpose, simple |
| `deadline` | EDF scheduling | Hard real-time |

```toml
[scheduler]
module = "cfs"
time_slice_ms = 4          # Base time slice
priority_levels = 140      # Number of priority levels
load_balancing = true      # SMP load balancing
interactive_boost = true   # Boost for interactive tasks
```

### Allocator Selection

| Module | Description | Best For |
|--------|-------------|----------|
| `bitmap` | Simple bitmap | Minimal systems |
| `buddy` | Buddy system | General purpose |
| `tlsf` | Two-Level Segregate Fit | Real-time |
| `slab` | Slab allocator | Kernel objects |

```toml
[allocator]
physical = "buddy"   # Physical page allocator
heap = "slab"        # Kernel heap allocator
```

## Module Selection

### Core Modules

```toml
[modules.static]
# Always needed
scheduler = "helix-scheduler-cfs"
allocator = "helix-allocator-buddy"

# Optional core
ipc = "helix-ipc-channels"
```

### Filesystem Modules

```toml
[modules.dynamic]
filesystems = [
    "helix-fs-ramfs",      # RAM-based FS (always useful)
    "helix-fs-devfs",      # Device filesystem
    "helix-fs-procfs",     # Process information
    "helix-fs-ext2",       # Ext2 support
    "helix-fs-fat32",      # FAT32 support
]
```

### Driver Modules

```toml
[modules.dynamic]
drivers = [
    "helix-driver-serial",     # Serial port
    "helix-driver-keyboard",   # PS/2 keyboard
    "helix-driver-virtio",     # VirtIO devices
    "helix-driver-ahci",       # SATA controller
    "helix-driver-nvme",       # NVMe controller
]
```

## Security Configuration

### Capability-Based Security

```toml
[security]
# Enable capability system
capabilities = true

# Default capabilities for new processes
default_caps = [
    "CAP_READ",
    "CAP_WRITE",
    "CAP_EXEC",
]

# Capability inheritance
inherit_caps = true
```

### Mandatory Access Control

```toml
[security]
# Enable MAC
mac = true

# MAC policy module
mac_module = "helix-mac-simple"

# Default labels
default_label = "user"
kernel_label = "system"
```

### Sandboxing

```toml
[security]
sandbox = true

# Sandbox restrictions
sandbox_config = {
    network = false,
    filesystem = "readonly",
    syscalls = "whitelist",
}
```

## Boot Configuration

### Command Line

```toml
[boot]
cmdline = "console=serial,115200 loglevel=4 init=/sbin/init"
```

### Boot Sequence

```toml
[boot]
# Early init (before scheduler)
early_init = [
    "console",
    "memory",
    "interrupts",
]

# Late init (after scheduler)
late_init = [
    "modules",
    "filesystems",
    "network",
]
```

## Building Different Variants

### Development Build

```bash
cargo build --target x86_64-unknown-none
```

### Release Build

```bash
cargo build --release --target x86_64-unknown-none
```

### With Debug Symbols

```bash
cargo build --release --target x86_64-unknown-none \
    --config 'profile.release.debug=true'
```

### For Different Architectures

```bash
# x86_64
cargo build --target x86_64-unknown-none

# ARM64
cargo build --target aarch64-unknown-none

# RISC-V 64
cargo build --target riscv64gc-unknown-none-elf
```

## Creating Bootable Images

### ISO (with GRUB/Limine)

```bash
helix-build iso --profile myos --output myos.iso
```

### UEFI

```bash
helix-build uefi --profile myos --output myos.efi
```

### Raw Disk Image

```bash
helix-build disk --profile myos --output myos.img --size 256M
```

## Testing

### QEMU

```bash
# x86_64
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/myos \
    -serial stdio -m 256M

# With debug
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/myos \
    -serial stdio -m 256M -s -S
```

### Integration Tests

```bash
helix-test run --profile myos
```

## Examples

### Minimal Embedded OS

```toml
[profile]
name = "embedded-minimal"
target = "embedded"

[profile.features]
multicore = false
hot_reload = false
userspace = false

[memory]
min_ram_mb = 4
heap_size_kb = 256
virtual_memory = false

[scheduler]
module = "cooperative"

[modules.static]
modules = ["helix-scheduler-cooperative"]
```

### Real-Time OS

```toml
[profile]
name = "realtime"
target = "embedded"

[profile.features]
multicore = true

[scheduler]
module = "realtime"

[allocator]
physical = "tlsf"  # Deterministic allocation

[security]
capabilities = true
```

### Secure OS

```toml
[profile]
name = "secure"
target = "secure"

[security]
capabilities = true
mac = true
sandbox = true
audit = true

[boot]
# Minimal cmdline
cmdline = "quiet"

[debug]
level = "minimal"
symbols = false
```

## Next Steps

1. Read the [Module Guide](MODULE_GUIDE.md) to create custom modules
2. See the [Architecture](ARCHITECTURE.md) for internals
3. Check [examples/](../profiles/) for complete profile examples
