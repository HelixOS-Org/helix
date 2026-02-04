//! High-Level Protocol Abstractions
//!
//! This module provides safe, ergonomic wrappers around UEFI protocols.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Protocol Abstractions                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
//! │  │   Console   │ │  Graphics   │ │ FileSystem  │ │   Block   │ │
//! │  │   Protocol  │ │   Output    │ │  Protocol   │ │    I/O    │ │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘ │
//! │                                                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
//! │  │   Serial    │ │    PCI      │ │   Network   │ │   USB     │ │
//! │  │    Port     │ │   Access    │ │   Stack     │ │   Stack   │ │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘ │
//! │                                                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
//! │  │   ACPI      │ │   SMBIOS    │ │  Security   │ │  Random   │ │
//! │  │   Tables    │ │   Tables    │ │  Protocols  │ │  Number   │ │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘ │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Console**: Text input/output with full keyboard support
//! - **Graphics**: Framebuffer access with pixel drawing
//! - **FileSystem**: File operations with path abstraction
//! - **Block**: Raw disk access with partition support
//! - **Serial**: Debug output and communication
//! - **PCI**: Device enumeration and configuration
//! - **Network**: TCP/IP stack abstraction
//! - **USB**: USB device and host support
//! - **ACPI**: ACPI table parsing
//! - **SMBIOS**: System information
//! - **Security**: Secure Boot and authentication
//! - **RNG**: Cryptographic random numbers

pub mod acpi;
pub mod block;
pub mod console;
pub mod filesystem;
pub mod graphics;
pub mod network;
pub mod pci;
pub mod rng;
pub mod security;
pub mod serial;
pub mod smbios;
pub mod usb;

// Re-exports
pub use acpi::AcpiTables;
pub use block::{BlockDevice, DiskInfo, Partition};
pub use console::{Console, InputKey, KeyModifiers, ScanCode};
pub use filesystem::{Directory, File, FileAttributes, FileInfo, FileMode, FileSystem};
pub use graphics::{Framebuffer, GraphicsOutput, Pixel, PixelFormat, Resolution};
pub use network::{IpAddress, MacAddress, NetworkInterface};
pub use pci::{PciClass, PciConfig, PciDevice, PciLocation};
pub use rng::EntropySource;
pub use security::SecureBoot;
pub use serial::{Parity, SerialConfig, SerialPort, StopBits};
pub use smbios::SmbiosTables;
pub use usb::UsbDevice;

use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// PROTOCOL TRAIT
// =============================================================================

/// Trait for UEFI protocols
pub trait Protocol: Sized {
    /// Protocol GUID
    const GUID: Guid;

    /// Open protocol on handle
    fn open(handle: Handle) -> Result<Self>;

    /// Close protocol (optional)
    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Trait for enumerable protocols
pub trait EnumerableProtocol: Protocol {
    /// Enumerate all instances of this protocol
    fn enumerate() -> Result<alloc::vec::Vec<Self>>;

    /// Get first instance
    fn first() -> Result<Self> {
        Self::enumerate()?.into_iter().next().ok_or(Error::NotFound)
    }
}

// =============================================================================
// PROTOCOL HANDLE
// =============================================================================

/// RAII wrapper for protocol instances
pub struct ProtocolHandle<P: Protocol> {
    /// Protocol instance
    protocol: P,
    /// Handle
    handle: Handle,
    /// Auto-close on drop
    auto_close: bool,
}

impl<P: Protocol> ProtocolHandle<P> {
    /// Create new protocol handle
    pub fn new(protocol: P, handle: Handle) -> Self {
        Self {
            protocol,
            handle,
            auto_close: true,
        }
    }

    /// Disable auto-close
    pub fn leak(mut self) -> P {
        self.auto_close = false;
        // Can't move out directly, need unsafe
        unsafe {
            let protocol = core::ptr::read(&self.protocol);
            core::mem::forget(self);
            protocol
        }
    }

    /// Get underlying handle
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// Get protocol reference
    pub fn protocol(&self) -> &P {
        &self.protocol
    }

    /// Get mutable protocol reference
    pub fn protocol_mut(&mut self) -> &mut P {
        &mut self.protocol
    }
}

impl<P: Protocol> core::ops::Deref for ProtocolHandle<P> {
    type Target = P;

