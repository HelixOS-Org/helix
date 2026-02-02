//! Extended Attribute Operations
//!
//! Xattr get, set, list, and remove operations.

use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum xattr name length
pub const MAX_XATTR_NAME: usize = 255;

/// Maximum xattr value size
pub const MAX_XATTR_VALUE: usize = 65536;

/// Maximum xattrs per inode
pub const MAX_XATTRS_PER_INODE: usize = 128;

/// Maximum total xattr size per inode
pub const MAX_XATTR_SIZE_PER_INODE: usize = 1024 * 1024; // 1MB

// ============================================================================
// Xattr Namespace
// ============================================================================

/// Extended attribute namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum XattrNamespace {
    /// User namespace (user.*)
    User     = 0,
    /// System namespace (system.*)
    System   = 1,
    /// Trusted namespace (trusted.*)
    Trusted  = 2,
    /// Security namespace (security.*)
    Security = 3,
}

impl XattrNamespace {
    /// From name prefix
    pub fn from_name(name: &[u8]) -> Option<Self> {
        if name.starts_with(b"user.") {
            Some(Self::User)
        } else if name.starts_with(b"system.") {
            Some(Self::System)
        } else if name.starts_with(b"trusted.") {
            Some(Self::Trusted)
        } else if name.starts_with(b"security.") {
            Some(Self::Security)
        } else {
            None
        }
    }

    /// Get prefix
    pub fn prefix(&self) -> &'static [u8] {
        match self {
            Self::User => b"user.",
            Self::System => b"system.",
            Self::Trusted => b"trusted.",
            Self::Security => b"security.",
        }
    }

    /// Check access permission
    pub fn can_access(&self, uid: u32, _is_write: bool) -> bool {
        match self {
            Self::User => true,
            Self::System => uid == 0,
            Self::Trusted => uid == 0,
            Self::Security => uid == 0,
        }
    }
}

impl Default for XattrNamespace {
    fn default() -> Self {
        Self::User
    }
}

// ============================================================================
// Xattr Entry
// ============================================================================

/// Extended attribute entry.
#[derive(Clone, Copy)]
pub struct XattrEntry {
    /// Attribute name
    pub name: [u8; MAX_XATTR_NAME],
    /// Name length
    pub name_len: u8,
    /// Value size
    pub value_size: u32,
    /// Value offset (in xattr block)
    pub value_offset: u32,
    /// Namespace
    pub namespace: XattrNamespace,
    /// Hash of name (for fast lookup)
    pub name_hash: u32,
    /// Used flag
    pub used: bool,
}

