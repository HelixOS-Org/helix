# Helix OS Framework Kernel - Architecture Conceptuelle

```
╔══════════════════════════════════════════════════════════════════════════════════════════════════════╗
║                              HELIX OS FRAMEWORK KERNEL                                               ║
║                    "Un framework de création de systèmes d'exploitation"                             ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝

┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                    LAYER 7: USERLAND INTERFACE                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                     │
│  │ POSIX Shim  │ │ Native API  │ │ WASM Runtime│ │ FFI Bridge  │ │ SDK Client  │                     │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘                     │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                                    LAYER 6: POLICY LAYER                                             │
│  ┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐             │
│  │ SecurityPolicies  │ │ ResourcePolicies  │ │ SchedulingPolicy  │ │ IsolationPolicy   │             │
│  │  ├─ AccessControl │ │  ├─ Quotas        │ │  ├─ Priority      │ │  ├─ Sandboxing    │             │
│  │  ├─ Capabilities  │ │  ├─ Limits        │ │  ├─ Fairness      │ │  ├─ Namespaces    │             │
│  │  └─ AuditRules    │ │  └─ Reservations  │ │  └─ Deadlines     │ │  └─ TrustZones    │             │
│  └───────────────────┘ └───────────────────┘ └───────────────────┘ └───────────────────┘             │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                                LAYER 5: IPC / MESSAGE BUS UNIVERSEL                                  │
│                                                                                                      │
│  ┌─────────────────────────────────────────────────────────────────────────────────────────────────┐ │
│  │                              HELIX MESSAGE FABRIC (HMF)                                         │ │
│  │  ┌───────────────┐ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐      │ │
│  │  │ SyncChannel   │ │ AsyncChannel  │ │ BroadcastBus  │ │ EventStream   │ │ SharedMemory  │      │ │
│  │  └───────┬───────┘ └───────┬───────┘ └───────┬───────┘ └───────┬───────┘ └───────┬───────┘      │ │
│  │          └─────────────────┴─────────────────┴─────────────────┴─────────────────┘              │ │
│  │                                              │                                                  │ │
│  │  ┌───────────────────────────────────────────▼───────────────────────────────────────────────┐  │ │
│  │  │                            MESSAGE ROUTER & DISPATCHER                                    │  │ │
│  │  │   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐         │  │ │
│  │  │   │ Addressing  │ │ Serialization│ │ Validation  │ │ Routing     │ │ QoS Manager │         │  │ │
│  │  │   └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘         │  │ │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                              LAYER 4: MODULE RUNTIME & LOADER                                        │
│                                                                                                      │
│  ┌─────────────────────────────────┐     ┌─────────────────────────────────┐                         │
│  │       MODULE REGISTRY           │     │      DEPENDENCY RESOLVER        │                         │
│  │  ┌─────────────────────────┐    │     │  ┌─────────────────────────┐    │                         │
│  │  │ StaticModules           │    │     │  │ VersionConstraints      │    │                         │
│  │  │ DynamicModules          │    │     │  │ CapabilityRequirements  │    │                         │
│  │  │ UserSpaceModules        │    │     │  │ DependencyGraph         │    │                         │
│  │  │ KernelSpaceModules      │    │     │  │ ConflictDetection       │    │                         │
│  │  └─────────────────────────┘    │     │  └─────────────────────────┘    │                         │
│  └─────────────────────────────────┘     └─────────────────────────────────┘                         │
│                                                                                                      │
│  ┌─────────────────────────────────┐     ┌─────────────────────────────────┐                         │
│  │       HOT RELOAD ENGINE         │     │      ABI COMPATIBILITY          │                         │
│  │  ┌─────────────────────────┐    │     │  ┌─────────────────────────┐    │                         │
│  │  │ StatePreservation       │    │     │  │ ABIVersioning           │    │                         │
│  │  │ GracefulMigration       │    │     │  │ SymbolResolution        │    │                         │
│  │  │ RollbackMechanism       │    │     │  │ CompatibilityShims      │    │                         │
│  │  │ LivePatching            │    │     │  │ DeprecationWarnings     │    │                         │
│  │  └─────────────────────────┘    │     │  └─────────────────────────┘    │                         │
│  └─────────────────────────────────┘     └─────────────────────────────────┘                         │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                              LAYER 3: SUBSYSTEMS FRAMEWORK                                           │
│                                                                                                      │
│  ┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐                    │
│  │    EXECUTION ENGINE    │ │    MEMORY SUBSYSTEM    │ │    I/O SUBSYSTEM       │                    │
│  │  ┌──────────────────┐  │ │  ┌──────────────────┐  │ │  ┌──────────────────┐  │                    │
│  │  │ SchedulerFw      │  │ │  │ MemoryModel      │  │ │  │ DriverFramework  │  │                    │
│  │  │ ExecutionDomains │  │ │  │ AllocatorFw      │  │ │  │ DeviceGraph      │  │                    │
│  │  │ ContextSwitch    │  │ │  │ VirtualMemory    │  │ │  │ BufferManagement │  │                    │
│  │  │ ThreadRegistry   │  │ │  │ IsolationDomains │  │ │  │ DMAController    │  │                    │
│  │  └──────────────────┘  │ │  └──────────────────┘  │ │  └──────────────────┘  │                    │
│  └────────────────────────┘ └────────────────────────┘ └────────────────────────┘                    │
│                                                                                                      │
│  ┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐                    │
│  │  SECURITY SUBSYSTEM    │ │ COMMUNICATION SUBSYS   │ │  TIME SUBSYSTEM        │                    │
│  │  ┌──────────────────┐  │ │  ┌──────────────────┐  │ │  ┌──────────────────┐  │                    │
│  │  │ CapabilityEngine │  │ │  │ IPCCore          │  │ │  │ ClockSource      │  │                    │
│  │  │ SandboxRuntime   │  │ │  │ SignalDispatcher │  │ │  │ TimerFramework   │  │                    │
│  │  │ TrustZones       │  │ │  │ EventSystem      │  │ │  │ TimeSync         │  │                    │
│  │  │ CryptoProvider   │  │ │  │ PubSubBroker     │  │ │  │ Watchdogs        │  │                    │
│  │  └──────────────────┘  │ │  └──────────────────┘  │ │  └──────────────────┘  │                    │
│  └────────────────────────┘ └────────────────────────┘ └────────────────────────┘                    │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                              LAYER 2: KERNEL CORE (ORCHESTRATOR)                                     │
│                                                                                                      │
│  ┌────────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │                              KERNEL ORCHESTRATOR                                               │  │
│  │                                                                                                │  │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐               │  │
│  │  │ LifecycleManager│ │CapabilityBroker │ │ ResourceBroker  │ │ PanicHandler    │               │  │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘               │  │
│  │                                                                                                │  │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐               │  │
│  │  │ SyscallGateway  │ │ InterruptRouter │ │ ExceptionHandler│ │ DebugInterface  │               │  │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘               │  │
│  │                                                                                                │  │
│  │  ┌───────────────────────────────────────────────────────────────────────────────────────┐    │  │
│  │  │                         MINIMAL TRUSTED COMPUTING BASE (TCB)                          │    │  │
│  │  │   • Capability validation   • Memory protection primitives   • Context switching      │    │  │
│  │  │   • Interrupt dispatch      • Exception handling             • Panic recovery         │    │  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────┘    │  │
│  └────────────────────────────────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
                                                             │
┌────────────────────────────────────────────────────────────▼─────────────────────────────────────────┐
│                           LAYER 1: BOOT & HARDWARE ABSTRACTION LAYER (HAL)                           │
│                                                                                                      │
│  ┌────────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │                              HARDWARE ABSTRACTION LAYER                                        │  │
│  │                                                                                                │  │
│  │  ┌───────────────────────────────────────────────────────────────────────────────────────────┐│  │
│  │  │                              ARCHITECTURE ADAPTERS                                        ││  │
│  │  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     ││  │
│  │  │  │ x86_64       │ │ aarch64      │ │ riscv64      │ │ loongarch64  │ │ ...          │     ││  │
│  │  │  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘     ││  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────┘│  │
│  │                                                                                                │  │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐               │  │
│  │  │ CPU Primitives  │ │ MMU Abstraction │ │ InterruptCtrl   │ │ FirmwareInterface│              │  │
│  │  │ • Registers     │ │ • PageTables    │ │ • PIC/APIC/GIC  │ │ • UEFI/BIOS     │               │  │
│  │  │ • Atomics       │ │ • TLB           │ │ • MSI/MSI-X     │ │ • DeviceTree    │               │  │
│  │  │ • Barriers      │ │ • ASID          │ │ • IPI           │ │ • ACPI          │               │  │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘               │  │
│  └────────────────────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                                      │
│  ┌────────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │                              BOOTLOADER INTERFACE                                              │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                           │  │
│  │  │ Multiboot2   │ │ Limine       │ │ UEFI Direct  │ │ Custom       │                           │  │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘                           │  │
│  │                                                                                                │  │
│  │  ┌──────────────────────────────────────────────────────────────────────────────┐              │  │
│  │  │  EARLY BOOT (ASM)                                                            │              │  │
│  │  │  • Stack setup • GDT/IDT • Paging init • BSP handoff • AP wakeup             │              │  │
│  │  └──────────────────────────────────────────────────────────────────────────────┘              │  │
│  └────────────────────────────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────┘

════════════════════════════════════════════════════════════════════════════════════════════════════════
                                        CROSS-CUTTING CONCERNS
════════════════════════════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                         OBSERVABILITY                                               │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐               │
│  │ Tracing      │ │ Metrics      │ │ Logging      │ │ Profiling    │ │ Debugging    │               │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘               │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                         CONFIGURATION                                               │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐               │
│  │ Boot Config  │ │ Runtime Cfg  │ │ Feature Flags│ │ Tuning Params│ │ OS Profiles  │               │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘               │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Philosophie Architecturale

### Pourquoi cette complexité ?

1. **Séparation Mécanisme/Politique** : Le kernel fournit les mécanismes, les modules fournissent les politiques
2. **Composition over Inheritance** : Chaque OS est une composition de modules
3. **Isolation Stricte** : Chaque composant est isolé, testable, remplaçable
4. **Évolution Progressive** : L'architecture permet l'évolution sans casser la compatibilité

### Trusted Computing Base (TCB) Minimal

Le TCB de Helix est volontairement minimal :
- ~5000 lignes de Rust critique
- ~500 lignes d'assembleur
- Tout le reste est modulaire et remplaçable

## Description Détaillée des Couches

### Layer 1 : Boot & HAL

**Rôle** : Abstraction matérielle complète, permettant au reste du kernel d'être architecture-agnostic.

**Composants** :
- **Bootloader Interface** : Support Multiboot2, Limine, UEFI
- **Architecture Adapters** : x86_64, aarch64, riscv64
- **CPU Primitives** : Atomics, barriers, registres spéciaux
- **MMU Abstraction** : Interface unifiée pour la pagination
- **Interrupt Controller** : Abstraction APIC/GIC/PLIC

**Justification** : Sans cette couche, chaque module devrait gérer les spécificités hardware.

### Layer 2 : Kernel Core

**Rôle** : Orchestration minimale, gestion du cycle de vie, dispatch des événements.

**Composants** :
- **Lifecycle Manager** : Boot, shutdown, suspend/resume
- **Capability Broker** : Distribution et validation des capabilities
- **Syscall Gateway** : Point d'entrée unique pour les appels système
- **Interrupt Router** : Distribution des interrupts aux handlers

**Pourquoi minimal ?** : Chaque ligne de code dans le core est dans le TCB. Moins de code = moins de surface d'attaque.

### Layer 3 : Subsystems Framework

**Rôle** : Fournir les frameworks pour les sous-systèmes majeurs.

**Subsystems** :
- **Execution** : Scheduler framework, threads, contextes
- **Memory** : Allocators, virtual memory, isolation
- **I/O** : Drivers, devices, buffers
- **Security** : Capabilities, sandboxing, crypto
- **Communication** : IPC, events, pub/sub
- **Time** : Clocks, timers, watchdogs

**Clé** : Ce sont des FRAMEWORKS, pas des implémentations. Les modules fournissent les implémentations.

### Layer 4 : Module Runtime

**Rôle** : Chargement, gestion des dépendances, hot-reload des modules.

**Composants** :
- **Module Registry** : Catalogue de tous les modules
- **Dependency Resolver** : Résolution des dépendances avec versioning
- **Hot Reload Engine** : Remplacement à chaud avec préservation d'état
- **ABI Compatibility** : Gestion des versions ABI

### Layer 5 : Message Bus

**Rôle** : Communication universelle entre tous les composants.

**Helix Message Fabric (HMF)** :
- Channels synchrones et asynchrones
- Broadcast et multicast
- Event streaming
- Shared memory pour les cas critiques

**Principe** : Zero dépendances directes entre modules. Tout passe par le message bus.

### Layer 6 : Policy Layer

**Rôle** : Couche OPTIONNELLE qui définit les politiques.

**Types de politiques** :
- Sécurité (access control, audit)
- Ressources (quotas, limites)
- Scheduling (priorités, deadlines)
- Isolation (sandboxing, namespaces)

**Optionnelle car** : Un OS embarqué minimal peut ne pas avoir de policy layer.

### Layer 7 : Userland Interface

**Rôle** : Interface entre le kernel et l'espace utilisateur.

**Options** :
- POSIX Shim pour compatibilité
- Native API pour performance
- WASM Runtime pour sandboxing
- FFI Bridge pour langages étrangers
