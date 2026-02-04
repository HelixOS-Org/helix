//! VirtIO Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive VirtIO device support for virtualized
//! environments (QEMU, KVM, VMware, etc.) in the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         VirtIO Protocol Stack                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Device Drivers  │  Block  │  Network  │  Console  │  GPU  │  SCSI     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Virtqueue       │  Split Queue  │  Packed Queue  │  Indirect Desc     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Transport       │  PCI  │  MMIO  │  CCW (s390)                         │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware        │  QEMU  │  KVM  │  Hyper-V  │  VMware  │  Xen        │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - VirtIO 1.0/1.1/1.2 specification support
//! - PCI and MMIO transport layers
//! - Split and packed virtqueue implementations
//! - Block device (disk) support
//! - Network device support
//! - Console device support
//! - GPU device support
//! - SCSI device support
//! - Interrupt handling

#![no_std]

use core::fmt;

// =============================================================================
// VIRTIO CONSTANTS
// =============================================================================

/// VirtIO vendor ID (PCI)
pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;

/// VirtIO subsystem device ID range (legacy)
pub const VIRTIO_LEGACY_DEVICE_ID_START: u16 = 0x1000;
pub const VIRTIO_LEGACY_DEVICE_ID_END: u16 = 0x103F;

/// VirtIO subsystem device ID range (modern)
pub const VIRTIO_MODERN_DEVICE_ID_START: u16 = 0x1040;
pub const VIRTIO_MODERN_DEVICE_ID_END: u16 = 0x107F;

/// Maximum number of virtqueues per device
pub const MAX_VIRTQUEUES: usize = 8;

/// Default queue size
pub const DEFAULT_QUEUE_SIZE: u16 = 256;

/// Maximum queue size
pub const MAX_QUEUE_SIZE: u16 = 32768;

/// VirtIO MMIO magic value
pub const VIRTIO_MMIO_MAGIC: u32 = 0x74726976; // "virt"

/// VirtIO version (1.x)
pub const VIRTIO_VERSION_1: u32 = 1;
pub const VIRTIO_VERSION_2: u32 = 2;

// =============================================================================
// VIRTIO DEVICE TYPES
// =============================================================================

/// VirtIO device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VirtioDeviceType {
    /// Invalid device
    Invalid           = 0,
    /// Network device
    Network           = 1,
    /// Block device (disk)
    Block             = 2,
    /// Console device
    Console           = 3,
    /// Entropy source (RNG)
    Entropy           = 4,
    /// Memory balloon
    Balloon           = 5,
    /// IO memory
    IoMemory          = 6,
    /// RPMSG device
    Rpmsg             = 7,
    /// SCSI host
    Scsi              = 8,
    /// 9P transport
    NineP             = 9,
    /// MAC 802.11 WLAN
    Wlan              = 10,
    /// RPROC serial
    RprocSerial       = 11,
    /// CAIF device
    Caif              = 12,
    /// Memory balloon
    MemoryBalloon     = 13,
    /// GPU device
    Gpu               = 16,
    /// Timer device
    Timer             = 17,
    /// Input device
    Input             = 18,
    /// Socket device
    Socket            = 19,
    /// Crypto device
    Crypto            = 20,
    /// Signal distribution module
    SignalDist        = 21,
    /// PSTORE device
    Pstore            = 22,
    /// IOMMU device
    Iommu             = 23,
    /// Memory device
    Memory            = 24,
    /// Audio device
    Audio             = 25,
    /// Filesystem device
    Filesystem        = 26,
    /// PMEM device
    Pmem              = 27,
    /// RPMB device
    Rpmb              = 28,
    /// MAC 802.11 WLAN (alt)
    Mac80211Wlan      = 29,
    /// Video encoder
    VideoEncoder      = 30,
    /// Video decoder
    VideoDecoder      = 31,
    /// SCMI device
    Scmi              = 32,
    /// NitroSecureModule
    NitroSecureModule = 33,
    /// I2C adapter
    I2cAdapter        = 34,
    /// Watchdog
    Watchdog          = 35,
    /// CAN device
    Can               = 36,
    /// Parameter server
    ParamServer       = 38,
    /// Audio policy
    AudioPolicy       = 39,
    /// Bluetooth device
    Bluetooth         = 40,
    /// GPIO device
    Gpio              = 41,
    /// RDMA device
    Rdma              = 42,
}

