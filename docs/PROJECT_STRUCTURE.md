# Helix OS Framework - Structure du Projet

## Arborescence Complète

```
helix/
├── Cargo.toml                          # Workspace manifest
├── rust-toolchain.toml                 # Nightly Rust configuration
├── .cargo/
│   └── config.toml                     # Cargo configuration (target, linker)
│
├── docs/                               # Documentation du framework
│   ├── ARCHITECTURE.md                 # Architecture conceptuelle
│   ├── PROJECT_STRUCTURE.md            # Ce fichier
│   ├── MODULE_GUIDE.md                 # Guide de création de modules
│   ├── OS_BUILDER_GUIDE.md             # Guide de création d'OS
│   └── API_REFERENCE.md                # Référence API
│
├── boot/                               # ═══ LAYER 1: BOOT ═══
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs                      # Bootloader protocol abstraction
│   ├── multiboot2/                     # Multiboot2 protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── header.rs
│   ├── limine/                         # Limine protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   └── uefi/                           # UEFI direct boot
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
│
├── hal/                                # ═══ LAYER 1: HAL ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                      # HAL trait definitions
│   │   ├── cpu.rs                      # CPU abstraction traits
│   │   ├── mmu.rs                      # MMU abstraction traits
│   │   ├── interrupts.rs               # Interrupt controller traits
│   │   └── firmware.rs                 # Firmware interface traits
│   │
│   ├── arch/                           # Architecture implementations
│   │   ├── x86_64/
│   │   │   ├── Cargo.toml
│   │   │   ├── src/
│   │   │   │   ├── lib.rs
│   │   │   │   ├── cpu.rs              # x86_64 CPU implementation
│   │   │   │   ├── gdt.rs              # Global Descriptor Table
│   │   │   │   ├── idt.rs              # Interrupt Descriptor Table
│   │   │   │   ├── paging.rs           # x86_64 paging (4/5 level)
│   │   │   │   ├── apic.rs             # APIC implementation
│   │   │   │   ├── tss.rs              # Task State Segment
│   │   │   │   └── instructions.rs     # CPU instructions wrappers
│   │   │   └── asm/
│   │   │       ├── boot.s              # Early boot assembly
│   │   │       ├── context_switch.s    # Context switch
│   │   │       ├── interrupts.s        # Interrupt stubs
│   │   │       └── syscall.s           # Syscall entry
│   │   │
│   │   ├── aarch64/
│   │   │   ├── Cargo.toml
│   │   │   ├── src/
│   │   │   │   ├── lib.rs
│   │   │   │   ├── cpu.rs              # AArch64 CPU implementation
│   │   │   │   ├── mmu.rs              # ARM MMU
│   │   │   │   ├── gic.rs              # Generic Interrupt Controller
│   │   │   │   ├── exceptions.rs       # Exception handling
│   │   │   │   └── psci.rs             # Power State Coordination
│   │   │   └── asm/
│   │   │       ├── boot.s
│   │   │       ├── vectors.s
│   │   │       └── context_switch.s
│   │   │
│   │   └── riscv64/
│   │       ├── Cargo.toml
│   │       ├── src/
│   │       │   ├── lib.rs
│   │       │   ├── cpu.rs
│   │       │   ├── paging.rs           # Sv39/Sv48/Sv57
│   │       │   ├── plic.rs             # Platform-Level Interrupt Controller
│   │       │   └── sbi.rs              # Supervisor Binary Interface
│   │       └── asm/
│   │           ├── boot.s
│   │           └── trap.s
│   │
│   └── firmware/                       # Firmware interfaces
│       ├── acpi/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       └── lib.rs
│       └── devicetree/
│           ├── Cargo.toml
│           └── src/
│               └── lib.rs
│
├── core/                               # ═══ LAYER 2: KERNEL CORE ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                      # Core kernel entry
│   │   │
│   │   ├── orchestrator/               # Kernel orchestrator
│   │   │   ├── mod.rs
│   │   │   ├── lifecycle.rs            # System lifecycle (boot, shutdown)
│   │   │   ├── capability_broker.rs    # Capability distribution
│   │   │   ├── resource_broker.rs      # Resource management
│   │   │   └── panic_handler.rs        # Kernel panic handling
│   │   │
│   │   ├── syscall/                    # Syscall framework
│   │   │   ├── mod.rs
│   │   │   ├── gateway.rs              # Syscall entry point
│   │   │   ├── dispatcher.rs           # Syscall routing
│   │   │   ├── validation.rs           # Argument validation
│   │   │   └── registry.rs             # Syscall registration
│   │   │
│   │   ├── interrupts/                 # Interrupt management
│   │   │   ├── mod.rs
│   │   │   ├── router.rs               # Interrupt routing
│   │   │   ├── handlers.rs             # Handler registration
│   │   │   └── exceptions.rs           # Exception handling
│   │   │
│   │   └── debug/                      # Debug interface
│   │       ├── mod.rs
│   │       ├── console.rs              # Debug console
│   │       ├── gdb_stub.rs             # GDB remote debugging
│   │       └── kprobes.rs              # Kernel probes
│
├── subsystems/                         # ═══ LAYER 3: SUBSYSTEMS ═══
│   │
│   ├── execution/                      # Execution subsystem
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   │
│   │   │   ├── scheduler/              # Scheduler FRAMEWORK
│   │   │   │   ├── mod.rs
│   │   │   │   ├── traits.rs           # Scheduler trait definitions
│   │   │   │   ├── queue.rs            # Run queue abstraction
│   │   │   │   ├── priority.rs         # Priority management
│   │   │   │   └── metrics.rs          # Scheduler metrics
│   │   │   │
│   │   │   ├── domains/                # Execution domains
│   │   │   │   ├── mod.rs
│   │   │   │   ├── domain.rs           # Domain abstraction
│   │   │   │   ├── kernel_domain.rs    # Kernel execution domain
│   │   │   │   └── user_domain.rs      # User execution domain
│   │   │   │
│   │   │   ├── context/                # Context management
│   │   │   │   ├── mod.rs
│   │   │   │   ├── cpu_context.rs      # CPU state
│   │   │   │   ├── fpu_context.rs      # FPU/SIMD state
│   │   │   │   └── switch.rs           # Context switch logic
│   │   │   │
│   │   │   ├── thread/                 # Thread abstraction
│   │   │   │   ├── mod.rs
│   │   │   │   ├── thread.rs           # Thread structure
│   │   │   │   ├── registry.rs         # Thread registry
│   │   │   │   ├── local_storage.rs    # Thread-local storage
│   │   │   │   └── states.rs           # Thread states
│   │   │   │
│   │   │   └── process/                # Process abstraction
│   │   │       ├── mod.rs
│   │   │       ├── process.rs          # Process structure
│   │   │       ├── registry.rs         # Process registry
│   │   │       └── hierarchy.rs        # Process tree
│   │
│   ├── memory/                         # Memory subsystem
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   │
│   │   │   ├── model/                  # Memory model
│   │   │   │   ├── mod.rs
│   │   │   │   ├── address.rs          # Address types
│   │   │   │   ├── regions.rs          # Memory regions
│   │   │   │   └── permissions.rs      # Memory permissions
│   │   │   │
│   │   │   ├── physical/               # Physical memory
│   │   │   │   ├── mod.rs
│   │   │   │   ├── frame_allocator.rs  # Frame allocator trait
│   │   │   │   ├── buddy.rs            # Buddy allocator impl
│   │   │   │   ├── bitmap.rs           # Bitmap allocator impl
│   │   │   │   └── numa.rs             # NUMA awareness
│   │   │   │
│   │   │   ├── virtual/                # Virtual memory
│   │   │   │   ├── mod.rs
│   │   │   │   ├── address_space.rs    # Address space management
│   │   │   │   ├── mapper.rs           # Page mapping
│   │   │   │   ├── vmm.rs              # Virtual memory manager
│   │   │   │   └── demand_paging.rs    # Demand paging
│   │   │   │
│   │   │   ├── heap/                   # Kernel heap
│   │   │   │   ├── mod.rs
│   │   │   │   ├── allocator.rs        # Heap allocator trait
│   │   │   │   ├── slab.rs             # Slab allocator
│   │   │   │   └── bump.rs             # Bump allocator (early boot)
│   │   │   │
│   │   │   └── isolation/              # Memory isolation
│   │   │       ├── mod.rs
│   │   │       ├── domains.rs          # Isolation domains
│   │   │       └── protection.rs       # Memory protection keys
│   │
│   ├── io/                             # I/O subsystem
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   │
│   │   │   ├── driver/                 # Driver framework
│   │   │   │   ├── mod.rs
│   │   │   │   ├── traits.rs           # Driver trait definitions
│   │   │   │   ├── lifecycle.rs        # Driver lifecycle
│   │   │   │   ├── binding.rs          # Device-driver binding
│   │   │   │   └── power.rs            # Power management
│   │   │   │
│   │   │   ├── device/                 # Device abstraction
│   │   │   │   ├── mod.rs
│   │   │   │   ├── device.rs           # Device structure
│   │   │   │   ├── graph.rs            # Device tree/graph
│   │   │   │   ├── discovery.rs        # Device discovery
│   │   │   │   └── classes.rs          # Device classes
│   │   │   │
│   │   │   ├── bus/                    # Bus abstraction
│   │   │   │   ├── mod.rs
│   │   │   │   ├── traits.rs           # Bus traits
│   │   │   │   ├── pci.rs              # PCI bus support
│   │   │   │   └── platform.rs         # Platform bus
│   │   │   │
│   │   │   └── buffer/                 # Buffer management
│   │   │       ├── mod.rs
│   │   │       ├── dma.rs              # DMA buffers
│   │   │       └── ring.rs             # Ring buffers
│   │
│   ├── security/                       # Security subsystem
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   │
│   │   │   ├── capability/             # Capability system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── capability.rs       # Capability definition
│   │   │   │   ├── token.rs            # Capability tokens
│   │   │   │   ├── space.rs            # Capability space
│   │   │   │   └── transfer.rs         # Capability transfer
│   │   │   │
│   │   │   ├── sandbox/                # Sandboxing
│   │   │   │   ├── mod.rs
│   │   │   │   ├── sandbox.rs          # Sandbox abstraction
│   │   │   │   ├── policy.rs           # Sandbox policies
│   │   │   │   └── seccomp.rs          # Syscall filtering
│   │   │   │
│   │   │   ├── trust/                  # Trust zones
│   │   │   │   ├── mod.rs
│   │   │   │   ├── zones.rs            # Trust zone definitions
│   │   │   │   └── attestation.rs      # Attestation
│   │   │   │
│   │   │   ├── crypto/                 # Crypto primitives
│   │   │   │   ├── mod.rs
│   │   │   │   ├── random.rs           # Random number generation
│   │   │   │   ├── hash.rs             # Hashing
│   │   │   │   └── keys.rs             # Key management
│   │   │   │
│   │   │   └── audit/                  # Audit system
│   │   │       ├── mod.rs
│   │   │       ├── logger.rs           # Audit logging
│   │   │       └── events.rs           # Audit events
│   │
│   ├── communication/                  # Communication subsystem
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   │
│   │   │   ├── ipc/                    # Inter-process communication
│   │   │   │   ├── mod.rs
│   │   │   │   ├── channel.rs          # IPC channels
│   │   │   │   ├── endpoint.rs         # IPC endpoints
│   │   │   │   ├── message.rs          # IPC messages
│   │   │   │   └── ports.rs            # Port-based IPC
│   │   │   │
│   │   │   ├── events/                 # Event system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── event.rs            # Event definitions
│   │   │   │   ├── dispatcher.rs       # Event dispatcher
│   │   │   │   └── handlers.rs         # Event handlers
│   │   │   │
│   │   │   ├── signals/                # Signal system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── signal.rs           # Signal definitions
│   │   │   │   └── delivery.rs         # Signal delivery
│   │   │   │
│   │   │   └── pubsub/                 # Publish/Subscribe
│   │   │       ├── mod.rs
│   │   │       ├── broker.rs           # Message broker
│   │   │       └── topics.rs           # Topic management
│   │
│   └── time/                           # Time subsystem
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   │
│       │   ├── clock/                  # Clock sources
│       │   │   ├── mod.rs
│       │   │   ├── traits.rs           # Clock traits
│       │   │   └── monotonic.rs        # Monotonic clock
│       │   │
│       │   ├── timer/                  # Timer framework
│       │   │   ├── mod.rs
│       │   │   ├── traits.rs           # Timer traits
│       │   │   ├── wheel.rs            # Timer wheel
│       │   │   └── hrtimer.rs          # High-resolution timers
│       │   │
│       │   └── watchdog/               # Watchdog
│       │       ├── mod.rs
│       │       └── traits.rs
│
├── modules/                            # ═══ LAYER 4: MODULE SYSTEM ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   │
│   │   ├── loader/                     # Module loader
│   │   │   ├── mod.rs
│   │   │   ├── elf.rs                  # ELF loading
│   │   │   ├── relocation.rs           # Relocation handling
│   │   │   └── symbols.rs              # Symbol resolution
│   │   │
│   │   ├── registry/                   # Module registry
│   │   │   ├── mod.rs
│   │   │   ├── catalog.rs              # Module catalog
│   │   │   ├── metadata.rs             # Module metadata
│   │   │   └── queries.rs              # Registry queries
│   │   │
│   │   ├── dependencies/               # Dependency management
│   │   │   ├── mod.rs
│   │   │   ├── resolver.rs             # Dependency resolver
│   │   │   ├── graph.rs                # Dependency graph
│   │   │   ├── version.rs              # Semantic versioning
│   │   │   └── conflicts.rs            # Conflict detection
│   │   │
│   │   ├── abi/                        # ABI management
│   │   │   ├── mod.rs
│   │   │   ├── version.rs              # ABI versioning
│   │   │   ├── compatibility.rs        # Compatibility checking
│   │   │   └── shims.rs                # Compatibility shims
│   │   │
│   │   ├── hot_reload/                 # Hot reload engine
│   │   │   ├── mod.rs
│   │   │   ├── state.rs                # State preservation
│   │   │   ├── migration.rs            # State migration
│   │   │   ├── rollback.rs             # Rollback mechanism
│   │   │   └── patching.rs             # Live patching
│   │   │
│   │   └── interface/                  # Module interface
│   │       ├── mod.rs
│   │       ├── traits.rs               # Module traits
│   │       └── macros.rs               # Helper macros
│
├── ipc/                                # ═══ LAYER 5: MESSAGE BUS ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   │
│   │   ├── fabric/                     # Helix Message Fabric
│   │   │   ├── mod.rs
│   │   │   ├── core.rs                 # Fabric core
│   │   │   └── config.rs               # Fabric configuration
│   │   │
│   │   ├── channels/                   # Channel types
│   │   │   ├── mod.rs
│   │   │   ├── sync.rs                 # Synchronous channels
│   │   │   ├── async_channel.rs        # Asynchronous channels
│   │   │   ├── broadcast.rs            # Broadcast channels
│   │   │   └── shared_memory.rs        # Shared memory channels
│   │   │
│   │   ├── routing/                    # Message routing
│   │   │   ├── mod.rs
│   │   │   ├── router.rs               # Message router
│   │   │   ├── addressing.rs           # Address resolution
│   │   │   └── qos.rs                  # Quality of service
│   │   │
│   │   ├── serialization/              # Message serialization
│   │   │   ├── mod.rs
│   │   │   ├── encoder.rs              # Message encoding
│   │   │   └── decoder.rs              # Message decoding
│   │   │
│   │   └── validation/                 # Message validation
│   │       ├── mod.rs
│   │       └── schema.rs               # Schema validation
│
├── policy/                             # ═══ LAYER 6: POLICY LAYER ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   │
│   │   ├── security/                   # Security policies
│   │   │   ├── mod.rs
│   │   │   ├── access_control.rs       # Access control
│   │   │   ├── mandatory.rs            # Mandatory access control
│   │   │   └── discretionary.rs        # Discretionary access control
│   │   │
│   │   ├── resource/                   # Resource policies
│   │   │   ├── mod.rs
│   │   │   ├── quotas.rs               # Resource quotas
│   │   │   ├── limits.rs               # Resource limits
│   │   │   └── reservations.rs         # Resource reservations
│   │   │
│   │   ├── scheduling/                 # Scheduling policies
│   │   │   ├── mod.rs
│   │   │   ├── priority.rs             # Priority policies
│   │   │   ├── fairness.rs             # Fairness policies
│   │   │   └── realtime.rs             # Real-time policies
│   │   │
│   │   └── isolation/                  # Isolation policies
│   │       ├── mod.rs
│   │       ├── namespaces.rs           # Namespace policies
│   │       └── containers.rs           # Container policies
│
├── interface/                          # ═══ LAYER 7: USERLAND INTERFACE ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   │
│   │   ├── native/                     # Native API
│   │   │   ├── mod.rs
│   │   │   ├── syscalls.rs             # Native syscalls
│   │   │   └── types.rs                # Native types
│   │   │
│   │   ├── posix/                      # POSIX compatibility
│   │   │   ├── mod.rs
│   │   │   ├── shim.rs                 # POSIX shim
│   │   │   ├── errno.rs                # Error codes
│   │   │   └── syscalls/               # POSIX syscalls
│   │   │       ├── mod.rs
│   │   │       ├── file.rs
│   │   │       ├── process.rs
│   │   │       └── memory.rs
│   │   │
│   │   ├── wasm/                       # WASM runtime interface
│   │   │   ├── mod.rs
│   │   │   └── runtime.rs
│   │   │
│   │   └── ffi/                        # FFI bridge
│   │       ├── mod.rs
│   │       └── c_abi.rs                # C ABI exports
│
├── sdk/                                # ═══ KERNEL SDK ═══
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   │
│   │   ├── prelude.rs                  # SDK prelude
│   │   │
│   │   ├── module/                     # Module SDK
│   │   │   ├── mod.rs
│   │   │   ├── builder.rs              # Module builder
│   │   │   ├── testing.rs              # Module testing framework
│   │   │   └── examples/               # Example modules
│   │   │
│   │   ├── driver/                     # Driver SDK
│   │   │   ├── mod.rs
│   │   │   ├── builder.rs              # Driver builder
│   │   │   └── templates/              # Driver templates
│   │   │
│   │   └── os/                         # OS Builder SDK
│   │       ├── mod.rs
│   │       ├── profile.rs              # OS profile builder
│   │       ├── config.rs               # OS configuration
│   │       └── templates/              # OS templates
│
├── profiles/                           # ═══ OS PROFILES ═══
│   │
│   ├── minimal/                        # Minimal OS profile
│   │   ├── Cargo.toml
│   │   ├── helix.toml                  # Profile configuration
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── desktop/                        # Desktop OS profile
│   │   ├── Cargo.toml
│   │   ├── helix.toml
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── server/                         # Server OS profile
│   │   ├── Cargo.toml
│   │   ├── helix.toml
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── embedded/                       # Embedded OS profile
│   │   ├── Cargo.toml
│   │   ├── helix.toml
│   │   └── src/
│   │       └── main.rs
│   │
│   └── secure/                         # Secure OS profile
│       ├── Cargo.toml
│       ├── helix.toml
│       └── src/
│           └── main.rs
│
├── modules_impl/                       # ═══ MODULE IMPLEMENTATIONS ═══
│   │
│   ├── schedulers/                     # Scheduler implementations
│   │   ├── round_robin/
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   ├── cfs/                        # Completely Fair Scheduler
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   ├── realtime/                   # Real-time scheduler
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   └── cooperative/                # Cooperative scheduler
│   │       ├── Cargo.toml
│   │       └── src/lib.rs
│   │
│   ├── allocators/                     # Allocator implementations
│   │   ├── buddy/
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   ├── slab/
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   └── tlsf/                       # Two-Level Segregated Fit
│   │       ├── Cargo.toml
│   │       └── src/lib.rs
│   │
│   ├── filesystems/                    # Filesystem implementations
│   │   ├── ramfs/
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   ├── ext4/
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   └── fat32/
│   │       ├── Cargo.toml
│   │       └── src/lib.rs
│   │
│   └── drivers/                        # Driver implementations
│       ├── serial/
│       │   ├── Cargo.toml
│       │   └── src/lib.rs
│       ├── keyboard/
│       │   ├── Cargo.toml
│       │   └── src/lib.rs
│       └── virtio/
│           ├── Cargo.toml
│           └── src/lib.rs
│
├── tools/                              # ═══ BUILD TOOLS ═══
│   ├── helix-build/                    # Build system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── helix-pack/                     # Module packager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── helix-test/                     # Test framework
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
└── tests/                              # ═══ TESTS ═══
    ├── integration/                    # Integration tests
    │   └── ...
    ├── unit/                           # Unit tests
    │   └── ...
    └── fuzz/                           # Fuzz tests
        └── ...
```

