//! Link Operations
//!
//! Hard links, symbolic links, and rename operations.

use super::MAX_SYMLINK_LOOPS;
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Link Parameters
// ============================================================================

/// Hard link parameters.
#[derive(Clone, Copy)]
pub struct LinkParams {
    /// Source inode
    pub src_ino: u64,
    /// Destination parent inode
    pub dst_parent_ino: u64,
    /// New name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
}

impl LinkParams {
    /// Create new link params
    pub fn new(src_ino: u64, dst_parent_ino: u64, name: &[u8]) -> Self {
        let mut params = Self {
            src_ino,
            dst_parent_ino,
            name: [0; 256],
            name_len: 0,
        };
        let len = core::cmp::min(name.len(), 255);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
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

        // Check for invalid names
        let name = self.name();
        if name == b"." || name == b".." {
            return Err(HfsError::InvalidArgument);
        }

        // Check for slashes
        if name.contains(&b'/') {
            return Err(HfsError::InvalidArgument);
        }

        Ok(())
    }
}

/// Unlink parameters.
#[derive(Clone, Copy)]
pub struct UnlinkParams {
    /// Parent inode
    pub parent_ino: u64,
    /// File name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
    /// Is directory (for rmdir)
    pub is_dir: bool,
}

impl UnlinkParams {
    /// Create unlink params
    pub fn new(parent_ino: u64, name: &[u8]) -> Self {
        let mut params = Self {
            parent_ino,
            name: [0; 256],
            name_len: 0,
            is_dir: false,
        };
        let len = core::cmp::min(name.len(), 255);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }

    /// Create rmdir params
    pub fn rmdir(parent_ino: u64, name: &[u8]) -> Self {
        let mut params = Self::new(parent_ino, name);
        params.is_dir = true;
        params
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

// ============================================================================
// Symlink Parameters
// ============================================================================

/// Maximum symlink target length
pub const MAX_SYMLINK_LEN: usize = 4096;

/// Symlink parameters.
#[derive(Clone, Copy)]
pub struct SymlinkParams {
    /// Parent inode
    pub parent_ino: u64,
    /// Link name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
    /// Target path
    pub target: [u8; MAX_SYMLINK_LEN],
    /// Target length
    pub target_len: u16,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
}

impl SymlinkParams {
    /// Create new symlink params
    pub fn new(parent_ino: u64, name: &[u8], target: &[u8], uid: u32, gid: u32) -> Self {
        let mut params = Self {
            parent_ino,
            name: [0; 256],
            name_len: 0,
            target: [0; MAX_SYMLINK_LEN],
            target_len: 0,
            uid,
            gid,
        };

        let name_len = core::cmp::min(name.len(), 255);
        params.name[..name_len].copy_from_slice(&name[..name_len]);
        params.name_len = name_len as u8;

        let target_len = core::cmp::min(target.len(), MAX_SYMLINK_LEN);
        params.target[..target_len].copy_from_slice(&target[..target_len]);
        params.target_len = target_len as u16;

        params
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    /// Get target
    pub fn target(&self) -> &[u8] {
        &self.target[..self.target_len as usize]
    }

    /// Validate
    pub fn validate(&self) -> HfsResult<()> {
        if self.name_len == 0 {
            return Err(HfsError::InvalidArgument);
        }
        if self.target_len == 0 {
            return Err(HfsError::InvalidArgument);
        }

        let name = self.name();
        if name == b"." || name == b".." {
            return Err(HfsError::InvalidArgument);
        }
        if name.contains(&b'/') {
            return Err(HfsError::InvalidArgument);
        }

        Ok(())
    }
}

/// Readlink result.
#[derive(Clone, Copy)]
pub struct ReadlinkResult {
    /// Target path
    pub target: [u8; MAX_SYMLINK_LEN],
    /// Target length
    pub len: u16,
}

impl ReadlinkResult {
    /// Create from target
    pub fn new(target: &[u8]) -> Self {
        let mut result = Self {
            target: [0; MAX_SYMLINK_LEN],
            len: 0,
        };
        let len = core::cmp::min(target.len(), MAX_SYMLINK_LEN);
        result.target[..len].copy_from_slice(&target[..len]);
        result.len = len as u16;
        result
    }

    /// Get target
    pub fn target(&self) -> &[u8] {
        &self.target[..self.len as usize]
    }
}

impl Default for ReadlinkResult {
    fn default() -> Self {
        Self {
            target: [0; MAX_SYMLINK_LEN],
            len: 0,
        }
    }
}

// ============================================================================
// Rename Parameters
// ============================================================================

/// Rename flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct RenameFlags(pub u32);

impl RenameFlags {
    /// Don't overwrite existing
    pub const NOREPLACE: u32 = 1 << 0;
    /// Exchange source and dest
    pub const EXCHANGE: u32 = 1 << 1;
    /// Whiteout source
    pub const WHITEOUT: u32 = 1 << 2;

    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Is exchange
    pub fn is_exchange(&self) -> bool {
        self.has(Self::EXCHANGE)
    }

    /// Is noreplace
    pub fn is_noreplace(&self) -> bool {
        self.has(Self::NOREPLACE)
    }
}

/// Rename parameters.
#[derive(Clone, Copy)]
pub struct RenameParams {
    /// Source parent inode
    pub src_parent_ino: u64,
    /// Source name
    pub src_name: [u8; 256],
    /// Source name length
    pub src_name_len: u8,
    /// Destination parent inode
    pub dst_parent_ino: u64,
    /// Destination name
    pub dst_name: [u8; 256],
    /// Destination name length
    pub dst_name_len: u8,
    /// Flags
    pub flags: RenameFlags,
}

impl RenameParams {
    /// Create new rename params
    pub fn new(src_parent_ino: u64, src_name: &[u8], dst_parent_ino: u64, dst_name: &[u8]) -> Self {
        let mut params = Self {
            src_parent_ino,
            src_name: [0; 256],
            src_name_len: 0,
            dst_parent_ino,
            dst_name: [0; 256],
            dst_name_len: 0,
            flags: RenameFlags::default(),
        };

        let src_len = core::cmp::min(src_name.len(), 255);
        params.src_name[..src_len].copy_from_slice(&src_name[..src_len]);
        params.src_name_len = src_len as u8;

        let dst_len = core::cmp::min(dst_name.len(), 255);
        params.dst_name[..dst_len].copy_from_slice(&dst_name[..dst_len]);
        params.dst_name_len = dst_len as u8;

        params
    }

    /// With flags
    pub fn with_flags(mut self, flags: RenameFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get source name
    pub fn src_name(&self) -> &[u8] {
        &self.src_name[..self.src_name_len as usize]
    }

    /// Get destination name
    pub fn dst_name(&self) -> &[u8] {
        &self.dst_name[..self.dst_name_len as usize]
    }

    /// Is same directory
    pub fn is_same_dir(&self) -> bool {
        self.src_parent_ino == self.dst_parent_ino
    }

    /// Validate
    pub fn validate(&self) -> HfsResult<()> {
        if self.src_name_len == 0 || self.dst_name_len == 0 {
            return Err(HfsError::InvalidArgument);
        }

        let src = self.src_name();
        let dst = self.dst_name();

        // Cannot rename . or ..
        if src == b"." || src == b".." || dst == b"." || dst == b".." {
            return Err(HfsError::InvalidArgument);
        }

        // Check for slashes
        if src.contains(&b'/') || dst.contains(&b'/') {
            return Err(HfsError::InvalidArgument);
        }

        // EXCHANGE and NOREPLACE are mutually exclusive
        if self.flags.is_exchange() && self.flags.is_noreplace() {
            return Err(HfsError::InvalidArgument);
        }

        Ok(())
    }
}

// ============================================================================
// Link Count Management
// ============================================================================

/// Maximum hard links
pub const MAX_LINKS: u32 = 65000;

/// Link count operations.
pub struct LinkCount {
    /// Current count
    count: u32,
}

impl LinkCount {
    /// Create new link count
    pub fn new(count: u32) -> Self {
        Self { count }
    }

    /// Get count
    pub fn get(&self) -> u32 {
        self.count
    }

    /// Increment
    pub fn inc(&mut self) -> HfsResult<u32> {
        if self.count >= MAX_LINKS {
            return Err(HfsError::TooManyLinks);
        }
        self.count += 1;
        Ok(self.count)
    }

    /// Decrement
    pub fn dec(&mut self) -> u32 {
        self.count = self.count.saturating_sub(1);
        self.count
    }

    /// Is orphaned (no links)
    pub fn is_orphaned(&self) -> bool {
        self.count == 0
    }
}

impl Default for LinkCount {
    fn default() -> Self {
        Self::new(1)
    }
}

// ============================================================================
// Symlink Resolution
// ============================================================================

/// Symlink resolution context.
#[derive(Clone, Copy)]
pub struct SymlinkContext {
    /// Current depth
    pub depth: u8,
    /// Maximum depth
    pub max_depth: u8,
    /// Total links followed
    pub total_links: u8,
    /// Maximum total links
    pub max_total: u8,
}

impl SymlinkContext {
    /// Create new context
    pub fn new() -> Self {
        Self {
            depth: 0,
            max_depth: 8,
            total_links: 0,
            max_total: MAX_SYMLINK_LOOPS as u8,
        }
    }

    /// Can follow another link
    pub fn can_follow(&self) -> bool {
        self.depth < self.max_depth && self.total_links < self.max_total
    }

    /// Enter symlink
    pub fn enter(&mut self) -> HfsResult<()> {
        if !self.can_follow() {
            return Err(HfsError::SymlinkLoop);
        }
        self.depth += 1;
        self.total_links += 1;
        Ok(())
    }

    /// Exit symlink
    pub fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}

impl Default for SymlinkContext {
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
    fn test_link_params() {
        let params = LinkParams::new(100, 2, b"newlink");
        assert_eq!(params.name(), b"newlink");
        assert!(params.validate().is_ok());

        // Invalid: contains slash
        let mut params = LinkParams::new(100, 2, b"invalid/name");
        params.name[7] = b'/';
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_symlink_params() {
        let params = SymlinkParams::new(2, b"link", b"/path/to/target", 1000, 1000);
        assert_eq!(params.name(), b"link");
        assert_eq!(params.target(), b"/path/to/target");
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_rename_params() {
        let params = RenameParams::new(2, b"old.txt", 2, b"new.txt");
        assert_eq!(params.src_name(), b"old.txt");
        assert_eq!(params.dst_name(), b"new.txt");
        assert!(params.is_same_dir());
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_rename_flags() {
        let flags = RenameFlags(RenameFlags::NOREPLACE);
        assert!(flags.is_noreplace());
        assert!(!flags.is_exchange());

        // EXCHANGE and NOREPLACE together is invalid
        let params = RenameParams::new(2, b"a", 2, b"b")
            .with_flags(RenameFlags(RenameFlags::NOREPLACE | RenameFlags::EXCHANGE));
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_link_count() {
        let mut lc = LinkCount::new(1);
        assert_eq!(lc.get(), 1);

        assert!(lc.inc().is_ok());
        assert_eq!(lc.get(), 2);

        lc.dec();
        assert_eq!(lc.get(), 1);

        lc.dec();
        assert!(lc.is_orphaned());
    }

    #[test]
    fn test_symlink_context() {
        let mut ctx = SymlinkContext::new();

        for _ in 0..8 {
            assert!(ctx.can_follow());
            ctx.enter().unwrap();
        }

        // Max depth reached
        assert!(!ctx.can_follow());
        assert!(ctx.enter().is_err());
    }
}