impl VirtioDeviceType {
    /// Get device type name
    pub const fn name(&self) -> &'static str {
        match self {
            VirtioDeviceType::Invalid => "Invalid",
            VirtioDeviceType::Network => "Network",
            VirtioDeviceType::Block => "Block Device",
            VirtioDeviceType::Console => "Console",
            VirtioDeviceType::Entropy => "Entropy Source",
            VirtioDeviceType::Balloon => "Memory Balloon",
            VirtioDeviceType::IoMemory => "IO Memory",
            VirtioDeviceType::Rpmsg => "RPMSG",
            VirtioDeviceType::Scsi => "SCSI Host",
            VirtioDeviceType::NineP => "9P Transport",
            VirtioDeviceType::Wlan => "WLAN",
            VirtioDeviceType::RprocSerial => "RPROC Serial",
            VirtioDeviceType::Caif => "CAIF",
            VirtioDeviceType::MemoryBalloon => "Memory Balloon",
            VirtioDeviceType::Gpu => "GPU",
            VirtioDeviceType::Timer => "Timer",
            VirtioDeviceType::Input => "Input",
            VirtioDeviceType::Socket => "Socket",
            VirtioDeviceType::Crypto => "Crypto",
            VirtioDeviceType::SignalDist => "Signal Distribution",
            VirtioDeviceType::Pstore => "Pstore",
            VirtioDeviceType::Iommu => "IOMMU",
            VirtioDeviceType::Memory => "Memory",
            VirtioDeviceType::Audio => "Audio",
            VirtioDeviceType::Filesystem => "Filesystem",
            VirtioDeviceType::Pmem => "Persistent Memory",
            VirtioDeviceType::Rpmb => "RPMB",
            VirtioDeviceType::Mac80211Wlan => "MAC80211 WLAN",
            VirtioDeviceType::VideoEncoder => "Video Encoder",
            VirtioDeviceType::VideoDecoder => "Video Decoder",
            VirtioDeviceType::Scmi => "SCMI",
            VirtioDeviceType::NitroSecureModule => "Nitro Secure Module",
            VirtioDeviceType::I2cAdapter => "I2C Adapter",
            VirtioDeviceType::Watchdog => "Watchdog",
            VirtioDeviceType::Can => "CAN",
            VirtioDeviceType::ParamServer => "Parameter Server",
            VirtioDeviceType::AudioPolicy => "Audio Policy",
            VirtioDeviceType::Bluetooth => "Bluetooth",
            VirtioDeviceType::Gpio => "GPIO",
            VirtioDeviceType::Rdma => "RDMA",
        }
    }

    /// Get device type from PCI device ID
    pub const fn from_pci_device_id(device_id: u16) -> Option<Self> {
        // Modern device IDs (1040h + device type)
        if device_id >= VIRTIO_MODERN_DEVICE_ID_START && device_id <= VIRTIO_MODERN_DEVICE_ID_END {
            let device_type = device_id - VIRTIO_MODERN_DEVICE_ID_START;
            return Self::from_u32(device_type as u32);
        }

        // Legacy device IDs (1000h + device type)
        if device_id >= VIRTIO_LEGACY_DEVICE_ID_START && device_id <= VIRTIO_LEGACY_DEVICE_ID_END {
            let device_type = device_id - VIRTIO_LEGACY_DEVICE_ID_START;
            return Self::from_u32(device_type as u32);
        }

        None
    }

    /// Convert from u32
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(VirtioDeviceType::Invalid),
            1 => Some(VirtioDeviceType::Network),
            2 => Some(VirtioDeviceType::Block),
            3 => Some(VirtioDeviceType::Console),
            4 => Some(VirtioDeviceType::Entropy),
            5 => Some(VirtioDeviceType::Balloon),
            6 => Some(VirtioDeviceType::IoMemory),
            7 => Some(VirtioDeviceType::Rpmsg),
            8 => Some(VirtioDeviceType::Scsi),
            9 => Some(VirtioDeviceType::NineP),
            16 => Some(VirtioDeviceType::Gpu),
            18 => Some(VirtioDeviceType::Input),
            19 => Some(VirtioDeviceType::Socket),
            20 => Some(VirtioDeviceType::Crypto),
            26 => Some(VirtioDeviceType::Filesystem),
            _ => None,
        }
    }
}

impl fmt::Display for VirtioDeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// VIRTIO DEVICE STATUS
// =============================================================================

/// VirtIO device status flags
#[derive(Debug, Clone, Copy)]
pub struct DeviceStatus(pub u8);

impl DeviceStatus {
    /// Device reset
    pub const RESET: u8 = 0;
    /// Guest OS has found the device
    pub const ACKNOWLEDGE: u8 = 1;
    /// Guest OS knows how to drive the device
    pub const DRIVER: u8 = 2;
    /// Driver is ready to drive the device
    pub const DRIVER_OK: u8 = 4;
    /// Feature negotiation is complete
    pub const FEATURES_OK: u8 = 8;
    /// Device has experienced an error
    pub const DEVICE_NEEDS_RESET: u8 = 64;
    /// Something went wrong in the guest
    pub const FAILED: u8 = 128;

    /// Create new status
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Create reset status
    pub const fn reset() -> Self {
        Self(Self::RESET)
    }

    /// Check if acknowledged
    pub const fn is_acknowledged(&self) -> bool {
        (self.0 & Self::ACKNOWLEDGE) != 0
    }

    /// Check if driver ready
    pub const fn is_driver(&self) -> bool {
        (self.0 & Self::DRIVER) != 0
    }

    /// Check if features OK
    pub const fn is_features_ok(&self) -> bool {
        (self.0 & Self::FEATURES_OK) != 0
    }

    /// Check if driver OK
    pub const fn is_driver_ok(&self) -> bool {
        (self.0 & Self::DRIVER_OK) != 0
    }

    /// Check if device needs reset
    pub const fn needs_reset(&self) -> bool {
        (self.0 & Self::DEVICE_NEEDS_RESET) != 0
    }

    /// Check if failed
    pub const fn is_failed(&self) -> bool {
        (self.0 & Self::FAILED) != 0
    }

    /// Set acknowledge
    pub fn acknowledge(&mut self) {
        self.0 |= Self::ACKNOWLEDGE;
    }

    /// Set driver
    pub fn set_driver(&mut self) {
        self.0 |= Self::DRIVER;
    }

    /// Set features OK
    pub fn set_features_ok(&mut self) {
        self.0 |= Self::FEATURES_OK;
    }

    /// Set driver OK
    pub fn set_driver_ok(&mut self) {
        self.0 |= Self::DRIVER_OK;
    }

    /// Set failed
    pub fn set_failed(&mut self) {
        self.0 |= Self::FAILED;
    }
}

// =============================================================================
// VIRTIO FEATURE BITS
// =============================================================================

/// Common VirtIO feature bits (bits 24-37, 50-127)
pub mod features {
    /// Ring indirect descriptor support
    pub const RING_INDIRECT_DESC: u64 = 1 << 28;
    /// Ring event index support
    pub const RING_EVENT_IDX: u64 = 1 << 29;
    /// Version 1 (VirtIO 1.0)
    pub const VERSION_1: u64 = 1 << 32;
    /// Access platform
    pub const ACCESS_PLATFORM: u64 = 1 << 33;
    /// Ring packed virtqueue
    pub const RING_PACKED: u64 = 1 << 34;
    /// In-order completions
    pub const IN_ORDER: u64 = 1 << 35;
    /// Order platform
    pub const ORDER_PLATFORM: u64 = 1 << 36;
    /// Single root I/O virtualization
    pub const SR_IOV: u64 = 1 << 37;
    /// Notification data
    pub const NOTIFICATION_DATA: u64 = 1 << 38;
    /// Notification config change
    pub const NOTIF_CONFIG_DATA: u64 = 1 << 39;
    /// Ring reset
    pub const RING_RESET: u64 = 1 << 40;
}

