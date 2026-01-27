# Helix OS Changelog

<div align="center">

📜 **Version History and Release Notes**

*All notable changes to the Helix OS project*

</div>

---

## Versioning

Helix OS follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality (backwards-compatible)
- **PATCH**: Bug fixes (backwards-compatible)

---

## [Unreleased]

### Added
- HelixFS filesystem integration with B-tree indexing
- Copy-on-Write (CoW) snapshot support
- Journaling for crash recovery
- LZ4 and Zstd compression support
- Comprehensive documentation in `docs/` directory

### Changed
- Improved boot sequence reliability
- Enhanced serial console output formatting
- Updated memory allocator with better fragmentation handling

### Fixed
- Triple fault during early boot initialization
- Page table setup for identity mapping
- GDT configuration for long mode

### Security
- Added SMEP/SMAP support detection

---

## [0.3.0] - 2024-XX-XX

### Added

#### Core System
- **Differentiated Intent Scheduler (DIS)**
  - Intent-based task scheduling
  - Multiple queue types: Realtime, Interactive, Batch, Background
  - EDF (Earliest Deadline First) for realtime tasks
  - Dynamic priority adjustment
  
- **Hot-Reload System**
  - Module hot-reloading without restart
  - State preservation during reload
  - Graceful degradation on failure
  
- **IPC System**
  - Typed message channels
  - Event bus for pub/sub
  - Message routing infrastructure

#### Memory Management
- **Buddy Allocator**
  - O(log n) allocation/deallocation
  - Efficient memory coalescing
  - Support for various block sizes

- **Slab Allocator**
  - Per-type object caching
  - Reduced fragmentation for small objects
  - Magazine-based per-CPU caching

- **Virtual Memory**
  - 4-level page table management
  - Demand paging support
  - Copy-on-Write for fork optimization

#### Hardware Abstraction
- **x86_64 Platform Support**
  - APIC/IOAPIC interrupt handling
  - MSI/MSI-X support
  - ACPI table parsing
  
- **CPU Features**
  - SSE/AVX detection and enabling
  - TSC calibration
  - CPUID feature detection

### Changed
- Refactored module system for better ABI compatibility
- Improved kernel logging with log levels
- Enhanced panic handler with stack traces

### Fixed
- Race condition in scheduler queue management
- Memory leak in IPC channel cleanup
- Interrupt routing for multiple IOAPICs

### Deprecated
- Legacy PIC support (use APIC instead)
- Simple round-robin scheduler (use DIS)

---

## [0.2.0] - 2024-XX-XX

### Added

#### Module System
- **Dynamic Module Loading**
  - ELF binary loading
  - Symbol resolution
  - Dependency management
  
- **Module Registry**
  - Version tracking
  - Interface discovery
  - Hot-swap support

#### Interrupt Handling
- **IDT Setup**
  - Exception handlers for all CPU exceptions
  - Interrupt routing infrastructure
  - IST (Interrupt Stack Table) support
  
- **Exception Handling**
  - Page fault handler with debug output
  - Double fault handler with separate stack
  - General protection fault handling

#### Debug Infrastructure
- **Serial Console**
  - COM1 output support
  - Formatted printing macros
  - Debug log levels

- **Panic Handler**
  - Stack trace printing
  - Register dump
  - Halt behavior

### Changed
- Moved from assembly GDT to Rust implementation
- Improved build system with better dependency tracking
- Updated toolchain to latest nightly

### Fixed
- Stack alignment issues in interrupt handlers
- Incorrect segment selector in TSS
- Boot hang on some UEFI systems

---

## [0.1.0] - 2024-XX-XX

### Added

#### Boot System
- **Multiboot2 Compliance**
  - GRUB bootloader support
  - Multiboot2 header parsing
  - Memory map handling
  
- **Boot Assembly**
  - 32-bit to 64-bit transition
  - Initial page tables
  - GDT setup for long mode

#### Core Infrastructure
- **Kernel Entry**
  - `kernel_main` entry point
  - Basic initialization sequence
  - Kernel heap setup

- **HAL Foundation**
  - CPU abstraction trait
  - MMU abstraction trait
  - Port I/O functions

#### Memory
- **Physical Memory**
  - Bitmap allocator
  - Frame allocation
  - Memory map parsing

- **Virtual Memory**
  - Identity mapping for kernel
  - Higher-half kernel mapping
  - Basic page table operations

### Notes
- Initial release
- x86_64 only
- Requires GRUB2 bootloader

---

## Version Comparison

| Version | Tasks | Memory | FS | Modules | Hot-Reload |
|---------|-------|--------|-------|---------|------------|
| 0.1.0   | ❌    | Basic  | ❌    | ❌      | ❌         |
| 0.2.0   | ❌    | Buddy  | ❌    | ✅      | ❌         |
| 0.3.0   | DIS   | Full   | ❌    | ✅      | ✅         |
| 0.4.0   | DIS   | Full   | HelixFS | ✅   | ✅         |

---

## Migration Guides

### 0.2.x to 0.3.x

#### Module API Changes

```rust
// Old (0.2.x)
impl Module for MyModule {
    fn init(&self) -> Result<()> { ... }
    fn cleanup(&self) -> Result<()> { ... }
}

// New (0.3.x)
impl Module for MyModule {
    fn name(&self) -> &'static str { "my_module" }
    fn version(&self) -> Version { Version::new(1, 0, 0) }
    fn init(&mut self) -> ModuleResult<()> { ... }
    fn cleanup(&mut self) -> ModuleResult<()> { ... }
}
```

#### Scheduler Changes

```rust
// Old (0.2.x) - Manual priority
scheduler.add_task(task, priority: 10);

// New (0.3.x) - Intent-based
scheduler.spawn(task, Intent::interactive(1000));
```

### 0.1.x to 0.2.x

#### Memory API Changes

```rust
// Old (0.1.x)
let frame = allocate_frame();

// New (0.2.x)
let frame = FRAME_ALLOCATOR.lock().allocate()?;
```

#### Interrupt Registration

```rust
// Old (0.1.x)
register_handler(14, page_fault);

// New (0.2.x)
idt[14].set_handler_fn(page_fault_handler);
```

---

## Contributors

Thanks to all contributors who made these releases possible!

See [CONTRIBUTORS.md](../CONTRIBUTORS.md) for the full list.

---

## Links

- [GitHub Releases](https://github.com/helix-os/helix/releases)
- [Issue Tracker](https://github.com/helix-os/helix/issues)
- [Roadmap](ROADMAP.md)

---

<div align="center">

📜 *Every line of code tells a story* 📜

</div>
