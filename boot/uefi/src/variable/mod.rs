//! Variable Services
//!
//! UEFI variable storage and runtime variable access.

use core::fmt;

// =============================================================================
// VARIABLE ATTRIBUTES
// =============================================================================

/// Variable attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct VariableAttributes(u32);

impl VariableAttributes {
    /// Non-volatile (survives reset)
    pub const NON_VOLATILE: Self = Self(0x00000001);

    /// Bootservice access
    pub const BOOTSERVICE_ACCESS: Self = Self(0x00000002);

    /// Runtime access
    pub const RUNTIME_ACCESS: Self = Self(0x00000004);

    /// Hardware error record
    pub const HARDWARE_ERROR_RECORD: Self = Self(0x00000008);

    /// Authenticated write access (deprecated)
    pub const AUTHENTICATED_WRITE_ACCESS: Self = Self(0x00000010);

    /// Time based authenticated write
    pub const TIME_BASED_AUTHENTICATED_WRITE_ACCESS: Self = Self(0x00000020);

    /// Append write
    pub const APPEND_WRITE: Self = Self(0x00000040);

    /// Enhanced authenticated access
    pub const ENHANCED_AUTHENTICATED_ACCESS: Self = Self(0x00000080);

    /// Common attributes for boot variables
    pub const BOOT_VAR: Self = Self(0x00000007); // NV + BS + RT

    /// Common attributes for runtime variables
    pub const RT_VAR: Self = Self(0x00000006); // BS + RT

    /// Empty
    pub const NONE: Self = Self(0);

    /// Create from raw value
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get raw value
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Combine attributes
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if contains attribute
    pub fn contains(self, attr: Self) -> bool {
        self.0 & attr.0 == attr.0
    }

    /// Is non-volatile
    pub fn is_non_volatile(self) -> bool {
        self.contains(Self::NON_VOLATILE)
    }

    /// Has boot service access
    pub fn is_bootservice_access(self) -> bool {
        self.contains(Self::BOOTSERVICE_ACCESS)
    }

    /// Has runtime access
    pub fn is_runtime_access(self) -> bool {
        self.contains(Self::RUNTIME_ACCESS)
    }

    /// Is authenticated
    pub fn is_authenticated(self) -> bool {
        self.contains(Self::TIME_BASED_AUTHENTICATED_WRITE_ACCESS)
            || self.contains(Self::AUTHENTICATED_WRITE_ACCESS)
    }
}

impl Default for VariableAttributes {
    fn default() -> Self {
        Self::NONE
    }
}

// =============================================================================
// VARIABLE NAME
// =============================================================================

/// Variable name with GUID
#[derive(Clone)]
pub struct VariableName<const N: usize> {
    /// Variable name (UCS-2)
    name: [u16; N],
    /// Name length
    len: usize,
    /// Vendor GUID
    vendor_guid: [u8; 16],
}

impl<const N: usize> VariableName<N> {
    /// Create new variable name
    pub fn new(name: &str, vendor_guid: &[u8; 16]) -> Self {
        let mut result = Self {
            name: [0; N],
            len: 0,
            vendor_guid: *vendor_guid,
        };

        for c in name.chars() {
            if result.len >= N - 1 {
                break;
            }
            if (c as u32) <= 0xFFFF {
                result.name[result.len] = c as u16;
                result.len += 1;
            }
        }

        result.name[result.len] = 0; // Null terminate
        result
    }

    /// Get name as slice
    pub fn as_slice(&self) -> &[u16] {
        &self.name[..self.len]
    }

    /// Get name as pointer
    pub fn as_ptr(&self) -> *const u16 {
        self.name.as_ptr()
    }

    /// Get vendor GUID
    pub fn vendor_guid(&self) -> &[u8; 16] {
        &self.vendor_guid
    }

    /// Get name length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// =============================================================================
// VARIABLE INFO
// =============================================================================

/// Variable information
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Attributes
    pub attributes: VariableAttributes,
    /// Data size
    pub size: usize,
}

impl VariableInfo {
    /// Create new info
    pub const fn new(attributes: VariableAttributes, size: usize) -> Self {
        Self { attributes, size }
    }
}

// =============================================================================
// VARIABLE STORAGE (SOFTWARE EMULATION)
// =============================================================================

/// Maximum variable size
pub const MAX_VARIABLE_SIZE: usize = 4096;

/// Maximum variable count
pub const MAX_VARIABLE_COUNT: usize = 64;

/// Maximum variable name length
pub const MAX_VARIABLE_NAME_LEN: usize = 128;