/// VirtIO-Net feature bits
pub mod net_features {
    /// Device has checksum offload
    pub const CSUM: u64 = 1 << 0;
    /// Driver can handle GSO
    pub const GUEST_CSUM: u64 = 1 << 1;
    /// Control channel available
    pub const CTRL_GUEST_OFFLOADS: u64 = 1 << 2;
    /// Device maximum MTU reporting is supported
    pub const MTU: u64 = 1 << 3;
    /// Device has MAC address
    pub const MAC: u64 = 1 << 5;
    /// Guest can handle GSO
    pub const GUEST_TSO4: u64 = 1 << 7;
    /// Guest can handle GSO (IPv6)
    pub const GUEST_TSO6: u64 = 1 << 8;
    /// Guest can handle GSO (ECN)
    pub const GUEST_ECN: u64 = 1 << 9;
    /// Guest can handle UFO
    pub const GUEST_UFO: u64 = 1 << 10;
    /// Device can send TSO4
    pub const HOST_TSO4: u64 = 1 << 11;
    /// Device can send TSO6
    pub const HOST_TSO6: u64 = 1 << 12;
    /// Device can send ECN
    pub const HOST_ECN: u64 = 1 << 13;
    /// Device can send UFO
    pub const HOST_UFO: u64 = 1 << 14;
    /// Device can merge receive buffers
    pub const MRG_RXBUF: u64 = 1 << 15;
    /// Configuration status available
    pub const STATUS: u64 = 1 << 16;
    /// Control channel available
    pub const CTRL_VQ: u64 = 1 << 17;
    /// Control RX mode
    pub const CTRL_RX: u64 = 1 << 18;
    /// Control VLAN filtering
    pub const CTRL_VLAN: u64 = 1 << 19;
    /// Extra RX mode control
    pub const CTRL_RX_EXTRA: u64 = 1 << 20;
    /// Guest can announce
    pub const GUEST_ANNOUNCE: u64 = 1 << 21;
    /// Multiqueue supported
    pub const MQ: u64 = 1 << 22;
    /// Guest can set MAC address
    pub const CTRL_MAC_ADDR: u64 = 1 << 23;
    /// Device supports hash reporting
    pub const HASH_REPORT: u64 = 1 << 57;
    /// Guest receives packets with RSS hash
    pub const RSS: u64 = 1 << 60;
    /// Device RSS with hash configuration
    pub const RSS_EXT: u64 = 1 << 61;
    /// Device standby support
    pub const STANDBY: u64 = 1 << 62;
    /// Device speed/duplex
    pub const SPEED_DUPLEX: u64 = 1 << 63;
}

/// VirtIO-Block feature bits
pub mod blk_features {
    /// Maximum size of any single segment is in size_max
    pub const SIZE_MAX: u64 = 1 << 1;
    /// Maximum number of segments in a request is in seg_max
    pub const SEG_MAX: u64 = 1 << 2;
    /// Device geometry is in geometry
    pub const GEOMETRY: u64 = 1 << 4;
    /// Device is read-only
    pub const RO: u64 = 1 << 5;
    /// Block size is in blk_size
    pub const BLK_SIZE: u64 = 1 << 6;
    /// Device supports flush
    pub const FLUSH: u64 = 1 << 9;
    /// Device topology is in topology
    pub const TOPOLOGY: u64 = 1 << 10;
    /// Device supports multiqueue
    pub const MQ: u64 = 1 << 12;
    /// Device supports discard
    pub const DISCARD: u64 = 1 << 13;
    /// Device supports write zeroes
    pub const WRITE_ZEROES: u64 = 1 << 14;
    /// Device supports lifetime
    pub const LIFETIME: u64 = 1 << 15;
    /// Device supports secure erase
    pub const SECURE_ERASE: u64 = 1 << 16;
}

/// VirtIO-Console feature bits
pub mod console_features {
    /// Device has multiple ports
    pub const MULTIPORT: u64 = 1 << 1;
    /// Device supports emergency write
    pub const EMERG_WRITE: u64 = 1 << 2;
}

/// VirtIO-GPU feature bits
pub mod gpu_features {
    /// Supports VirGL
    pub const VIRGL: u64 = 1 << 0;
    /// Supports EDID
    pub const EDID: u64 = 1 << 1;
    /// Supports resource UUID
    pub const RESOURCE_UUID: u64 = 1 << 2;
    /// Supports resource blob
    pub const RESOURCE_BLOB: u64 = 1 << 3;
    /// Supports context init
    pub const CONTEXT_INIT: u64 = 1 << 4;
}

// =============================================================================
// VIRTQUEUE DESCRIPTOR
// =============================================================================

/// Virtqueue descriptor flags
pub mod desc_flags {
    /// Marks a buffer as continuing via the next field
    pub const NEXT: u16 = 1;
    /// Marks a buffer as write-only (otherwise read-only)
    pub const WRITE: u16 = 2;
    /// Buffer contains a list of indirect descriptors
    pub const INDIRECT: u16 = 4;
}

/// Virtqueue descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqDesc {
    /// Physical address of the buffer
    pub addr: u64,
    /// Length of the buffer
    pub len: u32,
    /// Descriptor flags
    pub flags: u16,
    /// Index of next descriptor in chain
    pub next: u16,
}

