//! Variable Services
//!
//! High-level interface for UEFI variables.

use super::runtime::runtime_services;
use crate::raw::runtime_services::variable_attributes;
use crate::raw::types::*;

// =============================================================================
// VARIABLE
// =============================================================================

/// UEFI Variable
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name (UTF-16)
    pub name: alloc::vec::Vec<u16>,
    /// Vendor GUID
    pub vendor_guid: Guid,
    /// Attributes
    pub attributes: VariableAttributes,
    /// Data
    pub data: alloc::vec::Vec<u8>,
}

impl Variable {
    /// Create a new variable
    pub fn new(
        name: &str,
        vendor_guid: Guid,
        attributes: VariableAttributes,
        data: alloc::vec::Vec<u8>,
    ) -> Self {
        Self {
            name: string_to_utf16(name),
            vendor_guid,
            attributes,
            data,
        }
    }

    /// Get name as string
    pub fn name_string(&self) -> alloc::string::String {
        utf16_to_string(&self.name)
    }

    /// Read variable from UEFI
    pub fn read(name: &str, vendor_guid: &Guid) -> Result<Self, Status> {
        let name_utf16 = string_to_utf16(name);
        Self::read_utf16(&name_utf16, vendor_guid)
    }

    /// Read variable from UEFI (UTF-16 name)
    pub fn read_utf16(name: &[u16], vendor_guid: &Guid) -> Result<Self, Status> {
        let rs = unsafe { runtime_services() };

        // First, try with a small buffer to get size
        let mut buffer = alloc::vec![0u8; 256];
        let result = rs.get_variable(name, vendor_guid, &mut buffer);

        let (attributes, size) = match result {
            Ok((attrs, size)) => (attrs, size),
            Err(Status::BUFFER_TOO_SMALL) => {
                // Retry with larger buffer (get size from error context)
                // For now, try increasingly larger buffers
                for size in [1024, 4096, 16384, 65536] {
                    buffer.resize(size, 0);
                    if let Ok((attrs, actual_size)) =
                        rs.get_variable(name, vendor_guid, &mut buffer)
                    {
                        return Ok(Self {
                            name: name.to_vec(),
                            vendor_guid: *vendor_guid,
                            attributes: VariableAttributes::from_bits(attrs),
                            data: buffer[..actual_size].to_vec(),
                        });
                    }
                }
                return Err(Status::BUFFER_TOO_SMALL);
            },
            Err(e) => return Err(e),
        };

        Ok(Self {
            name: name.to_vec(),
            vendor_guid: *vendor_guid,
            attributes: VariableAttributes::from_bits(attributes),
            data: buffer[..size].to_vec(),
        })
    }

    /// Write variable to UEFI
    pub fn write(&self) -> Result<(), Status> {
        let rs = unsafe { runtime_services() };
        rs.set_variable(
            &self.name,
            &self.vendor_guid,
            self.attributes.bits(),
            &self.data,
        )
    }

    /// Delete the variable
    pub fn delete(&self) -> Result<(), Status> {
        let rs = unsafe { runtime_services() };
        rs.delete_variable(&self.name, &self.vendor_guid)
    }
}

// =============================================================================
// VARIABLE ATTRIBUTES
// =============================================================================

/// Variable attributes wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableAttributes(u32);

impl VariableAttributes {
    /// Non-volatile (persists across reboot)
    pub const NON_VOLATILE: Self = Self(variable_attributes::NON_VOLATILE);

    /// Accessible at boot time
    pub const BOOT_SERVICE_ACCESS: Self = Self(variable_attributes::BOOTSERVICE_ACCESS);

    /// Accessible at runtime
    pub const RUNTIME_ACCESS: Self = Self(variable_attributes::RUNTIME_ACCESS);

    /// Hardware error record
    pub const HARDWARE_ERROR_RECORD: Self = Self(variable_attributes::HARDWARE_ERROR_RECORD);

    /// Authenticated write access
    pub const AUTHENTICATED_WRITE_ACCESS: Self =
        Self(variable_attributes::AUTHENTICATED_WRITE_ACCESS);

    /// Time-based authenticated write
    pub const TIME_BASED_AUTHENTICATED_WRITE: Self =
        Self(variable_attributes::TIME_BASED_AUTHENTICATED_WRITE_ACCESS);

    /// Append write
    pub const APPEND_WRITE: Self = Self(variable_attributes::APPEND_WRITE);

    /// Enhanced authenticated access
    pub const ENHANCED_AUTHENTICATED_ACCESS: Self =
        Self(variable_attributes::ENHANCED_AUTHENTICATED_ACCESS);

    /// Standard attributes for non-volatile runtime variable
    pub const NV_BS_RT: Self = Self(
        variable_attributes::NON_VOLATILE
            | variable_attributes::BOOTSERVICE_ACCESS
            | variable_attributes::RUNTIME_ACCESS,
    );

    /// Standard attributes for boot-time only variable
    pub const BS: Self = Self(variable_attributes::BOOTSERVICE_ACCESS);

