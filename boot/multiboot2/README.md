# Helix Multiboot2

**Revolutionary type-safe Multiboot2 implementation for Helix OS**

## 🎯 Design Goals

1. **Zero-Copy Parsing**: Tags are parsed in-place with borrowed references
2. **Compile-Time Safety**: Header generation with static checksum validation
3. **Type-Safe Abstractions**: Strongly typed tags with exhaustive enums
4. **Lifetime-Bound**: All parsed data is lifetime-bound to boot information
5. **Future-Proof**: Designed for easy extension to UEFI/Limine

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      helix-multiboot2                        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  header/           - Compile-time header generation          │
│    ├── builder.rs  - Type-safe header builder                │
│    └── tags.rs     - Header tag types                        │
│                                                              │
│  info/             - Boot information parsing                │
│    ├── mod.rs      - Main Multiboot2Info struct              │
│    ├── tags.rs     - Tag enum and parsing                    │
│    ├── memory.rs   - Memory map abstraction                  │
│    ├── framebuffer.rs - Framebuffer info                     │
│    └── iterator.rs - Zero-copy tag iterator                  │
│                                                              │
│  boot_info.rs      - Unified BootInfo abstraction            │
│  validate.rs       - Runtime validation                      │
│  lib.rs            - Public API                              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 🚀 Usage

### Header Generation (compile-time)

```rust
use helix_multiboot2::header;

// Generates a valid Multiboot2 header at compile time
header::define_multiboot2_header! {
    // Optional: request specific features
    framebuffer: (1024, 768, 32),
    // Memory alignment requirements
    alignment: 4096,
}
```

### Parsing Boot Information

```rust
use helix_multiboot2::{Multiboot2Info, Tag};

// Safe parsing with lifetime-bound references
let boot_info = unsafe { Multiboot2Info::from_ptr(multiboot_ptr)? };

// Type-safe tag iteration
for tag in boot_info.tags() {
    match tag {
        Tag::MemoryMap(mmap) => {
            for region in mmap.regions() {
                println!("Memory: {:x} - {:x} ({:?})",
                    region.start(), region.end(), region.kind());
            }
        }
        Tag::Cmdline(cmdline) => {
            println!("Command line: {}", cmdline.as_str());
        }
        Tag::Framebuffer(fb) => {
            println!("Framebuffer: {}x{}", fb.width(), fb.height());
        }
        _ => {}
    }
}
```

### Unified BootInfo (protocol-agnostic)

```rust
use helix_multiboot2::BootInfo;

// Works with Multiboot2, future Limine, UEFI...
fn kernel_main(boot_info: &BootInfo) {
    let memory_map = boot_info.memory_map();
    let cmdline = boot_info.cmdline();
    let framebuffer = boot_info.framebuffer();
}
```

## 🔒 Safety Model

### Unsafe Boundary

Only ONE unsafe operation is required:
```rust
// The only unsafe call - raw pointer interpretation
let info = unsafe { Multiboot2Info::from_ptr(ptr)? };

// Everything else is safe!
for tag in info.tags() { /* safe iteration */ }
```

### Invariants Guaranteed

1. **Alignment**: All tags are 8-byte aligned (spec requirement)
2. **Bounds**: Tag sizes are validated against total size
3. **Lifetimes**: Parsed data cannot outlive boot information
4. **Types**: Tags are exhaustively typed with `#[non_exhaustive]`

## 📊 Memory Layout

```
Multiboot2 Information Structure:
┌─────────────────────────────────────────┐
│ total_size: u32                         │ ← Total size including this field
│ reserved: u32                           │
├─────────────────────────────────────────┤
│ Tag 1: type=X, size=Y                   │ ← 8-byte aligned
│   ... tag data ...                      │
│   ... padding to 8-byte alignment ...   │
├─────────────────────────────────────────┤
│ Tag 2: type=X, size=Y                   │
│   ... tag data ...                      │
├─────────────────────────────────────────┤
│ ...                                     │
├─────────────────────────────────────────┤
│ End Tag: type=0, size=8                 │ ← Terminator
└─────────────────────────────────────────┘
```

## 🎨 Revolutionary Features

### 1. Compile-Time Header Validation
```rust
// Checksum is computed at compile time - invalid headers won't compile!
const HEADER: Multiboot2Header = Multiboot2Header::new()
    .with_checksum_validated(); // Compile error if checksum wrong
```

### 2. Zero-Copy Memory Map
```rust
// No allocation needed - direct memory access
let mmap = boot_info.memory_map();
for region in mmap.usable_regions() {
    // region is a view into boot info memory
}
```

### 3. Protocol Abstraction Layer
```rust
// Future: same code works with different boot protocols
trait BootProtocol {
    fn memory_map(&self) -> impl Iterator<Item = MemoryRegion>;
    fn cmdline(&self) -> Option<&str>;
}

impl BootProtocol for Multiboot2Info<'_> { /* ... */ }
impl BootProtocol for LimineInfo<'_> { /* future */ }
impl BootProtocol for UefiInfo<'_> { /* future */ }
```

## License

MIT OR Apache-2.0
