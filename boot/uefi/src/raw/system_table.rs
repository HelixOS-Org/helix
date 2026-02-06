//! Raw UEFI System Table Definition
//!
//! The System Table is the primary structure provided to a UEFI application.
//! It contains pointers to the Boot Services, Runtime Services, and
//! Configuration Tables.

use core::ptr::NonNull;

use super::boot_services::EfiBootServices;
use super::runtime_services::EfiRuntimeServices;
use super::types::*;

// =============================================================================
// SYSTEM TABLE
// =============================================================================

/// EFI System Table
///
/// This is the main structure provided to every UEFI application on entry.
/// It provides access to all UEFI services and configuration tables.
#[derive(Debug)]
#[repr(C)]
pub struct EfiSystemTable {
    /// Table header
    pub hdr: TableHeader,

    /// Pointer to null-terminated firmware vendor string (UCS-2)
    pub firmware_vendor: *const Char16,

    /// Firmware revision number
    pub firmware_revision: u32,

    /// Handle for console input device
    pub console_in_handle: Handle,

    /// Pointer to Simple Text Input protocol
    pub con_in: *mut EfiSimpleTextInputProtocol,

    /// Handle for console output device
    pub console_out_handle: Handle,

    /// Pointer to Simple Text Output protocol
    pub con_out: *mut EfiSimpleTextOutputProtocol,

    /// Handle for standard error device
    pub standard_error_handle: Handle,

    /// Pointer to Simple Text Output protocol for standard error
    pub std_err: *mut EfiSimpleTextOutputProtocol,

    /// Pointer to Runtime Services table
    pub runtime_services: *mut EfiRuntimeServices,

    /// Pointer to Boot Services table
    pub boot_services: *mut EfiBootServices,

    /// Number of entries in configuration table
    pub number_of_table_entries: usize,

    /// Pointer to array of configuration tables
    pub configuration_table: *mut ConfigurationTable,
}

impl EfiSystemTable {
    /// System Table signature: "IBI SYST"
    pub const SIGNATURE: u64 = TableHeader::SYSTEM_TABLE_SIGNATURE;

    /// Validate the system table
    pub fn validate(&self) -> bool {
        self.hdr.validate(Self::SIGNATURE)
    }

    /// Get the UEFI specification version
    pub fn uefi_version(&self) -> (u16, u16) {
        self.hdr.version()
    }

    /// Get boot services (unsafe - caller must ensure boot services not exited)
    ///
    /// # Safety
    /// The caller must ensure that ExitBootServices has not been called.
    pub unsafe fn boot_services(&self) -> Option<&EfiBootServices> {
        if self.boot_services.is_null() {
            None
        } else {
            Some(&*self.boot_services)
        }
    }

    /// Get mutable boot services
    ///
    /// # Safety
    /// The caller must ensure that ExitBootServices has not been called.
    pub unsafe fn boot_services_mut(&mut self) -> Option<&mut EfiBootServices> {
        if self.boot_services.is_null() {
            None
        } else {
            Some(&mut *self.boot_services)
        }
    }

    /// Get runtime services
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn runtime_services(&self) -> Option<&EfiRuntimeServices> {
        if self.runtime_services.is_null() {
            None
        } else {
            Some(&*self.runtime_services)
        }
    }

    /// Get mutable runtime services
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn runtime_services_mut(&mut self) -> Option<&mut EfiRuntimeServices> {
        if self.runtime_services.is_null() {
            None
        } else {
            Some(&mut *self.runtime_services)
        }
    }

    /// Get console input protocol
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn con_in(&self) -> Option<&EfiSimpleTextInputProtocol> {
        if self.con_in.is_null() {
            None
        } else {
            Some(&*self.con_in)
        }
    }

    /// Get console output protocol
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn con_out(&self) -> Option<&EfiSimpleTextOutputProtocol> {
        if self.con_out.is_null() {
            None
        } else {
            Some(&*self.con_out)
        }
    }

    /// Get console output protocol (mutable)
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn con_out_mut(&mut self) -> Option<&mut EfiSimpleTextOutputProtocol> {
        if self.con_out.is_null() {
            None
        } else {
            Some(&mut *self.con_out)
        }
    }

    /// Get standard error protocol
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn std_err(&self) -> Option<&EfiSimpleTextOutputProtocol> {
        if self.std_err.is_null() {
            None
        } else {
            Some(&*self.std_err)
        }
    }

    /// Get the firmware vendor string
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn firmware_vendor(&self) -> Option<&[Char16]> {
        if self.firmware_vendor.is_null() {
            return None;
        }

        // Find null terminator
        let mut len = 0;
        let mut ptr = self.firmware_vendor;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
            // Safety limit
            if len > 1024 {
                break;
            }
        }