impl VirtqDesc {
    /// Create an empty descriptor
    pub const fn new() -> Self {
        Self {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    }

    /// Create a read descriptor
    pub const fn read(addr: u64, len: u32) -> Self {
        Self {
            addr,
            len,
            flags: 0,
            next: 0,
        }
    }

    /// Create a write descriptor
    pub const fn write(addr: u64, len: u32) -> Self {
        Self {
            addr,
            len,
            flags: desc_flags::WRITE,
            next: 0,
        }
    }

    /// Create a read descriptor with next
    pub const fn read_next(addr: u64, len: u32, next: u16) -> Self {
        Self {
            addr,
            len,
            flags: desc_flags::NEXT,
            next,
        }
    }

    /// Create a write descriptor with next
    pub const fn write_next(addr: u64, len: u32, next: u16) -> Self {
        Self {
            addr,
            len,
            flags: desc_flags::WRITE | desc_flags::NEXT,
            next,
        }
    }

    /// Check if has next
    pub const fn has_next(&self) -> bool {
        (self.flags & desc_flags::NEXT) != 0
    }

    /// Check if write-only
    pub const fn is_write(&self) -> bool {
        (self.flags & desc_flags::WRITE) != 0
    }

    /// Check if indirect
    pub const fn is_indirect(&self) -> bool {
        (self.flags & desc_flags::INDIRECT) != 0
    }
}

impl Default for VirtqDesc {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// VIRTQUEUE AVAILABLE RING
// =============================================================================

/// Virtqueue available ring flags
pub mod avail_flags {
    /// Don't interrupt when the device consumes a buffer
    pub const NO_INTERRUPT: u16 = 1;
}

/// Virtqueue available ring header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqAvail {
    /// Available ring flags
    pub flags: u16,
    /// Index of next available entry
    pub idx: u16,
    // Followed by ring[queue_size] and used_event
}

impl VirtqAvail {
    /// Create new available ring header
    pub const fn new() -> Self {
        Self { flags: 0, idx: 0 }
    }

    /// Disable interrupts
    pub fn disable_interrupts(&mut self) {
        self.flags = avail_flags::NO_INTERRUPT;
    }

    /// Enable interrupts
    pub fn enable_interrupts(&mut self) {
        self.flags = 0;
    }
}

impl Default for VirtqAvail {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// VIRTQUEUE USED RING
// =============================================================================

/// Virtqueue used ring flags
pub mod used_flags {
    /// Don't notify when the driver adds a buffer
    pub const NO_NOTIFY: u16 = 1;
}

/// Virtqueue used element
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqUsedElem {
    /// Index of the descriptor chain
    pub id: u32,
    /// Length written to the descriptor chain
    pub len: u32,
}

impl VirtqUsedElem {
    /// Create new used element
    pub const fn new() -> Self {
        Self { id: 0, len: 0 }
    }
}

impl Default for VirtqUsedElem {
    fn default() -> Self {
        Self::new()
    }
}

/// Virtqueue used ring header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqUsed {
    /// Used ring flags
    pub flags: u16,
    /// Index of next used entry
    pub idx: u16,
    // Followed by ring[queue_size] and avail_event
}

impl VirtqUsed {
    /// Create new used ring header
    pub const fn new() -> Self {
        Self { flags: 0, idx: 0 }
    }

    /// Check if notifications are suppressed
    pub const fn no_notify(&self) -> bool {
        (self.flags & used_flags::NO_NOTIFY) != 0
    }
}

impl Default for VirtqUsed {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// VIRTQUEUE STATE
// =============================================================================

/// Virtqueue state
#[derive(Debug, Clone, Copy)]
pub struct VirtqueueState {
    /// Queue size (power of 2)
    pub size: u16,
    /// Index of next free descriptor
    pub free_head: u16,
    /// Number of free descriptors
    pub num_free: u16,
    /// Last seen used index
    pub last_used_idx: u16,
    /// Queue index
    pub index: u16,
    /// Is queue ready
    pub ready: bool,
    /// Notification suppressed
    pub notification_suppressed: bool,
}

impl VirtqueueState {
    /// Create new virtqueue state
    pub const fn new(index: u16, size: u16) -> Self {
        Self {
            size,
            free_head: 0,
            num_free: size,
            last_used_idx: 0,
            index,
            ready: false,
            notification_suppressed: false,
        }
    }

    /// Check if queue is full
    pub const fn is_full(&self) -> bool {
        self.num_free == 0
    }