    /// Standard attributes for runtime-accessible variable (volatile)
    pub const BS_RT: Self =
        Self(variable_attributes::BOOTSERVICE_ACCESS | variable_attributes::RUNTIME_ACCESS);

    /// Create empty attributes
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create from raw bits
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Check if contains attribute
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if non-volatile
    pub const fn is_non_volatile(&self) -> bool {
        self.contains(Self::NON_VOLATILE)
    }

    /// Check if runtime accessible
    pub const fn is_runtime_accessible(&self) -> bool {
        self.contains(Self::RUNTIME_ACCESS)
    }
}

impl core::ops::BitOr for VariableAttributes {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for VariableAttributes {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Default for VariableAttributes {
    fn default() -> Self {
        Self::BS
    }
}

// =============================================================================
// VARIABLE ITERATOR
// =============================================================================

/// Iterator over all UEFI variables
pub struct VariableIterator {
    name_buffer: alloc::vec::Vec<u16>,
    vendor_guid: Guid,
    started: bool,
    finished: bool,
}

impl VariableIterator {
    /// Create a new variable iterator
    pub fn new() -> Self {
        let mut name_buffer = alloc::vec![0u16; 256];
        name_buffer[0] = 0; // Start with empty name

        Self {
            name_buffer,
            vendor_guid: Guid::NULL,
            started: false,
            finished: false,
        }
    }
}

impl Default for VariableIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for VariableIterator {
    type Item = (alloc::vec::Vec<u16>, Guid);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let rs = unsafe { runtime_services() };

