# 🌋 MAGMA - Revolutionary NVIDIA GPU Driver for Helix OS

```
  __  __    _    ____ __  __    _
 |  \/  |  / \  / ___|  \/  |  / \
 | |\/| | / _ \| |  _| |\/| | / _ \
 | |  | |/ ___ \ |_| | |  | |/ ___ \
 |_|  |_/_/   \_\____|_|  |_/_/   \_\
```

> **The GPU driver that makes legacy monolithic drivers obsolete.**

---

## 🎯 Vision

MAGMA is not a hobbyist experiment. It is an **industrial-grade infrastructure** designed to:

- Scale to **millions of lines of code**
- Support the **next decade of GPU generations**
- Provide **zero-overhead Vulkan 1.3** performance
- Guarantee **memory safety** through Rust's type system

---

## 🏗️ Architecture: The Four Pillars

### 1. 🔧 GSP-First & Firmware-Driven

MAGMA is a **"thin client"** driver. We offload 90% of hardware initialization and scheduling logic to the **NVIDIA GSP** (GPU System Processor) firmware.

```
┌─────────────────────────────────────────────────────────────────┐
│                        APPLICATION                              │
│                     (Vulkan 1.3 API)                            │
└──────────────────────────┬──────────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────┐
│                      u-magma (Userspace)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ magma-vulkan │  │  magma-cmd   │  │    magma-mem         │   │
│  │   (API)      │  │ (Commands)   │  │   (Allocator)        │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
│                           │                                      │
│  ┌────────────────────────▼─────────────────────────────────┐   │
│  │                    magma-rpc                              │   │
│  │          (GSP Firmware Communication)                     │   │
│  └────────────────────────┬─────────────────────────────────┘   │
└───────────────────────────│─────────────────────────────────────┘
                            │ IPC (minimal)
┌───────────────────────────▼─────────────────────────────────────┐
│                      k-magma (Kernel)                           │
│                   < 2,000 lines of code                         │
│         [ IRQ Handler ]  [ IOMMU Setup ]  [ BAR Map ]           │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                      GPU HARDWARE                               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   GSP Firmware                          │    │
│  │    (Handles 90% of initialization & scheduling)        │    │
│  └─────────────────────────────────────────────────────────┘    │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐     │
│  │ GR Eng  │  │ CE Eng  │  │ NVDEC   │  │ Display Engine  │     │
│  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

### 2. 🔒 Pure Userspace & Microkernel

| Component | Location | Lines of Code | Responsibility |
|-----------|----------|---------------|----------------|
| `k-magma` | Kernel   | < 2,000       | IRQ, IOMMU, BAR mapping |
| `u-magma` | Userspace | Unlimited    | Everything else |

**Why?** Crashes in userspace don't bring down the system. Hot-reload is possible.

### 3. 🎮 Vulkan-Native

- **No OpenGL** - No legacy baggage
- **No compatibility layers** - Direct hardware access
- **Zero-overhead** - Rust abstractions compile to optimal code

### 4. 🛡️ Safety via Type System

```rust
// Ownership guarantees no double-submit
pub struct CommandBuffer<'a, S: CommandBufferState> {
    ring: &'a CommandRing,
    handle: BufferHandle,
    _state: PhantomData<S>,
}

// State machine prevents invalid transitions
impl<'a> CommandBuffer<'a, Recording> {
    pub fn end(self) -> CommandBuffer<'a, Executable> { ... }
}

impl<'a> CommandBuffer<'a, Executable> {
    pub fn submit(self, queue: &Queue) -> Fence { ... }
}
```

---

## 📦 Crate Structure

```
magma/
├── crates/
│   ├── magma-core/           # Shared traits & types
│   ├── magma-core-types/     # Fundamental types
│   ├── magma-core-sync/      # Synchronization primitives
│   ├── magma-core-alloc/     # Allocation abstractions
│   │
│   ├── magma-hal/            # Hardware Abstraction Layer
│   ├── magma-hal-pci/        # PCI enumeration
│   ├── magma-hal-bar/        # BAR mapping
│   ├── magma-hal-mmio/       # MMIO access
│   ├── magma-hal-irq/        # Interrupt handling
│   │
│   ├── magma-rpc/            # GSP Communication
│   ├── magma-rpc-transport/  # Low-level transport
│   ├── magma-rpc-protocol/   # Message format (RM API)
│   ├── magma-rpc-handshake/  # Firmware loading
│   │
│   ├── magma-mem/            # Memory Management
│   ├── magma-mem-vram/       # VRAM buddy allocator
│   ├── magma-mem-mmu/        # GPU page tables
│   │
│   ├── magma-cmd/            # Command Submission
│   ├── magma-cmd-ring/       # Ring buffers
│   ├── magma-cmd-fence/      # Synchronization
│   │
│   ├── magma-vulkan/         # Vulkan 1.3 Implementation
│   ├── magma-vulkan-*/       # Per-object implementations
│   │
│   ├── k-magma/              # Kernel module (minimal)
│   ├── u-magma/              # Userspace daemon
│   │
│   └── magma-gen-*/          # Generation-specific code
```

---

## 🚀 Quick Start

```bash
# Build the driver
cargo build --release -p u-magma

# Load kernel module (requires root)
sudo insmod target/release/k_magma.ko

# Start userspace daemon
./target/release/u-magma --gpu=0

# Run Vulkan application
VK_ICD_FILENAMES=/etc/vulkan/icd.d/magma.json ./my_vulkan_app
```

---

## 📊 Performance Targets

| Metric | Target | Linux nouveau | MAGMA Goal |
|--------|--------|---------------|------------|
| Context switch | < 1μs | ~5μs | 200ns |
| Command submit | < 500ns | ~2μs | 100ns |
| Memory alloc | O(log n) | O(n) | O(log n) |
| Vulkan overhead | < 1% | N/A | < 0.5% |

---

## 🗺️ Roadmap

See [ROADMAP.md](./docs/ROADMAP.md) for the complete INFINITY roadmap.

| Milestone | Description | Target |
|-----------|-------------|--------|
| M1 | Boot & GSP Handshake | Q1 2026 |
| M2 | MMU & BAR Mapping | Q2 2026 |
| M3 | Command Submission | Q3 2026 |
| M4 | Display Engine | Q4 2026 |
| M5 | Vulkan 1.3 Compliance | Q2 2027 |

---

## 📜 License

MAGMA is dual-licensed under MIT and Apache 2.0.

---

## 🤝 Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

*Built with 🦀 Rust for Helix OS*