    /// Check if queue is empty
    pub const fn is_empty(&self) -> bool {
        self.num_free == self.size
    }
}

// =============================================================================
// VIRTIO MMIO REGISTERS
// =============================================================================

/// VirtIO MMIO register offsets
pub mod mmio {
    /// Magic value (0x74726976, "virt")
    pub const MAGIC_VALUE: usize = 0x000;
    /// Version (1 for legacy, 2 for modern)
    pub const VERSION: usize = 0x004;
    /// Device type
    pub const DEVICE_ID: usize = 0x008;
    /// Vendor ID
    pub const VENDOR_ID: usize = 0x00C;
    /// Device features
    pub const DEVICE_FEATURES: usize = 0x010;
    /// Device features select
    pub const DEVICE_FEATURES_SEL: usize = 0x014;
    /// Driver features
    pub const DRIVER_FEATURES: usize = 0x020;
    /// Driver features select
    pub const DRIVER_FEATURES_SEL: usize = 0x024;
    /// Queue select
    pub const QUEUE_SEL: usize = 0x030;
    /// Maximum queue size
    pub const QUEUE_NUM_MAX: usize = 0x034;
    /// Queue size
    pub const QUEUE_NUM: usize = 0x038;
    /// Queue ready
    pub const QUEUE_READY: usize = 0x044;
    /// Queue notify
    pub const QUEUE_NOTIFY: usize = 0x050;
    /// Interrupt status
    pub const INTERRUPT_STATUS: usize = 0x060;
    /// Interrupt acknowledge
    pub const INTERRUPT_ACK: usize = 0x064;
    /// Device status
    pub const STATUS: usize = 0x070;
    /// Queue descriptor low
    pub const QUEUE_DESC_LOW: usize = 0x080;
    /// Queue descriptor high
    pub const QUEUE_DESC_HIGH: usize = 0x084;
    /// Queue driver low
    pub const QUEUE_DRIVER_LOW: usize = 0x090;
    /// Queue driver high
    pub const QUEUE_DRIVER_HIGH: usize = 0x094;
    /// Queue device low
    pub const QUEUE_DEVICE_LOW: usize = 0x0A0;
    /// Queue device high
    pub const QUEUE_DEVICE_HIGH: usize = 0x0A4;
    /// Shared memory select
    pub const SHM_SEL: usize = 0x0AC;
    /// Shared memory length low
    pub const SHM_LEN_LOW: usize = 0x0B0;
    /// Shared memory length high
    pub const SHM_LEN_HIGH: usize = 0x0B4;
    /// Shared memory base low
    pub const SHM_BASE_LOW: usize = 0x0B8;
    /// Shared memory base high
    pub const SHM_BASE_HIGH: usize = 0x0BC;
    /// Queue reset
    pub const QUEUE_RESET: usize = 0x0C0;
    /// Configuration space start
    pub const CONFIG: usize = 0x100;
}

/// VirtIO MMIO interrupt status bits
pub mod mmio_int {
    /// Used buffer notification
    pub const USED_RING_UPDATE: u32 = 1 << 0;
    /// Configuration change notification
    pub const CONFIG_CHANGE: u32 = 1 << 1;
}

// =============================================================================
// VIRTIO PCI CAPABILITY
// =============================================================================

/// VirtIO PCI capability types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VirtioPciCapType {
    /// Common configuration
    CommonCfg       = 1,
    /// Notifications
    NotifyCfg       = 2,
    /// ISR status
    IsrCfg          = 3,
    /// Device-specific configuration
    DeviceCfg       = 4,
    /// PCI configuration access
    PciCfg          = 5,
    /// Shared memory region
    SharedMemoryCfg = 8,
    /// Vendor-specific data
    VendorCfg       = 9,
}

/// VirtIO PCI capability structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VirtioPciCap {
    /// Capability ID (vendor-specific = 0x09)
    pub cap_vndr: u8,
    /// Next capability offset
    pub cap_next: u8,
    /// Capability length
    pub cap_len: u8,
    /// Configuration type
    pub cfg_type: u8,
    /// BAR number
    pub bar: u8,
    /// ID (for shared memory)
    pub id: u8,
    /// Padding
    pub padding: [u8; 2],
    /// Offset within BAR
    pub offset: u32,
    /// Length of the structure
    pub length: u32,
}

/// VirtIO PCI notification capability
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VirtioPciNotifyCap {
    /// Base capability
    pub cap: VirtioPciCap,
    /// Notification offset multiplier
    pub notify_off_multiplier: u32,
}

// =============================================================================
// VIRTIO PCI COMMON CONFIGURATION
// =============================================================================

/// VirtIO PCI common configuration structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioPciCommonCfg {
    /// Device feature bits 0-31
    pub device_feature_select: u32,
    /// Device features
    pub device_feature: u32,
    /// Driver feature bits 0-31
    pub driver_feature_select: u32,
    /// Driver features
    pub driver_feature: u32,
    /// MSI-X configuration vector
    pub config_msix_vector: u16,
    /// Number of virtqueues
    pub num_queues: u16,
    /// Device status
    pub device_status: u8,
    /// Configuration generation
    pub config_generation: u8,
    /// Queue select
    pub queue_select: u16,
    /// Queue size
    pub queue_size: u16,
    /// Queue MSI-X vector
    pub queue_msix_vector: u16,
    /// Queue enable
    pub queue_enable: u16,
    /// Queue notify offset
    pub queue_notify_off: u16,
    /// Queue descriptor low
    pub queue_desc_lo: u32,
    /// Queue descriptor high
    pub queue_desc_hi: u32,
    /// Queue driver (available) low
    pub queue_driver_lo: u32,
    /// Queue driver (available) high
    pub queue_driver_hi: u32,
    /// Queue device (used) low
    pub queue_device_lo: u32,
    /// Queue device (used) high
    pub queue_device_hi: u32,
    /// Queue notify data
    pub queue_notify_data: u16,
    /// Queue reset
    pub queue_reset: u16,
}

// =============================================================================
// VIRTIO BLOCK DEVICE
// =============================================================================

/// VirtIO block device configuration
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioBlkConfig {
    /// Capacity in 512-byte sectors
    pub capacity: u64,
    /// Maximum segment size
    pub size_max: u32,
    /// Maximum number of segments
    pub seg_max: u32,
    /// Geometry: cylinders
    pub geometry_cylinders: u16,
    /// Geometry: heads
    pub geometry_heads: u8,
    /// Geometry: sectors
    pub geometry_sectors: u8,
    /// Block size
    pub blk_size: u32,
    /// Topology: physical block exp
    pub physical_block_exp: u8,
    /// Topology: alignment offset
    pub alignment_offset: u8,
    /// Topology: minimum I/O size
    pub min_io_size: u16,
    /// Topology: optimal I/O size
    pub opt_io_size: u32,
    /// Writeback mode
    pub wce: u8,
    /// Unused
    pub unused: u8,
    /// Number of queues
    pub num_queues: u16,
    /// Max discard sectors
    pub max_discard_sectors: u32,
    /// Max discard segment
    pub max_discard_seg: u32,
    /// Discard sector alignment
    pub discard_sector_alignment: u32,
    /// Max write zeroes sectors
    pub max_write_zeroes_sectors: u32,
    /// Max write zeroes segments
    pub max_write_zeroes_seg: u32,
    /// Write zeroes may unmap
    pub write_zeroes_may_unmap: u8,
    /// Unused
    pub unused1: [u8; 3],
    /// Max secure erase sectors
    pub max_secure_erase_sectors: u32,
    /// Max secure erase segments
    pub max_secure_erase_seg: u32,
    /// Secure erase sector alignment
    pub secure_erase_sector_alignment: u32,
}