        loop {
            let result = rs.get_next_variable_name(&mut self.name_buffer, &mut self.vendor_guid);

            match result {
                Ok(len) => {
                    self.started = true;
                    let name = self.name_buffer[..len].to_vec();
                    return Some((name, self.vendor_guid));
                },
                Err(Status::BUFFER_TOO_SMALL) => {
                    // Grow buffer and retry
                    let new_size = self.name_buffer.len() * 2;
                    self.name_buffer.resize(new_size, 0);
                },
                Err(Status::NOT_FOUND) => {
                    self.finished = true;
                    return None;
                },
                Err(_) => {
                    self.finished = true;
                    return None;
                },
            }
        }
    }
}

// =============================================================================
// WELL-KNOWN VARIABLES
// =============================================================================

/// Global variable vendor GUID
pub const GLOBAL_VARIABLE_GUID: Guid = guids::GLOBAL_VARIABLE;

/// Well-known variable names
pub mod variable_names {
    pub use crate::raw::runtime_services::variable_names::*;
}

/// Read a global variable by name
pub fn read_global_variable(name: &str) -> Result<Variable, Status> {
    Variable::read(name, &GLOBAL_VARIABLE_GUID)
}

/// Write a global variable
pub fn write_global_variable(
    name: &str,
    attributes: VariableAttributes,
    data: &[u8],
) -> Result<(), Status> {
    let var = Variable::new(name, GLOBAL_VARIABLE_GUID, attributes, data.to_vec());
    var.write()
}

/// Delete a global variable
pub fn delete_global_variable(name: &str) -> Result<(), Status> {
    let name_utf16 = string_to_utf16(name);
    let rs = unsafe { runtime_services() };
    rs.delete_variable(&name_utf16, &GLOBAL_VARIABLE_GUID)
}

// =============================================================================
// BOOT ORDER VARIABLES
// =============================================================================

/// Read boot order
pub fn read_boot_order() -> Result<alloc::vec::Vec<u16>, Status> {
    let var = read_global_variable("BootOrder")?;

    // Convert bytes to u16 array
    let count = var.data.len() / 2;
    let mut order = alloc::vec::Vec::with_capacity(count);

    for chunk in var.data.chunks(2) {
        if chunk.len() == 2 {
            order.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
    }

    Ok(order)
}

/// Write boot order
pub fn write_boot_order(order: &[u16]) -> Result<(), Status> {
    let mut data = alloc::vec::Vec::with_capacity(order.len() * 2);

    for &entry in order {
        data.extend_from_slice(&entry.to_le_bytes());
    }

    write_global_variable("BootOrder", VariableAttributes::NV_BS_RT, &data)
}

/// Read a boot option
pub fn read_boot_option(num: u16) -> Result<BootOption, Status> {
    let name = alloc::format!("Boot{:04X}", num);
    let var = read_global_variable(&name)?;
    BootOption::parse(&var.data)
}

/// Boot option structure
#[derive(Debug, Clone)]
pub struct BootOption {
    /// Attributes
    pub attributes: u32,
    /// Description
    pub description: alloc::string::String,
    /// Device path (raw bytes)
    pub device_path: alloc::vec::Vec<u8>,
    /// Optional data
    pub optional_data: alloc::vec::Vec<u8>,
}

impl BootOption {
    /// Active attribute
    pub const ACTIVE: u32 = 0x00000001;
    /// Force reconnect attribute
    pub const FORCE_RECONNECT: u32 = 0x00000002;
    /// Hidden attribute
    pub const HIDDEN: u32 = 0x00000008;
    /// Category mask
    pub const CATEGORY_MASK: u32 = 0x00001F00;
    /// Category boot
    pub const CATEGORY_BOOT: u32 = 0x00000000;
    /// Category app
    pub const CATEGORY_APP: u32 = 0x00000100;

    /// Parse boot option from raw data
    pub fn parse(data: &[u8]) -> Result<Self, Status> {
        if data.len() < 6 {
            return Err(Status::INVALID_PARAMETER);
        }

        let attributes = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let device_path_len = u16::from_le_bytes([data[4], data[5]]) as usize;

        let mut offset = 6;

        // Parse description (null-terminated UTF-16)
        let mut description_end = offset;
        while description_end + 1 < data.len() {
            let ch = u16::from_le_bytes([data[description_end], data[description_end + 1]]);
            if ch == 0 {
                break;
            }
            description_end += 2;
        }

        let description_bytes = &data[offset..description_end];
        let description: alloc::vec::Vec<u16> = description_bytes
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let description = utf16_to_string(&description);

        offset = description_end + 2; // Skip null terminator

        // Device path
        let device_path_end = (offset + device_path_len).min(data.len());
        let device_path = data[offset..device_path_end].to_vec();
        offset = device_path_end;

        // Optional data
        let optional_data = if offset < data.len() {
            data[offset..].to_vec()
        } else {
            alloc::vec::Vec::new()
        };

        Ok(Self {
            attributes,
            description,
            device_path,
            optional_data,
        })
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        (self.attributes & Self::ACTIVE) != 0
    }

    /// Check if hidden
    pub fn is_hidden(&self) -> bool {
        (self.attributes & Self::HIDDEN) != 0
    }
}

// =============================================================================
// SECURE BOOT VARIABLES
// =============================================================================

/// Check if secure boot is enabled
pub fn is_secure_boot_enabled() -> Result<bool, Status> {
    let var = read_global_variable("SecureBoot")?;
    Ok(!var.data.is_empty() && var.data[0] == 1)
}

/// Check if setup mode is active
pub fn is_setup_mode() -> Result<bool, Status> {
    let var = read_global_variable("SetupMode")?;
    Ok(!var.data.is_empty() && var.data[0] == 1)
}

/// Read platform key (PK)
pub fn read_platform_key() -> Result<Variable, Status> {
    read_global_variable("PK")
}

/// Read key exchange keys (KEK)
pub fn read_kek() -> Result<Variable, Status> {
    read_global_variable("KEK")
}

/// Read allowed signatures database (db)
pub fn read_db() -> Result<Variable, Status> {
    // db uses the image security database GUID
    Variable::read("db", &guids::IMAGE_SECURITY_DATABASE)
}

/// Read forbidden signatures database (dbx)
pub fn read_dbx() -> Result<Variable, Status> {
    Variable::read("dbx", &guids::IMAGE_SECURITY_DATABASE)
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Convert string to null-terminated UTF-16
pub fn string_to_utf16(s: &str) -> alloc::vec::Vec<u16> {
    let mut result: alloc::vec::Vec<u16> = s.encode_utf16().collect();
    result.push(0); // Null terminator
    result
}

/// Convert UTF-16 to string
pub fn utf16_to_string(s: &[u16]) -> alloc::string::String {
    // Find null terminator
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    char::decode_utf16(s[..len].iter().copied())
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

// =============================================================================
// VARIABLE INFO
// =============================================================================

/// Get variable storage information
pub fn get_variable_storage_info(
    attributes: VariableAttributes,
) -> Result<VariableStorageInfo, Status> {
    let rs = unsafe { runtime_services() };

    let info = rs.query_variable_info(attributes.bits())?;

    Ok(VariableStorageInfo {
        max_storage: info.max_storage,
        remaining_storage: info.remaining_storage,
        max_variable_size: info.max_variable_size,
    })
}

/// Variable storage information
#[derive(Debug, Clone, Copy)]
pub struct VariableStorageInfo {
    /// Maximum total storage
    pub max_storage: u64,
    /// Remaining storage
    pub remaining_storage: u64,
    /// Maximum single variable size
    pub max_variable_size: u64,
}

impl VariableStorageInfo {
    /// Get used storage
    pub fn used_storage(&self) -> u64 {
        self.max_storage.saturating_sub(self.remaining_storage)
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.max_storage == 0 {
            0.0
        } else {
            (self.used_storage() as f32 / self.max_storage as f32) * 100.0
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

    #[test]
    fn test_variable_attributes() {
        let attrs = VariableAttributes::NV_BS_RT;
        assert!(attrs.is_non_volatile());
        assert!(attrs.is_runtime_accessible());
    }

    #[test]
    fn test_string_conversion() {
        let s = "Hello";
        let utf16 = string_to_utf16(s);
        assert_eq!(utf16, &[72, 101, 108, 108, 111, 0]);

        let back = utf16_to_string(&utf16);
        assert_eq!(back, "Hello");
    }
}