## Justification de la Structure

### Pourquoi cette hiérarchie ?

1. **Séparation par couche** : Chaque couche de l'architecture est un dossier distinct
2. **Séparation framework/implémentation** : `subsystems/` contient les frameworks, `modules_impl/` les implémentations
3. **Profiles vs Modules** : Les profiles composent des modules, ils ne les définissent pas
4. **SDK distinct** : Le SDK est une crate séparée pour faciliter le développement externe

### Conventions

- Un dossier = une crate Cargo (sauf organisation)
- `mod.rs` pour les modules avec sous-modules
- `lib.rs` pour les bibliothèques
- `traits.rs` pour les définitions de traits

### Dépendances entre crates

```
                    profiles/
                       │
                       ▼
    ┌──────────────────┼──────────────────┐
    │                  │                  │
    ▼                  ▼                  ▼
modules_impl/       policy/          interface/
    │                  │                  │
    └──────────────────┼──────────────────┘
                       │
                       ▼
                    modules/
                       │
                       ▼
                     ipc/
                       │
                       ▼
                 subsystems/
                       │
                       ▼
                    core/
                       │
                       ▼
        ┌──────────────┴──────────────┐
        │                             │
        ▼                             ▼
      hal/                          boot/
        │
        ▼
    hal/arch/*
```