/// Stored variable entry
#[derive(Clone)]
pub struct StoredVariable {
    /// Is in use
    in_use: bool,
    /// Variable name (UCS-2, null-terminated)
    name: [u16; MAX_VARIABLE_NAME_LEN],
    /// Name length
    name_len: usize,
    /// Vendor GUID
    vendor_guid: [u8; 16],
    /// Attributes
    attributes: VariableAttributes,
    /// Data
    data: [u8; MAX_VARIABLE_SIZE],
    /// Data size
    data_size: usize,
}

impl StoredVariable {
    /// Create empty entry
    pub const fn empty() -> Self {
        Self {
            in_use: false,
            name: [0; MAX_VARIABLE_NAME_LEN],
            name_len: 0,
            vendor_guid: [0; 16],
            attributes: VariableAttributes::NONE,
            data: [0; MAX_VARIABLE_SIZE],
            data_size: 0,
        }
    }

    /// Is in use
    pub fn is_in_use(&self) -> bool {
        self.in_use
    }

    /// Get name as slice
    pub fn name(&self) -> &[u16] {
        &self.name[..self.name_len]
    }

    /// Get vendor GUID
    pub fn vendor_guid(&self) -> &[u8; 16] {
        &self.vendor_guid
    }

    /// Get attributes
    pub fn attributes(&self) -> VariableAttributes {
        self.attributes
    }

    /// Get data
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_size]
    }

    /// Get data size
    pub fn data_size(&self) -> usize {
        self.data_size
    }

    /// Match name and GUID
    pub fn matches(&self, name: &[u16], vendor_guid: &[u8; 16]) -> bool {
        if !self.in_use {
            return false;
        }

        if self.vendor_guid != *vendor_guid {
            return false;
        }

        if self.name_len != name.len() {
            return false;
        }

        // Compare names (case-insensitive for ASCII)
        for i in 0..self.name_len {
            let c1 = self.name[i];
            let c2 = name[i];

            // Case-insensitive compare for ASCII letters
            let c1_lower = if c1 >= 0x41 && c1 <= 0x5A {
                c1 + 32
            } else {
                c1
            };
            let c2_lower = if c2 >= 0x41 && c2 <= 0x5A {
                c2 + 32
            } else {
                c2
            };

            if c1_lower != c2_lower {
                return false;
            }
        }

        true
    }

    /// Set variable
    pub fn set(
        &mut self,
        name: &[u16],
        vendor_guid: &[u8; 16],
        attributes: VariableAttributes,
        data: &[u8],
    ) -> bool {
        if name.len() > MAX_VARIABLE_NAME_LEN - 1 || data.len() > MAX_VARIABLE_SIZE {
            return false;
        }

        self.in_use = true;
        self.name[..name.len()].copy_from_slice(name);
        self.name[name.len()] = 0;
        self.name_len = name.len();
        self.vendor_guid = *vendor_guid;
        self.attributes = attributes;
        self.data[..data.len()].copy_from_slice(data);
        self.data_size = data.len();

        true
    }

    /// Clear variable
    pub fn clear(&mut self) {
        self.in_use = false;
        self.data_size = 0;
    }
}

impl Default for StoredVariable {
    fn default() -> Self {
        Self::empty()
    }
}

/// Variable storage
pub struct VariableStorage {
    /// Variables
    variables: [StoredVariable; MAX_VARIABLE_COUNT],
    /// Variable count
    count: usize,
}

impl VariableStorage {
    /// Create new storage
    pub const fn new() -> Self {
        const EMPTY: StoredVariable = StoredVariable::empty();
        Self {
            variables: [EMPTY; MAX_VARIABLE_COUNT],
            count: 0,
        }
    }

    /// Get variable
    pub fn get(&self, name: &[u16], vendor_guid: &[u8; 16]) -> Option<&StoredVariable> {
        for var in &self.variables {
            if var.matches(name, vendor_guid) {
                return Some(var);
            }
        }
        None
    }

    /// Get variable data
    pub fn get_data(
        &self,
        name: &[u16],
        vendor_guid: &[u8; 16],
        buffer: &mut [u8],
    ) -> Result<(usize, VariableAttributes), VariableError> {
        let var = self.get(name, vendor_guid).ok_or(VariableError::NotFound)?;

        if buffer.len() < var.data_size {
            return Err(VariableError::BufferTooSmall(var.data_size));
        }

        buffer[..var.data_size].copy_from_slice(var.data());
        Ok((var.data_size, var.attributes))
    }

