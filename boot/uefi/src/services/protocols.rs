//! Protocol Locator Services
//!
//! Safe wrappers for locating and using UEFI protocols.

use core::marker::PhantomData;

use super::boot::boot_services;
use crate::raw::types::*;

// =============================================================================
// PROTOCOL TRAIT
// =============================================================================

/// Trait for UEFI protocols
pub trait Protocol {
    /// Protocol GUID
    const GUID: Guid;
}

// =============================================================================
// PROTOCOL HANDLE
// =============================================================================

/// Safe protocol handle wrapper
pub struct ProtocolHandle<'a, P: Protocol> {
    /// The protocol pointer
    interface: *mut P,
    /// Handle that owns this protocol
    handle: Handle,
    /// Agent handle (our image)
    agent: Handle,
    /// Whether to close on drop
    close_on_drop: bool,
    /// Lifetime marker
    _marker: PhantomData<&'a P>,
}

impl<P: Protocol> ProtocolHandle<'_, P> {
    /// Create a new protocol handle
    ///
    /// # Safety
    /// Interface pointer must be valid.
    pub unsafe fn new(
        interface: *mut P,
        handle: Handle,
        agent: Handle,
        close_on_drop: bool,
    ) -> Self {
        Self {
            interface,
            handle,
            agent,
            close_on_drop,
            _marker: PhantomData,
        }
    }

    /// Get the protocol interface
    pub fn interface(&self) -> &P {
        unsafe { &*self.interface }
    }

    /// Get the protocol interface mutably
    pub fn interface_mut(&mut self) -> &mut P {
        unsafe { &mut *self.interface }
    }

    /// Get the handle that owns this protocol
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// Release ownership (don't close on drop)
    pub fn release(mut self) -> *mut P {
        self.close_on_drop = false;
        self.interface
    }
}

impl<P: Protocol> core::ops::Deref for ProtocolHandle<'_, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.interface }
    }
}

impl<P: Protocol> core::ops::DerefMut for ProtocolHandle<'_, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.interface }
    }
}
impl<P: Protocol> Drop for ProtocolHandle<'_, P> {
    fn drop(&mut self) {
        if self.close_on_drop && !self.interface.is_null() {
            unsafe {
                let _ = boot_services().close_protocol(
                    self.handle,
                    &P::GUID,
                    self.agent,
                    Handle::null(),
                );
            }
        }
    }
}

// =============================================================================
// PROTOCOL LOCATOR
// =============================================================================

/// Protocol locator helper
pub struct ProtocolLocator;

impl ProtocolLocator {
    /// Locate a single protocol instance
    pub fn locate<P: Protocol>() -> Result<*mut P, Status> {
        let bs = unsafe { boot_services() };
        bs.locate_protocol::<P>(&P::GUID)
    }

    /// Locate protocol on specific handle
    pub fn open_on_handle<P: Protocol>(
        handle: Handle,
        agent: Handle,
    ) -> Result<ProtocolHandle<'static, P>, Status> {
        let bs = unsafe { boot_services() };

        // Open with EXCLUSIVE access
        let interface = bs.open_protocol::<P>(
            handle,
            &P::GUID,
            agent,
            Handle::null(),
            OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
        )?;

        Ok(unsafe { ProtocolHandle::new(interface, handle, agent, true) })
    }

    /// Find all handles supporting a protocol
    pub fn find_handles<P: Protocol>() -> Result<alloc::vec::Vec<Handle>, Status> {
        let bs = unsafe { boot_services() };
        let buffer = bs.locate_handle_buffer(LocateSearchType::ByProtocol, Some(&P::GUID))?;
        Ok(buffer.handles().to_vec())
    }

    /// Open exclusive access to protocol
    pub fn open_exclusive<P: Protocol>(
        handle: Handle,
        agent: Handle,
    ) -> Result<ProtocolHandle<'static, P>, Status> {
        let bs = unsafe { boot_services() };

        let interface = bs.open_protocol::<P>(
            handle,
            &P::GUID,
            agent,
            Handle::null(),
            OPEN_PROTOCOL_EXCLUSIVE,
        )?;

        Ok(unsafe { ProtocolHandle::new(interface, handle, agent, true) })
    }

    /// Get protocol without opening (simple lookup)
    pub fn get<P: Protocol>(handle: Handle) -> Result<*mut P, Status> {
        let bs = unsafe { boot_services() };
        bs.handle_protocol::<P>(handle, &P::GUID)
    }
}