    fn deref(&self) -> &P {
        &self.protocol
    }
}

impl<P: Protocol> core::ops::DerefMut for ProtocolHandle<P> {
    fn deref_mut(&mut self) -> &mut P {
        &mut self.protocol
    }
}

impl<P: Protocol> Drop for ProtocolHandle<P> {
    fn drop(&mut self) {
        if self.auto_close {
            let _ = self.protocol.close();
        }
    }
}

// =============================================================================
// PROTOCOL LOCATOR
// =============================================================================

/// Protocol locator utilities
pub struct ProtocolLocator;

impl ProtocolLocator {
    /// Locate protocol by GUID
    pub fn locate<P: Protocol>() -> Result<ProtocolHandle<P>> {
        let handle = Self::locate_handle(&P::GUID)?;
        let protocol = P::open(handle)?;
        Ok(ProtocolHandle::new(protocol, handle))
    }

    /// Locate all instances of a protocol
    pub fn locate_all<P: Protocol>() -> Result<alloc::vec::Vec<ProtocolHandle<P>>> {
        let handles = Self::locate_handles(&P::GUID)?;
        let mut protocols = alloc::vec::Vec::with_capacity(handles.len());

        for handle in handles {
            if let Ok(protocol) = P::open(handle) {
                protocols.push(ProtocolHandle::new(protocol, handle));
            }
        }

        Ok(protocols)
    }

    /// Locate single handle for protocol GUID
    fn locate_handle(guid: &Guid) -> Result<Handle> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };

        // LocateProtocol
        let mut interface: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).locate_protocol)(guid as *const Guid, core::ptr::null_mut(), &mut interface)
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // We need a handle, not just interface. Use HandleBuffer approach.
        let handles = Self::locate_handles(guid)?;
        handles.into_iter().next().ok_or(Error::NotFound)
    }

    /// Locate all handles for protocol GUID
    fn locate_handles(guid: &Guid) -> Result<alloc::vec::Vec<Handle>> {
        use crate::raw::types::LocateSearchType;
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };

        let mut buffer_size: usize = 0;
        let mut buffer: *mut Handle = core::ptr::null_mut();

        // First call to get size
        let result = unsafe {
            ((*bs).locate_handle)(
                LocateSearchType::ByProtocol,
                guid as *const Guid,
                core::ptr::null_mut(),
                &mut buffer_size,
                buffer,
            )
        };

        if result != Status::BUFFER_TOO_SMALL && result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        if buffer_size == 0 {
            return Ok(alloc::vec::Vec::new());
        }

        // Allocate buffer
        let handle_count = buffer_size / core::mem::size_of::<Handle>();
        let mut handles = alloc::vec![Handle(core::ptr::null_mut()); handle_count];
        buffer = handles.as_mut_ptr();

        // Second call to get handles
        let result = unsafe {
            ((*bs).locate_handle)(
                LocateSearchType::ByProtocol,
                guid as *const Guid,
                core::ptr::null_mut(),
                &mut buffer_size,
                buffer,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Remove null handles
        handles.retain(|h| !h.0.is_null());

        Ok(handles)
    }
}

// =============================================================================
// DEVICE PATH UTILITIES
// =============================================================================

/// Device path utilities
pub struct DevicePath {
    /// Raw device path data
    data: alloc::vec::Vec<u8>,
}

impl DevicePath {
    /// Create from raw pointer
    ///
    /// # Safety
    /// Pointer must be valid device path
    pub unsafe fn from_raw(
        ptr: *const crate::raw::protocols::loaded_image::EfiDevicePathProtocol,
    ) -> Self {
        if ptr.is_null() {
            return Self {
                data: alloc::vec::Vec::new(),
            };
        }

        // Calculate total length
        let mut total_len = 0usize;
        let mut current = ptr;

        loop {
            let node = &*current;
            let len = u16::from_le_bytes(node.length) as usize;
            total_len += len;

            // Check for end node
            if node.device_type == 0x7F && node.sub_type == 0xFF {
                break;
            }

            current = (current as *const u8).add(len)
                as *const crate::raw::protocols::loaded_image::EfiDevicePathProtocol;
        }

        // Copy data
        let mut data = alloc::vec![0u8; total_len];
        core::ptr::copy_nonoverlapping(ptr as *const u8, data.as_mut_ptr(), total_len);

        Self { data }
    }

    /// Get as raw pointer
    pub fn as_ptr(&self) -> *const crate::raw::protocols::loaded_image::EfiDevicePathProtocol {
        self.data.as_ptr() as *const _
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get length in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Convert to text representation
    pub fn to_text(&self) -> alloc::string::String {
        // TODO: Implement device path to text conversion
        alloc::format!("DevicePath({} bytes)", self.data.len())
    }

    /// Iterate over nodes
    pub fn nodes(&self) -> DevicePathNodeIter<'_> {
        DevicePathNodeIter {
            data: &self.data,
            offset: 0,
        }
    }
}

/// Device path node
#[derive(Debug, Clone)]
pub struct DevicePathNode {
    /// Node type
    pub node_type: u8,
    /// Sub-type
    pub sub_type: u8,
    /// Node data
    pub data: alloc::vec::Vec<u8>,
}