    /// Set variable
    pub fn set(
        &mut self,
        name: &[u16],
        vendor_guid: &[u8; 16],
        attributes: VariableAttributes,
        data: &[u8],
    ) -> Result<(), VariableError> {
        if data.is_empty() {
            // Delete variable
            return self.delete(name, vendor_guid);
        }

        // Check for existing variable
        for var in &mut self.variables {
            if var.matches(name, vendor_guid) {
                if var.set(name, vendor_guid, attributes, data) {
                    return Ok(());
                } else {
                    return Err(VariableError::WriteError);
                }
            }
        }

        // Find free slot
        for var in &mut self.variables {
            if !var.is_in_use() {
                if var.set(name, vendor_guid, attributes, data) {
                    self.count += 1;
                    return Ok(());
                } else {
                    return Err(VariableError::WriteError);
                }
            }
        }

        Err(VariableError::OutOfResources)
    }

    /// Delete variable
    pub fn delete(&mut self, name: &[u16], vendor_guid: &[u8; 16]) -> Result<(), VariableError> {
        for var in &mut self.variables {
            if var.matches(name, vendor_guid) {
                var.clear();
                self.count -= 1;
                return Ok(());
            }
        }

        Err(VariableError::NotFound)
    }

    /// Get next variable name
    pub fn get_next(
        &self,
        current_name: &[u16],
        current_guid: &[u8; 16],
    ) -> Option<(&[u16], &[u8; 16])> {
        // If current is empty, return first variable
        if current_name.is_empty() || current_name[0] == 0 {
            for var in &self.variables {
                if var.is_in_use() {
                    return Some((var.name(), var.vendor_guid()));
                }
            }
            return None;
        }

        // Find current and return next
        let mut found_current = false;
        for var in &self.variables {
            if !var.is_in_use() {
                continue;
            }

            if found_current {
                return Some((var.name(), var.vendor_guid()));
            }

            if var.matches(current_name, current_guid) {
                found_current = true;
            }
        }

        None
    }

    /// Get variable count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get storage statistics
    pub fn statistics(&self) -> StorageStats {
        let mut total_data = 0;
        let mut total_names = 0;

        for var in &self.variables {
            if var.is_in_use() {
                total_data += var.data_size;
                total_names += var.name_len * 2; // UCS-2
            }
        }

        StorageStats {
            variable_count: self.count,
            total_data_size: total_data,
            total_name_size: total_names,
            free_slots: MAX_VARIABLE_COUNT - self.count,
        }
    }

    /// Iterate variables
    pub fn iter(&self) -> VariableIter<'_> {
        VariableIter {
            storage: self,
            index: 0,
        }
    }
}

impl Default for VariableStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage statistics
#[derive(Debug, Clone, Copy)]
pub struct StorageStats {
    pub variable_count: usize,
    pub total_data_size: usize,
    pub total_name_size: usize,
    pub free_slots: usize,
}

/// Variable iterator
pub struct VariableIter<'a> {
    storage: &'a VariableStorage,
    index: usize,
}

impl<'a> Iterator for VariableIter<'a> {
    type Item = &'a StoredVariable;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < MAX_VARIABLE_COUNT {
            let var = &self.storage.variables[self.index];
            self.index += 1;

            if var.is_in_use() {
                return Some(var);
            }
        }

        None
    }
}

// =============================================================================
// WELL-KNOWN VARIABLES
// =============================================================================

/// Well-known variable names
pub mod variable_names {
    /// Boot order
    pub const BOOT_ORDER: &str = "BootOrder";

    /// Boot current
    pub const BOOT_CURRENT: &str = "BootCurrent";

    /// Boot next
    pub const BOOT_NEXT: &str = "BootNext";

    /// Timeout
    pub const TIMEOUT: &str = "Timeout";

    /// Language
    pub const LANG: &str = "Lang";

    /// Platform language codes
    pub const PLATFORM_LANG_CODES: &str = "PlatformLangCodes";

    /// Platform language
    pub const PLATFORM_LANG: &str = "PlatformLang";

    /// Console input
    pub const CON_IN: &str = "ConIn";

    /// Console output
    pub const CON_OUT: &str = "ConOut";

    /// Error output
    pub const ERR_OUT: &str = "ErrOut";

    /// Console input device
    pub const CON_IN_DEV: &str = "ConInDev";

    /// Console output device
    pub const CON_OUT_DEV: &str = "ConOutDev";

    /// Error output device
    pub const ERR_OUT_DEV: &str = "ErrOutDev";

    /// Secure boot
    pub const SECURE_BOOT: &str = "SecureBoot";

    /// Setup mode
    pub const SETUP_MODE: &str = "SetupMode";