impl XattrEntry {
    /// Create new entry
    pub fn new(name: &[u8], namespace: XattrNamespace) -> Self {
        let mut entry = Self {
            name: [0; MAX_XATTR_NAME],
            name_len: 0,
            value_size: 0,
            value_offset: 0,
            namespace,
            name_hash: 0,
            used: true,
        };

        let len = core::cmp::min(name.len(), MAX_XATTR_NAME);
        entry.name[..len].copy_from_slice(&name[..len]);
        entry.name_len = len as u8;
        entry.name_hash = Self::hash_name(&entry.name[..len]);

        entry
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    /// Hash name
    fn hash_name(name: &[u8]) -> u32 {
        let mut hash: u32 = 0;
        for &byte in name {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }

    /// Check if name matches
    pub fn matches(&self, name: &[u8]) -> bool {
        if !self.used {
            return false;
        }

        let hash = Self::hash_name(name);
        if hash != self.name_hash {
            return false;
        }

        self.name() == name
    }

    /// Total size (name + value)
    pub fn total_size(&self) -> usize {
        self.name_len as usize + self.value_size as usize
    }
}

impl Default for XattrEntry {
    fn default() -> Self {
        Self {
            name: [0; MAX_XATTR_NAME],
            name_len: 0,
            value_size: 0,
            value_offset: 0,
            namespace: XattrNamespace::User,
            name_hash: 0,
            used: false,
        }
    }
}

// ============================================================================
// Xattr Operations
// ============================================================================

/// Setxattr flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct XattrSetFlags(pub u32);

impl XattrSetFlags {
    /// Create (fail if exists)
    pub const XATTR_CREATE: u32 = 1;
    /// Replace (fail if doesn't exist)
    pub const XATTR_REPLACE: u32 = 2;

    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Is create only
    pub fn is_create(&self) -> bool {
        self.has(Self::XATTR_CREATE)
    }

    /// Is replace only
    pub fn is_replace(&self) -> bool {
        self.has(Self::XATTR_REPLACE)
    }
}

/// Getxattr parameters.
#[derive(Clone, Copy)]
pub struct GetxattrParams {
    /// Inode
    pub ino: u64,
    /// Attribute name
    pub name: [u8; MAX_XATTR_NAME],
    /// Name length
    pub name_len: u8,
    /// Buffer size (0 = query size only)
    pub size: usize,
}

impl GetxattrParams {
    /// Create new params
    pub fn new(ino: u64, name: &[u8]) -> Self {
        let mut params = Self {
            ino,
            name: [0; MAX_XATTR_NAME],
            name_len: 0,
            size: 0,
        };
        let len = core::cmp::min(name.len(), MAX_XATTR_NAME);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }

    /// With buffer size
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    /// Get namespace
    pub fn namespace(&self) -> Option<XattrNamespace> {
        XattrNamespace::from_name(self.name())
    }
}

/// Setxattr parameters.
#[derive(Clone, Copy)]
pub struct SetxattrParams {
    /// Inode
    pub ino: u64,
    /// Attribute name
    pub name: [u8; MAX_XATTR_NAME],
    /// Name length
    pub name_len: u8,
    /// Value buffer address
    pub value: usize,
    /// Value size
    pub size: usize,
    /// Flags
    pub flags: XattrSetFlags,
}

impl SetxattrParams {
    /// Create new params
    pub fn new(ino: u64, name: &[u8], value: usize, size: usize) -> Self {
        let mut params = Self {
            ino,
            name: [0; MAX_XATTR_NAME],
            name_len: 0,
            value,
            size,
            flags: XattrSetFlags::default(),
        };
        let len = core::cmp::min(name.len(), MAX_XATTR_NAME);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }

    /// With flags
    pub fn with_flags(mut self, flags: XattrSetFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    /// Validate
    pub fn validate(&self) -> HfsResult<()> {
        if self.name_len == 0 {
            return Err(HfsError::InvalidArgument);
        }
        if self.size > MAX_XATTR_VALUE {
            return Err(HfsError::TooBig);
        }

        // Must have valid namespace
        if XattrNamespace::from_name(self.name()).is_none() {
            return Err(HfsError::NotSupported);
        }

        Ok(())
    }
}

/// Listxattr parameters.
#[derive(Clone, Copy)]
pub struct ListxattrParams {
    /// Inode
    pub ino: u64,
    /// Buffer size (0 = query size only)
    pub size: usize,
}

impl ListxattrParams {
    /// Create new params
    pub fn new(ino: u64) -> Self {
        Self { ino, size: 0 }
    }

    /// With buffer size
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }
}

/// Removexattr parameters.
#[derive(Clone, Copy)]
pub struct RemovexattrParams {
    /// Inode
    pub ino: u64,
    /// Attribute name
    pub name: [u8; MAX_XATTR_NAME],
    /// Name length
    pub name_len: u8,
}

impl RemovexattrParams {
    /// Create new params
    pub fn new(ino: u64, name: &[u8]) -> Self {
        let mut params = Self {
            ino,
            name: [0; MAX_XATTR_NAME],
            name_len: 0,
        };
        let len = core::cmp::min(name.len(), MAX_XATTR_NAME);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

// ============================================================================
// Xattr List
// ============================================================================

/// Xattr list result.
pub struct XattrList {
    /// Names (null-separated)
    pub names: [u8; 4096],
    /// Total length
    pub len: usize,
    /// Count of attributes
    pub count: usize,
}

impl XattrList {
    /// Create empty list
    pub fn new() -> Self {
        Self {
            names: [0; 4096],
            len: 0,
            count: 0,
        }
    }

    /// Add name
    pub fn add(&mut self, name: &[u8]) -> bool {
        let needed = name.len() + 1; // +1 for null terminator
        if self.len + needed > 4096 {
            return false;
        }

        self.names[self.len..self.len + name.len()].copy_from_slice(name);
        self.len += name.len();
        self.names[self.len] = 0; // null terminator
        self.len += 1;
        self.count += 1;

        true
    }

    /// Get names slice
    pub fn as_slice(&self) -> &[u8] {
        &self.names[..self.len]
    }

    /// Iterate names
    pub fn iter(&self) -> XattrListIter<'_> {
        XattrListIter {
            data: &self.names[..self.len],
            pos: 0,
        }
    }
}

impl Default for XattrList {
    fn default() -> Self {
        Self::new()
    }
}

/// Xattr list iterator.
pub struct XattrListIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for XattrListIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        // Find null terminator
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }

        let name = &self.data[start..self.pos];
        self.pos += 1; // skip null

        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }
}

// ============================================================================
// Xattr Table
// ============================================================================

/// Per-inode xattr table.
pub struct XattrTable {
    /// Entries
    entries: [XattrEntry; MAX_XATTRS_PER_INODE],
    /// Count
    count: usize,
    /// Total size
    total_size: usize,
}