impl VirtioBlkConfig {
    /// Get capacity in bytes
    pub const fn capacity_bytes(&self) -> u64 {
        self.capacity * 512
    }

    /// Get capacity in megabytes
    pub const fn capacity_mb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024)
    }

    /// Get capacity in gigabytes
    pub const fn capacity_gb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024 * 1024)
    }
}

/// VirtIO block request types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VirtioBlkReqType {
    /// Read
    In          = 0,
    /// Write
    Out         = 1,
    /// Flush
    Flush       = 4,
    /// Get ID
    GetId       = 8,
    /// Get lifetime
    GetLifetime = 10,
    /// Discard
    Discard     = 11,
    /// Write zeroes
    WriteZeroes = 13,
    /// Secure erase
    SecureErase = 14,
}

/// VirtIO block request header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioBlkReqHeader {
    /// Request type
    pub req_type: u32,
    /// Reserved
    pub reserved: u32,
    /// Sector (512-byte offset)
    pub sector: u64,
}

impl VirtioBlkReqHeader {
    /// Create read request
    pub const fn read(sector: u64) -> Self {
        Self {
            req_type: VirtioBlkReqType::In as u32,
            reserved: 0,
            sector,
        }
    }

    /// Create write request
    pub const fn write(sector: u64) -> Self {
        Self {
            req_type: VirtioBlkReqType::Out as u32,
            reserved: 0,
            sector,
        }
    }

    /// Create flush request
    pub const fn flush() -> Self {
        Self {
            req_type: VirtioBlkReqType::Flush as u32,
            reserved: 0,
            sector: 0,
        }
    }
}

/// VirtIO block status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VirtioBlkStatus {
    /// Success
    Ok     = 0,
    /// I/O error
    IoErr  = 1,
    /// Unsupported operation
    Unsupp = 2,
}

// =============================================================================
// VIRTIO NETWORK DEVICE
// =============================================================================

/// VirtIO network configuration
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioNetConfig {
    /// MAC address
    pub mac: [u8; 6],
    /// Status
    pub status: u16,
    /// Maximum virtqueue pairs
    pub max_virtqueue_pairs: u16,
    /// MTU
    pub mtu: u16,
    /// Speed (in Mbps)
    pub speed: u32,
    /// Duplex (0 = half, 1 = full)
    pub duplex: u8,
    /// RSS max key size
    pub rss_max_key_size: u8,
    /// RSS max indirection table length
    pub rss_max_indirection_table_length: u16,
    /// Supported hash types
    pub supported_hash_types: u32,
}

impl VirtioNetConfig {
    /// Get MAC address as string
    pub fn mac_string(&self) -> [u8; 17] {
        let mut buf = [0u8; 17];
        let hex = b"0123456789ABCDEF";

        for (i, &byte) in self.mac.iter().enumerate() {
            buf[i * 3] = hex[(byte >> 4) as usize];
            buf[i * 3 + 1] = hex[(byte & 0xF) as usize];
            if i < 5 {
                buf[i * 3 + 2] = b':';
            }
        }

        buf
    }

    /// Check if link is up
    pub const fn is_link_up(&self) -> bool {
        (self.status & 1) != 0
    }

    /// Check if link was announced
    pub const fn is_announced(&self) -> bool {
        (self.status & 2) != 0
    }
}

/// VirtIO network header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioNetHeader {
    /// Flags
    pub flags: u8,
    /// GSO type
    pub gso_type: u8,
    /// Header length
    pub hdr_len: u16,
    /// GSO size
    pub gso_size: u16,
    /// Checksum start
    pub csum_start: u16,
    /// Checksum offset
    pub csum_offset: u16,
    /// Number of buffers
    pub num_buffers: u16,
}

impl VirtioNetHeader {
    /// Create empty header
    pub const fn new() -> Self {
        Self {
            flags: 0,
            gso_type: 0,
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        }
    }
}

impl Default for VirtioNetHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// VirtIO network GSO types
pub mod net_gso {
    /// No GSO
    pub const NONE: u8 = 0;
    /// TCP v4 GSO
    pub const TCPV4: u8 = 1;
    /// UDP GSO
    pub const UDP: u8 = 3;
    /// TCP v6 GSO
    pub const TCPV6: u8 = 4;
    /// UDP L4 GSO
    pub const UDP_L4: u8 = 5;
    /// ECN flag
    pub const ECN: u8 = 0x80;
}

/// VirtIO network header flags
pub mod net_flags {
    /// Needs checksum
    pub const NEEDS_CSUM: u8 = 1;
    /// Data valid
    pub const DATA_VALID: u8 = 2;
    /// RSC info
    pub const RSC_INFO: u8 = 4;
}

// =============================================================================
// VIRTIO CONSOLE DEVICE
// =============================================================================

/// VirtIO console configuration
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioConsoleConfig {
    /// Number of columns
    pub cols: u16,
    /// Number of rows
    pub rows: u16,
    /// Maximum number of ports
    pub max_nr_ports: u32,
    /// Emergency write
    pub emerg_wr: u32,
}

/// VirtIO console control message
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioConsoleControl {
    /// Port ID
    pub id: u32,
    /// Event type
    pub event: u16,
    /// Event value
    pub value: u16,
}

/// VirtIO console control events
pub mod console_event {
    /// Device ready
    pub const DEVICE_READY: u16 = 0;
    /// Device add
    pub const DEVICE_ADD: u16 = 1;
    /// Device remove
    pub const DEVICE_REMOVE: u16 = 2;
    /// Port ready
    pub const PORT_READY: u16 = 3;
    /// Console port
    pub const CONSOLE_PORT: u16 = 4;
    /// Resize
    pub const RESIZE: u16 = 5;
    /// Port open
    pub const PORT_OPEN: u16 = 6;
    /// Port name
    pub const PORT_NAME: u16 = 7;
}

// =============================================================================
// VIRTIO GPU DEVICE
// =============================================================================