    /// PK (Platform Key)
    pub const PK: &str = "PK";

    /// KEK (Key Exchange Key)
    pub const KEK: &str = "KEK";

    /// db (Signature Database)
    pub const DB: &str = "db";

    /// dbx (Forbidden Signature Database)
    pub const DBX: &str = "dbx";

    /// OS indications
    pub const OS_INDICATIONS: &str = "OsIndications";

    /// OS indications supported
    pub const OS_INDICATIONS_SUPPORTED: &str = "OsIndicationsSupported";
}

/// EFI Global Variable GUID bytes
pub const EFI_GLOBAL_VARIABLE_GUID: [u8; 16] = [
    0x61, 0xDF, 0xE4, 0x8B, 0xCA, 0x93, 0xD2, 0x11, 0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,
];

/// Image Security Database GUID bytes
pub const EFI_IMAGE_SECURITY_DATABASE_GUID: [u8; 16] = [
    0xCB, 0xB2, 0x19, 0xD7, 0x3A, 0x3D, 0x96, 0x45, 0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F,
];

// =============================================================================
// VARIABLE ERROR
// =============================================================================

/// Variable operation error
#[derive(Debug, Clone)]
pub enum VariableError {
    /// Variable not found
    NotFound,
    /// Buffer too small (includes required size)
    BufferTooSmall(usize),
    /// Invalid parameter
    InvalidParameter,
    /// Write protected
    WriteProtected,
    /// Out of resources
    OutOfResources,
    /// Security violation
    SecurityViolation,
    /// Write error
    WriteError,
    /// Invalid attributes
    InvalidAttributes,
}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "variable not found"),
            Self::BufferTooSmall(size) => write!(f, "buffer too small, need {} bytes", size),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::WriteProtected => write!(f, "write protected"),
            Self::OutOfResources => write!(f, "out of resources"),
            Self::SecurityViolation => write!(f, "security violation"),
            Self::WriteError => write!(f, "write error"),
            Self::InvalidAttributes => write!(f, "invalid attributes"),
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert string to UCS-2 variable name
pub fn str_to_ucs2(s: &str, buffer: &mut [u16]) -> usize {
    let mut len = 0;

    for c in s.chars() {
        if len >= buffer.len() - 1 {
            break;
        }
        if (c as u32) <= 0xFFFF {
            buffer[len] = c as u16;
            len += 1;
        }
    }

    buffer[len] = 0;
    len
}

/// Compare UCS-2 strings (case-insensitive for ASCII)
pub fn ucs2_eq_ignore_case(a: &[u16], b: &[u16]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        let c1 = a[i];
        let c2 = b[i];

        // Case-insensitive for ASCII
        let c1_lower = if c1 >= 0x41 && c1 <= 0x5A {
            c1 + 32
        } else {
            c1
        };
        let c2_lower = if c2 >= 0x41 && c2 <= 0x5A {
            c2 + 32
        } else {
            c2
        };

        if c1_lower != c2_lower {
            return false;
        }
    }

    true
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_variable_attributes() {
        let attrs = VariableAttributes::NON_VOLATILE.or(VariableAttributes::RUNTIME_ACCESS);
        assert!(attrs.is_non_volatile());
        assert!(attrs.is_runtime_access());
        assert!(!attrs.is_bootservice_access());
    }

    #[test]
    fn test_variable_storage() {
        let mut storage = VariableStorage::new();

        let name = [b'T' as u16, b'e' as u16, b's' as u16, b't' as u16];
        let guid = [0u8; 16];
        let data = [1, 2, 3, 4];

        // Set variable
        storage
            .set(&name, &guid, VariableAttributes::BOOT_VAR, &data)
            .unwrap();
        assert_eq!(storage.count(), 1);

        // Get variable
        let mut buffer = [0u8; 16];
        let (size, attrs) = storage.get_data(&name, &guid, &mut buffer).unwrap();
        assert_eq!(size, 4);
        assert_eq!(&buffer[..4], &data);
        assert!(attrs.is_non_volatile());

        // Delete variable
        storage.delete(&name, &guid).unwrap();
        assert_eq!(storage.count(), 0);
    }

    #[test]
    fn test_str_to_ucs2() {
        let mut buffer = [0u16; 16];
        let len = str_to_ucs2("Test", &mut buffer);

        assert_eq!(len, 4);
        assert_eq!(buffer[0], b'T' as u16);
        assert_eq!(buffer[1], b'e' as u16);
        assert_eq!(buffer[2], b's' as u16);
        assert_eq!(buffer[3], b't' as u16);
        assert_eq!(buffer[4], 0);
    }
}