        Some(core::slice::from_raw_parts(self.firmware_vendor, len))
    }

    /// Get configuration tables
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn configuration_tables(&self) -> &[ConfigurationTable] {
        if self.configuration_table.is_null() || self.number_of_table_entries == 0 {
            return &[];
        }

        core::slice::from_raw_parts(self.configuration_table, self.number_of_table_entries)
    }

    /// Find a configuration table by GUID
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn find_config_table(&self, guid: &Guid) -> Option<*mut core::ffi::c_void> {
        for table in self.configuration_tables() {
            if table.vendor_guid == *guid {
                return Some(table.vendor_table);
            }
        }
        None
    }

    /// Find ACPI RSDP (tries ACPI 2.0 first, then 1.0)
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn find_acpi_rsdp(&self) -> Option<*mut core::ffi::c_void> {
        // Try ACPI 2.0+ first
        if let Some(ptr) = self.find_config_table(&guids::ACPI_20_TABLE) {
            return Some(ptr);
        }
        // Fall back to ACPI 1.0
        self.find_config_table(&guids::ACPI_TABLE)
    }

    /// Find SMBIOS entry point (tries 3.0 first, then 2.x)
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn find_smbios(&self) -> Option<*mut core::ffi::c_void> {
        // Try SMBIOS 3.0 first
        if let Some(ptr) = self.find_config_table(&guids::SMBIOS3_TABLE) {
            return Some(ptr);
        }
        // Fall back to SMBIOS 2.x
        self.find_config_table(&guids::SMBIOS_TABLE)
    }

    /// Find Device Tree blob
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn find_device_tree(&self) -> Option<*mut core::ffi::c_void> {
        self.find_config_table(&guids::DEVICE_TREE)
    }
}

// Safety: EfiSystemTable is only accessed in single-threaded boot context
unsafe impl Send for EfiSystemTable {}
unsafe impl Sync for EfiSystemTable {}

// =============================================================================
// SIMPLE TEXT INPUT PROTOCOL
// =============================================================================

/// Simple Text Input Protocol
#[repr(C)]
pub struct EfiSimpleTextInputProtocol {
    /// Reset the input device
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: Boolean) -> Status,

    /// Read a keystroke from the input device
    pub read_key_stroke: unsafe extern "efiapi" fn(this: *mut Self, key: *mut InputKey) -> Status,

    /// Event to wait for a keystroke
    pub wait_for_key: Event,
}

impl EfiSimpleTextInputProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::SIMPLE_TEXT_INPUT_PROTOCOL;

    /// Reset the input device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn reset(&mut self, extended_verification: bool) -> Status {
        (self.reset)(self, extended_verification as Boolean)
    }

    /// Read a keystroke
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn read_key(&mut self) -> Result<InputKey, Status> {
        let mut key = InputKey::default();
        let status = (self.read_key_stroke)(self, &mut key);
        if status.is_success() {
            Ok(key)
        } else {
            Err(status)
        }
    }
}

impl core::fmt::Debug for EfiSimpleTextInputProtocol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiSimpleTextInputProtocol")
            .field("wait_for_key", &self.wait_for_key)
            .finish()
    }
}

// =============================================================================
// SIMPLE TEXT OUTPUT PROTOCOL
// =============================================================================

/// Simple Text Output Mode
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SimpleTextOutputMode {
    /// Maximum mode number supported
    pub max_mode: i32,
    /// Current mode number
    pub mode: i32,
    /// Current character attribute
    pub attribute: i32,
    /// Current cursor column
    pub cursor_column: i32,
    /// Current cursor row
    pub cursor_row: i32,
    /// Whether the cursor is visible
    pub cursor_visible: Boolean,
}

/// Simple Text Output Protocol
#[repr(C)]
pub struct EfiSimpleTextOutputProtocol {
    /// Reset the text output device
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: Boolean) -> Status,

    /// Write a string to the output device
    pub output_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const Char16) -> Status,

    /// Test if a string can be output
    pub test_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const Char16) -> Status,

    /// Query mode information
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> Status,

    /// Set the output mode
    pub set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: usize) -> Status,

    /// Set the text attribute
    pub set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attribute: usize) -> Status,

    /// Clear the screen
    pub clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Set cursor position
    pub set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, column: usize, row: usize) -> Status,

    /// Enable or disable cursor
    pub enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: Boolean) -> Status,

    /// Pointer to mode information
    pub mode: *mut SimpleTextOutputMode,
}

impl EfiSimpleTextOutputProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::SIMPLE_TEXT_OUTPUT_PROTOCOL;

    // Text attributes
    pub const BLACK: usize = 0x00;
    pub const BLUE: usize = 0x01;
    pub const GREEN: usize = 0x02;
    pub const CYAN: usize = 0x03;
    pub const RED: usize = 0x04;
    pub const MAGENTA: usize = 0x05;
    pub const BROWN: usize = 0x06;
    pub const LIGHTGRAY: usize = 0x07;
    pub const DARKGRAY: usize = 0x08;
    pub const LIGHTBLUE: usize = 0x09;
    pub const LIGHTGREEN: usize = 0x0A;
    pub const LIGHTCYAN: usize = 0x0B;
    pub const LIGHTRED: usize = 0x0C;
    pub const LIGHTMAGENTA: usize = 0x0D;
    pub const YELLOW: usize = 0x0E;
    pub const WHITE: usize = 0x0F;

    pub const BACKGROUND_BLACK: usize = 0x00;
    pub const BACKGROUND_BLUE: usize = 0x10;
    pub const BACKGROUND_GREEN: usize = 0x20;
    pub const BACKGROUND_CYAN: usize = 0x30;
    pub const BACKGROUND_RED: usize = 0x40;
    pub const BACKGROUND_MAGENTA: usize = 0x50;
    pub const BACKGROUND_BROWN: usize = 0x60;
    pub const BACKGROUND_LIGHTGRAY: usize = 0x70;

    /// Reset the output device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn reset(&mut self, extended_verification: bool) -> Status {
        (self.reset)(self, extended_verification as Boolean)
    }

    /// Output a string (UCS-2)
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and string are valid.
    pub unsafe fn output_string_raw(&mut self, string: *const Char16) -> Status {
        (self.output_string)(self, string)
    }

    /// Output a string slice
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn output_string(&mut self, string: &[Char16]) -> Status {
        // The string must be null-terminated for UEFI
        // This is a limitation - caller should ensure null termination
        if string.is_empty() || string[string.len() - 1] != 0 {
            return Status::INVALID_PARAMETER;
        }
        (self.output_string)(self, string.as_ptr())
    }

    /// Output an ASCII string (converts to UCS-2)
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn output_ascii(&mut self, s: &str) -> Status {
        // Convert ASCII to UCS-2 and output
        // Buffer on stack, limited size
        let mut buffer = [0u16; 256];
        let bytes = s.as_bytes();
        let len = core::cmp::min(bytes.len(), buffer.len() - 1);

        for (i, &b) in bytes[..len].iter().enumerate() {
            buffer[i] = b as u16;
        }
        buffer[len] = 0; // Null terminator

        (self.output_string)(self, buffer.as_ptr())
    }

    /// Clear the screen
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn clear(&mut self) -> Status {
        (self.clear_screen)(self)
    }

    /// Set text attribute (foreground and background color)
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_attr(&mut self, foreground: usize, background: usize) -> Status {
        (self.set_attribute)(self, foreground | background)
    }

    /// Set cursor position
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_cursor(&mut self, column: usize, row: usize) -> Status {
        (self.set_cursor_position)(self, column, row)
    }

    /// Enable or disable cursor
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn cursor_visible(&mut self, visible: bool) -> Status {
        (self.enable_cursor)(self, visible as Boolean)
    }

    /// Query mode dimensions
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn query_mode(&self, mode: usize) -> Result<(usize, usize), Status> {
        let mut columns = 0;
        let mut rows = 0;
        let status = (self.query_mode)(self as *const _ as *mut _, mode, &mut columns, &mut rows);
        if status.is_success() {
            Ok((columns, rows))
        } else {
            Err(status)
        }
    }

    /// Set display mode
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_mode(&mut self, mode: usize) -> Status {
        (self.set_mode)(self, mode)
    }

    /// Get current mode information
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn current_mode(&self) -> Option<&SimpleTextOutputMode> {
        if self.mode.is_null() {
            None
        } else {
            Some(&*self.mode)
        }
    }
}

impl core::fmt::Debug for EfiSimpleTextOutputProtocol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiSimpleTextOutputProtocol")
            .field("mode", &self.mode)
            .finish()
    }
}

// =============================================================================
// SYSTEM TABLE WRAPPER
// =============================================================================

/// Safe wrapper around the system table pointer
pub struct SystemTablePtr(NonNull<EfiSystemTable>);

impl SystemTablePtr {
    /// Create a new system table wrapper
    ///
    /// # Safety
    /// The pointer must be valid and point to a valid EFI System Table.
    pub unsafe fn new(ptr: *mut EfiSystemTable) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    /// Get reference to the system table
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref(&self) -> &EfiSystemTable {
        unsafe { self.0.as_ref() }
    }

    /// Get mutable reference to the system table
    #[allow(clippy::should_implement_trait)]
    pub fn as_mut(&mut self) -> &mut EfiSystemTable {
        unsafe { self.0.as_mut() }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut EfiSystemTable {
        self.0.as_ptr()
    }
}

impl core::fmt::Debug for SystemTablePtr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SystemTablePtr({:p})", self.0)
    }
}

// Safety: SystemTablePtr is only accessed in single-threaded boot context
unsafe impl Send for SystemTablePtr {}
unsafe impl Sync for SystemTablePtr {}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_system_table_signature() {
        assert_eq!(EfiSystemTable::SIGNATURE, 0x5453595320494249);
    }

    #[test]
    fn test_text_attributes() {
        let attr =
            EfiSimpleTextOutputProtocol::WHITE | EfiSimpleTextOutputProtocol::BACKGROUND_BLUE;
        assert_eq!(attr, 0x1F);
    }
}