impl DevicePathNode {
    /// Type: Hardware
    pub const TYPE_HARDWARE: u8 = 0x01;
    /// Type: ACPI
    pub const TYPE_ACPI: u8 = 0x02;
    /// Type: Messaging
    pub const TYPE_MESSAGING: u8 = 0x03;
    /// Type: Media
    pub const TYPE_MEDIA: u8 = 0x04;
    /// Type: BIOS Boot
    pub const TYPE_BIOS_BOOT: u8 = 0x05;
    /// Type: End
    pub const TYPE_END: u8 = 0x7F;

    /// Check if this is end node
    pub fn is_end(&self) -> bool {
        self.node_type == Self::TYPE_END
    }

    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self.node_type {
            Self::TYPE_HARDWARE => "Hardware",
            Self::TYPE_ACPI => "ACPI",
            Self::TYPE_MESSAGING => "Messaging",
            Self::TYPE_MEDIA => "Media",
            Self::TYPE_BIOS_BOOT => "BIOS Boot",
            Self::TYPE_END => "End",
            _ => "Unknown",
        }
    }
}

/// Iterator over device path nodes
pub struct DevicePathNodeIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for DevicePathNodeIter<'a> {
    type Item = DevicePathNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        // Read header
        let node_type = self.data[self.offset];
        let sub_type = self.data[self.offset + 1];
        let length =
            u16::from_le_bytes([self.data[self.offset + 2], self.data[self.offset + 3]]) as usize;

        if length < 4 || self.offset + length > self.data.len() {
            return None;
        }

        // Copy data (excluding header)
        let data = self.data[self.offset + 4..self.offset + length].to_vec();

        self.offset += length;

        Some(DevicePathNode {
            node_type,
            sub_type,
            data,
        })
    }
}

// =============================================================================
// PROTOCOL NOTIFICATION
// =============================================================================

/// Protocol registration notification
pub struct ProtocolNotification {
    /// Event handle
    event: Event,
    /// Registration key
    registration: *mut core::ffi::c_void,
}

impl ProtocolNotification {
    /// Register for protocol notification
    pub fn register<P: Protocol>() -> Result<Self> {
        use crate::event::EventType;
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };

        let mut event = Event(core::ptr::null_mut());
        let mut registration = core::ptr::null_mut();

        // Create notify event
        extern "efiapi" fn notify_stub(_event: Event, _context: *mut core::ffi::c_void) {}

        let result = unsafe {
            ((*bs).create_event)(
                EventType::NOTIFY_SIGNAL.as_raw(),
                crate::services::tpl::Tpl::CALLBACK.raw(),
                Some(notify_stub),
                core::ptr::null_mut(),
                &mut event,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Register for notification
        let result = unsafe {
            ((*bs).register_protocol_notify)(&P::GUID as *const Guid, event, &mut registration)
        };

        if result != Status::SUCCESS {
            // Clean up event
            unsafe { ((*bs).close_event)(event) };
            return Err(Error::from_status(result));
        }

        Ok(Self {
            event,
            registration,
        })
    }

    /// Check if protocol appeared
    pub fn check(&self) -> bool {
        // TODO: Implement proper checking
        false
    }

    /// Wait for protocol to appear
    pub fn wait(&self) -> Result<Handle> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };

        loop {
            // Try to locate handle
            let mut handle = Handle(core::ptr::null_mut());
            let result = unsafe {
                ((*bs).locate_device_path)(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    &mut handle,
                )
            };

            if result == Status::SUCCESS && !handle.0.is_null() {
                return Ok(handle);
            }

            // Wait on event
            let events = [self.event];
            let mut index = 0usize;
            let result = unsafe { ((*bs).wait_for_event)(1, events.as_ptr(), &mut index) };

            if result != Status::SUCCESS {
                return Err(Error::from_status(result));
            }
        }
    }
}

impl Drop for ProtocolNotification {
    fn drop(&mut self) {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        unsafe { ((*bs).close_event)(self.event) };
    }
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_device_path_node() {
        let node = DevicePathNode {
            node_type: DevicePathNode::TYPE_HARDWARE,
            sub_type: 1,
            data: alloc::vec![0, 1, 2, 3],
        };

        assert!(!node.is_end());
        assert_eq!(node.type_name(), "Hardware");
    }

    #[test]
    fn test_device_path_end() {
        let node = DevicePathNode {
            node_type: DevicePathNode::TYPE_END,
            sub_type: 0xFF,
            data: alloc::vec![],
        };

        assert!(node.is_end());
        assert_eq!(node.type_name(), "End");
    }
}