/// VirtIO GPU configuration
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuConfig {
    /// Number of events read
    pub events_read: u32,
    /// Events clear
    pub events_clear: u32,
    /// Number of scanouts
    pub num_scanouts: u32,
    /// Reserved
    pub reserved: u32,
}

/// VirtIO GPU command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VirtioGpuCmd {
    /// Get display info
    GetDisplayInfo     = 0x0100,
    /// Resource create 2D
    ResourceCreate2d   = 0x0101,
    /// Resource unref
    ResourceUnref      = 0x0102,
    /// Set scanout
    SetScanout         = 0x0103,
    /// Resource flush
    ResourceFlush      = 0x0104,
    /// Transfer to host 2D
    TransferToHost2d   = 0x0105,
    /// Resource attach backing
    ResourceAttachBacking = 0x0106,
    /// Resource detach backing
    ResourceDetachBacking = 0x0107,
    /// Get capset info
    GetCapsetInfo      = 0x0108,
    /// Get capset
    GetCapset          = 0x0109,
    /// Get EDID
    GetEdid            = 0x010A,
    /// Resource assign UUID
    ResourceAssignUuid = 0x010B,
    /// Resource create blob
    ResourceCreateBlob = 0x010C,
    /// Set scanout blob
    SetScanoutBlob     = 0x010D,

    /// Update cursor
    UpdateCursor       = 0x0300,
    /// Move cursor
    MoveCursor         = 0x0301,

    /// Response OK (no data)
    RespOkNodata       = 0x1100,
    /// Response OK (display info)
    RespOkDisplayInfo  = 0x1101,
    /// Response OK (capset info)
    RespOkCapsetInfo   = 0x1102,
    /// Response OK (capset)
    RespOkCapset       = 0x1103,
    /// Response OK (EDID)
    RespOkEdid         = 0x1104,
    /// Response OK (resource UUID)
    RespOkResourceUuid = 0x1105,
    /// Response OK (map info)
    RespOkMapInfo      = 0x1106,

    /// Response error (unspecified)
    RespErrUnspec      = 0x1200,
    /// Response error (out of memory)
    RespErrOutOfMemory = 0x1201,
    /// Response error (invalid scanout ID)
    RespErrInvalidScanoutId = 0x1202,
    /// Response error (invalid resource ID)
    RespErrInvalidResourceId = 0x1203,
    /// Response error (invalid context ID)
    RespErrInvalidContextId = 0x1204,
    /// Response error (invalid parameter)
    RespErrInvalidParameter = 0x1205,
}

/// VirtIO GPU control header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuCtrlHdr {
    /// Command type
    pub cmd_type: u32,
    /// Flags
    pub flags: u32,
    /// Fence ID
    pub fence_id: u64,
    /// Context ID
    pub ctx_id: u32,
    /// Ring index
    pub ring_idx: u8,
    /// Padding
    pub padding: [u8; 3],
}

impl VirtioGpuCtrlHdr {
    /// Create new control header
    pub const fn new(cmd_type: VirtioGpuCmd) -> Self {
        Self {
            cmd_type: cmd_type as u32,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        }
    }
}

/// VirtIO GPU rectangle
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuRect {
    /// X position
    pub x: u32,
    /// Y position
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// VirtIO GPU display info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuDisplayOne {
    /// Rectangle
    pub rect: VirtioGpuRect,
    /// Enabled
    pub enabled: u32,
    /// Flags
    pub flags: u32,
}

/// VirtIO GPU pixel formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VirtioGpuFormats {
    /// B8G8R8A8 unorm
    B8G8R8A8Unorm = 1,
    /// B8G8R8X8 unorm
    B8G8R8X8Unorm = 2,
    /// A8R8G8B8 unorm
    A8R8G8B8Unorm = 3,
    /// X8R8G8B8 unorm
    X8R8G8B8Unorm = 4,
    /// R8G8B8A8 unorm
    R8G8B8A8Unorm = 67,
    /// X8B8G8R8 unorm
    X8B8G8R8Unorm = 68,
    /// A8B8G8R8 unorm
    A8B8G8R8Unorm = 121,
    /// R8G8B8X8 unorm
    R8G8B8X8Unorm = 134,
}

// =============================================================================
// VIRTIO INPUT DEVICE
// =============================================================================

/// VirtIO input configuration select values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VirtioInputConfigSelect {
    /// Unset
    Unset    = 0x00,
    /// ID name
    IdName   = 0x01,
    /// ID serial
    IdSerial = 0x02,
    /// ID device IDs
    IdDevids = 0x03,
    /// Properties bits
    PropBits = 0x10,
    /// Event bits
    EvBits   = 0x11,
    /// Abs info
    AbsInfo  = 0x12,
}

/// VirtIO input device IDs
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioInputDevids {
    /// Bus type
    pub bustype: u16,
    /// Vendor ID
    pub vendor: u16,
    /// Product ID
    pub product: u16,
    /// Version
    pub version: u16,
}

/// VirtIO input absolute info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioInputAbsinfo {
    /// Minimum value
    pub min: u32,
    /// Maximum value
    pub max: u32,
    /// Fuzz
    pub fuzz: u32,
    /// Flat
    pub flat: u32,
    /// Resolution
    pub res: u32,
}

/// VirtIO input event
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioInputEvent {
    /// Event type
    pub event_type: u16,
    /// Event code
    pub code: u16,
    /// Event value
    pub value: u32,
}

// =============================================================================
// VIRTIO ERROR TYPES
// =============================================================================

/// VirtIO error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioError {
    /// Device not found
    DeviceNotFound,
    /// Invalid magic value
    InvalidMagic,
    /// Unsupported version
    UnsupportedVersion,
    /// Feature negotiation failed
    FeatureNegotiationFailed,
    /// Queue not available
    QueueNotAvailable,
    /// Queue already active
    QueueAlreadyActive,
    /// Queue full
    QueueFull,
    /// Invalid descriptor
    InvalidDescriptor,
    /// I/O error
    IoError,
    /// Timeout
    Timeout,
    /// Device error
    DeviceError,
    /// Invalid parameter
    InvalidParameter,
    /// Out of memory
    OutOfMemory,
    /// Not supported
    NotSupported,
}