/// Open protocol attributes
pub const OPEN_PROTOCOL_BY_HANDLE_PROTOCOL: u32 = 0x0000_0001;
pub const OPEN_PROTOCOL_GET_PROTOCOL: u32 = 0x0000_0002;
pub const OPEN_PROTOCOL_TEST_PROTOCOL: u32 = 0x0000_0004;
pub const OPEN_PROTOCOL_BY_CHILD_CONTROLLER: u32 = 0x0000_0008;
pub const OPEN_PROTOCOL_BY_DRIVER: u32 = 0x0000_0010;
pub const OPEN_PROTOCOL_EXCLUSIVE: u32 = 0x0000_0020;

// =============================================================================
// COMMON PROTOCOL IMPLEMENTATIONS
// =============================================================================

impl Protocol for crate::raw::protocols::gop::EfiGraphicsOutputProtocol {
    const GUID: Guid = crate::raw::protocols::gop::EfiGraphicsOutputProtocol::GUID;
}

impl Protocol for crate::raw::protocols::file::EfiSimpleFileSystemProtocol {
    const GUID: Guid = crate::raw::protocols::file::EfiSimpleFileSystemProtocol::GUID;
}

impl Protocol for crate::raw::protocols::block::EfiBlockIoProtocol {
    const GUID: Guid = crate::raw::protocols::block::EfiBlockIoProtocol::GUID;
}

impl Protocol for crate::raw::protocols::pci::EfiPciIoProtocol {
    const GUID: Guid = crate::raw::protocols::pci::EfiPciIoProtocol::GUID;
}

impl Protocol for crate::raw::protocols::serial::EfiSerialIoProtocol {
    const GUID: Guid = crate::raw::protocols::serial::EfiSerialIoProtocol::GUID;
}

impl Protocol for crate::raw::protocols::loaded_image::EfiLoadedImageProtocol {
    const GUID: Guid = crate::raw::protocols::loaded_image::EfiLoadedImageProtocol::GUID;
}

impl Protocol for crate::raw::protocols::rng::EfiRngProtocol {
    const GUID: Guid = crate::raw::protocols::rng::EfiRngProtocol::GUID;
}

// =============================================================================
// PROTOCOL ITERATOR
// =============================================================================

/// Iterator over protocol handles
pub struct ProtocolIterator<P: Protocol + 'static> {
    handles: alloc::vec::Vec<Handle>,
    index: usize,
    agent: Handle,
    _marker: PhantomData<P>,
}

impl<P: Protocol + 'static> ProtocolIterator<P> {
    /// Create a new protocol iterator
    pub fn new(agent: Handle) -> Result<Self, Status> {
        let handles = ProtocolLocator::find_handles::<P>()?;
        Ok(Self {
            handles,
            index: 0,
            agent,
            _marker: PhantomData,
        })
    }

    /// Number of handles
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

impl<P: Protocol + 'static> Iterator for ProtocolIterator<P> {
    type Item = Result<ProtocolHandle<'static, P>, Status>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.handles.len() {
            return None;
        }

        let handle = self.handles[self.index];
        self.index += 1;

        Some(ProtocolLocator::open_on_handle::<P>(handle, self.agent))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.handles.len() - self.index;
        (remaining, Some(remaining))
    }
}