impl XattrTable {
    /// Create new table
    pub const fn new() -> Self {
        Self {
            entries: [XattrEntry {
                name: [0; MAX_XATTR_NAME],
                name_len: 0,
                value_size: 0,
                value_offset: 0,
                namespace: XattrNamespace::User,
                name_hash: 0,
                used: false,
            }; MAX_XATTRS_PER_INODE],
            count: 0,
            total_size: 0,
        }
    }

    /// Find entry by name
    pub fn find(&self, name: &[u8]) -> Option<usize> {
        for i in 0..MAX_XATTRS_PER_INODE {
            if self.entries[i].matches(name) {
                return Some(i);
            }
        }
        None
    }

    /// Get entry
    pub fn get(&self, idx: usize) -> Option<&XattrEntry> {
        if idx < MAX_XATTRS_PER_INODE && self.entries[idx].used {
            Some(&self.entries[idx])
        } else {
            None
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: XattrEntry) -> HfsResult<usize> {
        // Check if exists
        if self.find(entry.name()).is_some() {
            return Err(HfsError::AlreadyExists);
        }

        // Check limits
        if self.count >= MAX_XATTRS_PER_INODE {
            return Err(HfsError::NoSpace);
        }
        if self.total_size + entry.total_size() > MAX_XATTR_SIZE_PER_INODE {
            return Err(HfsError::TooBig);
        }

        // Find free slot
        for i in 0..MAX_XATTRS_PER_INODE {
            if !self.entries[i].used {
                self.entries[i] = entry;
                self.count += 1;
                self.total_size += entry.total_size();
                return Ok(i);
            }
        }

        Err(HfsError::NoSpace)
    }

    /// Remove entry
    pub fn remove(&mut self, name: &[u8]) -> HfsResult<XattrEntry> {
        let idx = self.find(name).ok_or(HfsError::NotFound)?;

        let entry = self.entries[idx];
        self.entries[idx].used = false;
        self.count -= 1;
        self.total_size = self.total_size.saturating_sub(entry.total_size());

        Ok(entry)
    }

    /// Update entry value size
    pub fn update_size(&mut self, idx: usize, new_size: u32) -> HfsResult<()> {
        if idx >= MAX_XATTRS_PER_INODE || !self.entries[idx].used {
            return Err(HfsError::NotFound);
        }

        let old_size = self.entries[idx].value_size as usize;
        let new_total = self.total_size - old_size + new_size as usize;

        if new_total > MAX_XATTR_SIZE_PER_INODE {
            return Err(HfsError::TooBig);
        }

        self.entries[idx].value_size = new_size;
        self.total_size = new_total;

        Ok(())
    }

    /// List all names
    pub fn list(&self) -> XattrList {
        let mut list = XattrList::new();

        for entry in &self.entries {
            if entry.used {
                if !list.add(entry.name()) {
                    break;
                }
            }
        }

        list
    }

    /// Count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Total size
    pub fn total_size(&self) -> usize {
        self.total_size
    }
}

impl Default for XattrTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_namespace() {
        assert_eq!(
            XattrNamespace::from_name(b"user.test"),
            Some(XattrNamespace::User)
        );
        assert_eq!(
            XattrNamespace::from_name(b"security.selinux"),
            Some(XattrNamespace::Security)
        );
        assert_eq!(XattrNamespace::from_name(b"invalid"), None);
    }

    #[test]
    fn test_xattr_entry() {
        let entry = XattrEntry::new(b"user.test", XattrNamespace::User);
        assert_eq!(entry.name(), b"user.test");
        assert!(entry.matches(b"user.test"));
        assert!(!entry.matches(b"user.other"));
    }

    #[test]
    fn test_xattr_list() {
        let mut list = XattrList::new();

        assert!(list.add(b"user.attr1"));
        assert!(list.add(b"user.attr2"));

        let names: Vec<_> = list.iter().collect();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], b"user.attr1");
        assert_eq!(names[1], b"user.attr2");
    }

    #[test]
    fn test_xattr_table() {
        let mut table = XattrTable::new();

        let entry = XattrEntry::new(b"user.test", XattrNamespace::User);
        let idx = table.add(entry).unwrap();

        assert_eq!(table.count(), 1);
        assert!(table.find(b"user.test").is_some());

        let entry = table.get(idx).unwrap();
        assert_eq!(entry.name(), b"user.test");

        table.remove(b"user.test").unwrap();
        assert_eq!(table.count(), 0);
    }

    #[test]
    fn test_setxattr_validate() {
        let params = SetxattrParams::new(100, b"user.test", 0x1000, 256);
        assert!(params.validate().is_ok());

        // Invalid namespace
        let params = SetxattrParams::new(100, b"invalid", 0x1000, 256);
        assert!(params.validate().is_err());

        // Too big
        let params = SetxattrParams::new(100, b"user.test", 0x1000, MAX_XATTR_VALUE + 1);
        assert!(params.validate().is_err());
    }
}