impl fmt::Display for VirtioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VirtioError::DeviceNotFound => write!(f, "Device not found"),
            VirtioError::InvalidMagic => write!(f, "Invalid magic value"),
            VirtioError::UnsupportedVersion => write!(f, "Unsupported version"),
            VirtioError::FeatureNegotiationFailed => write!(f, "Feature negotiation failed"),
            VirtioError::QueueNotAvailable => write!(f, "Queue not available"),
            VirtioError::QueueAlreadyActive => write!(f, "Queue already active"),
            VirtioError::QueueFull => write!(f, "Queue full"),
            VirtioError::InvalidDescriptor => write!(f, "Invalid descriptor"),
            VirtioError::IoError => write!(f, "I/O error"),
            VirtioError::Timeout => write!(f, "Timeout"),
            VirtioError::DeviceError => write!(f, "Device error"),
            VirtioError::InvalidParameter => write!(f, "Invalid parameter"),
            VirtioError::OutOfMemory => write!(f, "Out of memory"),
            VirtioError::NotSupported => write!(f, "Not supported"),
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Calculate virtqueue size requirements
pub const fn virtqueue_size(queue_size: u16, align: usize) -> usize {
    let queue_size = queue_size as usize;

    // Descriptor table size
    let desc_size = queue_size * core::mem::size_of::<VirtqDesc>();

    // Available ring size (header + ring entries + used_event)
    let avail_size = 4 + queue_size * 2 + 2;

    // Used ring size (header + ring entries + avail_event)
    let used_size = 4 + queue_size * core::mem::size_of::<VirtqUsedElem>() + 2;

    // Align sizes
    let aligned_avail = (desc_size + avail_size + align - 1) & !(align - 1);
    aligned_avail + used_size
}

/// Check if a number is a power of 2
pub const fn is_power_of_2(n: u16) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Round up to next power of 2
pub const fn next_power_of_2(mut n: u16) -> u16 {
    if n == 0 {
        return 1;
    }
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n + 1
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_from_pci() {
        assert_eq!(
            VirtioDeviceType::from_pci_device_id(0x1001),
            Some(VirtioDeviceType::Network)
        );
        assert_eq!(
            VirtioDeviceType::from_pci_device_id(0x1002),
            Some(VirtioDeviceType::Block)
        );
        assert_eq!(
            VirtioDeviceType::from_pci_device_id(0x1041),
            Some(VirtioDeviceType::Network)
        );
        assert_eq!(
            VirtioDeviceType::from_pci_device_id(0x1042),
            Some(VirtioDeviceType::Block)
        );
    }

    #[test]
    fn test_device_status() {
        let mut status = DeviceStatus::reset();
        assert_eq!(status.0, 0);

        status.acknowledge();
        assert!(status.is_acknowledged());

        status.set_driver();
        assert!(status.is_driver());

        status.set_features_ok();
        assert!(status.is_features_ok());

        status.set_driver_ok();
        assert!(status.is_driver_ok());
    }

    #[test]
    fn test_virtq_desc() {
        let read = VirtqDesc::read(0x1000, 512);
        assert!(!read.is_write());
        assert!(!read.has_next());

        let write = VirtqDesc::write(0x2000, 1024);
        assert!(write.is_write());
        assert!(!write.has_next());

        let chained = VirtqDesc::read_next(0x3000, 256, 5);
        assert!(chained.has_next());
        assert_eq!(chained.next, 5);
    }

    #[test]
    fn test_virtqueue_size() {
        let size = virtqueue_size(256, 4096);
        assert!(size > 0);
        assert!(size < 1024 * 1024);
    }

    #[test]
    fn test_power_of_2() {
        assert!(is_power_of_2(1));
        assert!(is_power_of_2(2));
        assert!(is_power_of_2(256));
        assert!(!is_power_of_2(0));
        assert!(!is_power_of_2(3));
        assert!(!is_power_of_2(100));
    }

    #[test]
    fn test_next_power_of_2() {
        assert_eq!(next_power_of_2(0), 1);
        assert_eq!(next_power_of_2(1), 1);
        assert_eq!(next_power_of_2(3), 4);
        assert_eq!(next_power_of_2(100), 128);
        assert_eq!(next_power_of_2(256), 256);
    }

    #[test]
    fn test_block_config() {
        let config = VirtioBlkConfig {
            capacity: 2097152, // 1GB
            size_max: 0,
            seg_max: 0,
            geometry_cylinders: 0,
            geometry_heads: 0,
            geometry_sectors: 0,
            blk_size: 512,
            physical_block_exp: 0,
            alignment_offset: 0,
            min_io_size: 0,
            opt_io_size: 0,
            wce: 0,
            unused: 0,
            num_queues: 1,
            max_discard_sectors: 0,
            max_discard_seg: 0,
            discard_sector_alignment: 0,
            max_write_zeroes_sectors: 0,
            max_write_zeroes_seg: 0,
            write_zeroes_may_unmap: 0,
            unused1: [0; 3],
            max_secure_erase_sectors: 0,
            max_secure_erase_seg: 0,
            secure_erase_sector_alignment: 0,
        };

        assert_eq!(config.capacity_bytes(), 1073741824);
        assert_eq!(config.capacity_mb(), 1024);
        assert_eq!(config.capacity_gb(), 1);
    }

    #[test]
    fn test_net_mac_string() {
        let config = VirtioNetConfig {
            mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            status: 1,
            max_virtqueue_pairs: 1,
            mtu: 1500,
            speed: 1000,
            duplex: 1,
            rss_max_key_size: 0,
            rss_max_indirection_table_length: 0,
            supported_hash_types: 0,
        };

        let mac = config.mac_string();
        assert_eq!(&mac, b"52:54:00:12:34:56");
        assert!(config.is_link_up());
    }
}