// =============================================================================
// SCOPED PROTOCOL
// =============================================================================

/// Scoped protocol access with custom cleanup
pub struct ScopedProtocol<'a, P: Protocol, F>
where
    F: FnOnce(&mut P),
{
    protocol: ProtocolHandle<'a, P>,
    cleanup: Option<F>,
}

impl<'a, P: Protocol, F: FnOnce(&mut P)> ScopedProtocol<'a, P, F> {
    /// Create a scoped protocol with cleanup function
    pub fn new(protocol: ProtocolHandle<'a, P>, cleanup: F) -> Self {
        Self {
            protocol,
            cleanup: Some(cleanup),
        }
    }

    /// Get the inner protocol handle
    pub fn inner(&self) -> &ProtocolHandle<'a, P> {
        &self.protocol
    }

    /// Get the inner protocol handle mutably
    pub fn inner_mut(&mut self) -> &mut ProtocolHandle<'a, P> {
        &mut self.protocol
    }
}

impl<'a, P: Protocol, F: FnOnce(&mut P)> core::ops::Deref for ScopedProtocol<'a, P, F> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.protocol.interface }
    }
}

impl<'a, P: Protocol, F: FnOnce(&mut P)> core::ops::DerefMut for ScopedProtocol<'a, P, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.protocol.interface }
    }
}

impl<'a, P: Protocol, F: FnOnce(&mut P)> Drop for ScopedProtocol<'a, P, F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(unsafe { &mut *self.protocol.interface });
        }
    }
}

// =============================================================================
// PROTOCOL EXISTENCE CHECK
// =============================================================================

/// Check if a protocol exists on a handle
pub fn has_protocol<P: Protocol>(handle: Handle) -> bool {
    ProtocolLocator::get::<P>(handle).is_ok()
}

/// Check if any instance of a protocol exists
pub fn protocol_available<P: Protocol>() -> bool {
    ProtocolLocator::locate::<P>().is_ok()
}

/// Count protocol instances
pub fn count_protocol_instances<P: Protocol>() -> Result<usize, Status> {
    Ok(ProtocolLocator::find_handles::<P>()?.len())
}

// =============================================================================
// PROTOCOL NOTIFICATION
// =============================================================================

/// Register for protocol installation notification
pub struct ProtocolNotification {
    event: crate::raw::types::Event,
    registration: *mut core::ffi::c_void,
}

impl ProtocolNotification {
    /// Create a new protocol notification
    ///
    /// # Safety
    /// The callback must be safe to call from UEFI context.
    pub unsafe fn new<P: Protocol>(
        callback: super::boot::EventNotify,
        context: *mut core::ffi::c_void,
    ) -> Result<Self, Status> {
        let bs = boot_services();

        let event = bs.create_event(
            super::boot::EventType::NOTIFY_SIGNAL,
            super::boot::TPL_CALLBACK,
            Some(callback),
            context,
        )?;

        let mut registration = core::ptr::null_mut();

        // Register protocol notify
        let status = (bs.as_ptr().as_ref().unwrap().register_protocol_notify)(
            &P::GUID,
            event,
            &mut registration,
        );

        if !status.is_success() {
            bs.close_event(event)?;
            return Err(status);
        }

        Ok(Self {
            event,
            registration,
        })
    }

    /// Get the registration key
    pub fn registration(&self) -> *mut core::ffi::c_void {
        self.registration
    }
}

impl Drop for ProtocolNotification {
    fn drop(&mut self) {
        if !self.event.is_null() {
            unsafe {
                let _ = boot_services().close_event(self.event);
            }
        }
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

    struct DummyProtocol;

    impl Protocol for DummyProtocol {
        const GUID: Guid = Guid::new(0, 0, 0, [0; 8]);
    }

    #[test]
    fn test_protocol_guid() {
        assert_eq!(DummyProtocol::GUID, Guid::new(0, 0, 0, [0; 8]));
    }
}
