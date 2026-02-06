//! File and Directory Handles
//!
//! Manages file descriptors and open file state.

use super::vfs::{DirHandle, FileHandle};
use super::{OpenFlags, MAX_OPEN_FILES};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum open handles per table
pub const MAX_HANDLES: usize = 65536;

/// Handle table initial size
pub const INITIAL_HANDLES: usize = 1024;

/// Invalid handle ID
pub const INVALID_HANDLE: u64 = u64::MAX;

// ============================================================================
// Handle State
// ============================================================================

/// Handle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HandleState {
    /// Free slot
    Free    = 0,
    /// Allocated and valid
    Valid   = 1,
    /// Being opened
    Opening = 2,
    /// Being closed
    Closing = 3,
}

impl Default for HandleState {
    fn default() -> Self {
        Self::Free
    }
}

// ============================================================================
// Handle Type
// ============================================================================

/// Handle type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HandleType {
    /// Regular file
    File      = 0,
    /// Directory
    Directory = 1,
    /// Special file
    Special   = 2,
}

impl Default for HandleType {
    fn default() -> Self {
        Self::File
    }
}

// ============================================================================
// Handle Entry
// ============================================================================

/// Handle entry in handle table.
#[derive(Clone, Copy, Debug)]
pub struct HandleEntry {
    /// Handle ID
    pub id: u64,
    /// Inode number
    pub ino: u64,
    /// Handle type
    pub htype: HandleType,
    /// Handle state
    pub state: HandleState,
    /// Open flags
    pub flags: OpenFlags,
    /// Current position
    pub pos: u64,
    /// Generation (for validation)
    pub generation: u32,
    /// Reference count
    pub refcount: u32,
    /// Owner process ID
    pub pid: u32,
    /// Creation timestamp
    pub open_time: u64,
    /// Last access timestamp
    pub access_time: u64,
    /// Bytes read through this handle
    pub bytes_read: u64,
    /// Bytes written through this handle
    pub bytes_written: u64,
}

impl HandleEntry {
    /// Create new handle entry
    pub fn new(id: u64, ino: u64, flags: OpenFlags) -> Self {
        Self {
            id,
            ino,
            htype: HandleType::File,
            state: HandleState::Valid,
            flags,
            pos: 0,
            generation: 0,
            refcount: 1,
            pid: 0,
            open_time: 0,
            access_time: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }

    /// Create directory handle
    pub fn new_dir(id: u64, ino: u64) -> Self {
        Self {
            id,
            ino,
            htype: HandleType::Directory,
            state: HandleState::Valid,
            flags: OpenFlags(OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY),
            pos: 0,
            generation: 0,
            refcount: 1,
            pid: 0,
            open_time: 0,
            access_time: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }

    /// Is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.state == HandleState::Valid
    }

    /// Is free
    #[inline]
    pub fn is_free(&self) -> bool {
        self.state == HandleState::Free
    }

    /// Is file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.htype == HandleType::File
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.htype == HandleType::Directory
    }

    /// Can read
    #[inline]
    pub fn can_read(&self) -> bool {
        self.flags.is_read()
    }

    /// Can write
    #[inline]
    pub fn can_write(&self) -> bool {
        self.flags.is_write()
    }

    /// Reset entry to free state
    pub fn reset(&mut self) {
        self.state = HandleState::Free;
        self.refcount = 0;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Convert to FileHandle
    pub fn to_file_handle(&self) -> FileHandle {
        FileHandle {
            id: self.id,
            ino: self.ino,
            flags: self.flags,
            pos: self.pos,
            generation: self.generation,
        }
    }

    /// Convert to DirHandle
    pub fn to_dir_handle(&self) -> DirHandle {
        DirHandle {
            id: self.id,
            ino: self.ino,
            pos: self.pos,
            generation: self.generation,
        }
    }
}

impl Default for HandleEntry {
    fn default() -> Self {
        Self {
            id: INVALID_HANDLE,
            ino: 0,
            htype: HandleType::File,
            state: HandleState::Free,
            flags: OpenFlags::empty(),
            pos: 0,
            generation: 0,
            refcount: 0,
            pid: 0,
            open_time: 0,
            access_time: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }
}

// ============================================================================
// Handle Table
// ============================================================================

/// Handle table managing open file handles.
pub struct HandleTable {
    /// Handle entries
    entries: [HandleEntry; MAX_HANDLES],
    /// Number of active handles
    count: usize,
    /// Free list head
    free_head: usize,
    /// Next handle ID
    next_id: u64,
    /// Maximum concurrent handles
    max_handles: usize,
}

impl HandleTable {
    /// Create new handle table
    pub fn new() -> Self {
        let mut table = Self {
            entries: [HandleEntry::default(); MAX_HANDLES],
            count: 0,
            free_head: 0,
            next_id: 1,
            max_handles: MAX_HANDLES,
        };

        // Initialize free list using id field temporarily
        for i in 0..MAX_HANDLES - 1 {
            table.entries[i].id = (i + 1) as u64;
        }
        table.entries[MAX_HANDLES - 1].id = INVALID_HANDLE;

        table
    }

    /// Allocate new file handle
    pub fn alloc_file(&mut self, ino: u64, flags: OpenFlags) -> HfsResult<u64> {
        if self.count >= self.max_handles {
            return Err(HfsError::NoSpace);
        }

        if self.free_head >= MAX_HANDLES {
            return Err(HfsError::NoSpace);
        }

        let slot = self.free_head;
        let next_free = self.entries[slot].id as usize;

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1; // Skip 0
        }

        self.entries[slot] = HandleEntry::new(id, ino, flags);
        self.free_head = if next_free == INVALID_HANDLE as usize {
            MAX_HANDLES
        } else {
            next_free
        };
        self.count += 1;

        Ok(id)
    }

    /// Allocate new directory handle
    pub fn alloc_dir(&mut self, ino: u64) -> HfsResult<u64> {
        if self.count >= self.max_handles {
            return Err(HfsError::NoSpace);
        }

        if self.free_head >= MAX_HANDLES {
            return Err(HfsError::NoSpace);
        }

        let slot = self.free_head;
        let next_free = self.entries[slot].id as usize;

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }

        self.entries[slot] = HandleEntry::new_dir(id, ino);
        self.free_head = if next_free == INVALID_HANDLE as usize {
            MAX_HANDLES
        } else {
            next_free
        };
        self.count += 1;

        Ok(id)
    }

    /// Find entry by ID
    fn find_slot(&self, id: u64) -> Option<usize> {
        (0..MAX_HANDLES).find(|&i| self.entries[i].id == id && self.entries[i].is_valid())
    }

    /// Get handle by ID
    pub fn get(&self, id: u64) -> Option<&HandleEntry> {
        self.find_slot(id).map(|slot| &self.entries[slot])
    }

    /// Get handle by ID (mutable)
    pub fn get_mut(&mut self, id: u64) -> Option<&mut HandleEntry> {
        self.find_slot(id).map(move |slot| &mut self.entries[slot])
    }

    /// Validate handle
    pub fn validate(&self, id: u64, generation: u32) -> HfsResult<()> {
        let entry = self.get(id).ok_or(HfsError::BadHandle)?;

        if entry.generation != generation {
            return Err(HfsError::BadHandle);
        }

        if !entry.is_valid() {
            return Err(HfsError::BadHandle);
        }

        Ok(())
    }

    /// Free handle
    pub fn free(&mut self, id: u64) -> HfsResult<()> {
        let slot = self.find_slot(id).ok_or(HfsError::BadHandle)?;

        if self.entries[slot].refcount > 1 {
            self.entries[slot].refcount -= 1;
            return Ok(());
        }

        // Add to free list
        self.entries[slot].reset();
        self.entries[slot].id = self.free_head as u64;
        self.free_head = slot;
        self.count -= 1;

        Ok(())
    }

    /// Duplicate handle (increment refcount)
    pub fn dup(&mut self, id: u64) -> HfsResult<u64> {
        let entry = self.get_mut(id).ok_or(HfsError::BadHandle)?;
        entry.refcount = entry.refcount.checked_add(1).ok_or(HfsError::Overflow)?;
        Ok(id)
    }

    /// Update position
    pub fn set_pos(&mut self, id: u64, pos: u64) -> HfsResult<()> {
        let entry = self.get_mut(id).ok_or(HfsError::BadHandle)?;
        entry.pos = pos;
        Ok(())
    }

    /// Get position
    pub fn get_pos(&self, id: u64) -> HfsResult<u64> {
        let entry = self.get(id).ok_or(HfsError::BadHandle)?;
        Ok(entry.pos)
    }

    /// Record read
    pub fn record_read(&mut self, id: u64, bytes: usize) -> HfsResult<()> {
        let entry = self.get_mut(id).ok_or(HfsError::BadHandle)?;
        entry.bytes_read = entry.bytes_read.saturating_add(bytes as u64);
        Ok(())
    }

    /// Record write
    pub fn record_write(&mut self, id: u64, bytes: usize) -> HfsResult<()> {
        let entry = self.get_mut(id).ok_or(HfsError::BadHandle)?;
        entry.bytes_written = entry.bytes_written.saturating_add(bytes as u64);
        Ok(())
    }

    /// Count active handles
    pub fn count(&self) -> usize {
        self.count
    }

    /// Count handles for inode
    pub fn count_for_inode(&self, ino: u64) -> usize {
        self.entries
            .iter()
            .filter(|e| e.is_valid() && e.ino == ino)
            .count()
    }

    /// Iterate over handles for inode
    pub fn for_inode(&self, ino: u64) -> impl Iterator<Item = &HandleEntry> {
        self.entries
            .iter()
            .filter(move |e| e.is_valid() && e.ino == ino)
    }

    /// Close all handles for inode
    pub fn close_for_inode(&mut self, ino: u64) -> usize {
        let mut closed = 0;

        for i in 0..MAX_HANDLES {
            if self.entries[i].is_valid() && self.entries[i].ino == ino {
                self.entries[i].reset();
                self.entries[i].id = self.free_head as u64;
                self.free_head = i;
                self.count -= 1;
                closed += 1;
            }
        }

        closed
    }
}

impl Default for HandleTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Per-Process Handle Table
// ============================================================================

/// Per-process file descriptor table.
pub struct FdTable {
    /// File descriptor entries (handle ID for each fd)
    fds: [u64; MAX_OPEN_FILES],
    /// Highest used fd + 1
    max_fd: usize,
    /// Number of open fds
    count: usize,
    /// Close-on-exec flags
    cloexec: [u64; MAX_OPEN_FILES / 64],
}

impl FdTable {
    /// Create new FD table
    pub fn new() -> Self {
        Self {
            fds: [INVALID_HANDLE; MAX_OPEN_FILES],
            max_fd: 0,
            count: 0,
            cloexec: [0; MAX_OPEN_FILES / 64],
        }
    }

    /// Allocate lowest available fd
    pub fn alloc(&mut self, handle_id: u64) -> HfsResult<i32> {
        for i in 0..MAX_OPEN_FILES {
            if self.fds[i] == INVALID_HANDLE {
                self.fds[i] = handle_id;
                self.count += 1;

                if i >= self.max_fd {
                    self.max_fd = i + 1;
                }

                return Ok(i as i32);
            }
        }

        Err(HfsError::TooManyOpenFiles)
    }

    /// Allocate specific fd
    pub fn alloc_at(&mut self, fd: i32, handle_id: u64) -> HfsResult<()> {
        if fd < 0 || fd as usize >= MAX_OPEN_FILES {
            return Err(HfsError::InvalidArgument);
        }

        let idx = fd as usize;

        if self.fds[idx] != INVALID_HANDLE {
            return Err(HfsError::Busy);
        }

        self.fds[idx] = handle_id;
        self.count += 1;

        if idx >= self.max_fd {
            self.max_fd = idx + 1;
        }

        Ok(())
    }

    /// Free fd
    pub fn free(&mut self, fd: i32) -> HfsResult<u64> {
        if fd < 0 || fd as usize >= MAX_OPEN_FILES {
            return Err(HfsError::InvalidArgument);
        }

        let idx = fd as usize;

        if self.fds[idx] == INVALID_HANDLE {
            return Err(HfsError::BadFd);
        }

        let handle_id = self.fds[idx];
        self.fds[idx] = INVALID_HANDLE;
        self.count -= 1;

        // Clear cloexec
        self.cloexec[idx / 64] &= !(1 << (idx % 64));

        // Update max_fd
        if idx + 1 == self.max_fd {
            while self.max_fd > 0 && self.fds[self.max_fd - 1] == INVALID_HANDLE {
                self.max_fd -= 1;
            }
        }

        Ok(handle_id)
    }

    /// Get handle ID for fd
    pub fn get(&self, fd: i32) -> HfsResult<u64> {
        if fd < 0 || fd as usize >= MAX_OPEN_FILES {
            return Err(HfsError::BadFd);
        }

        let handle_id = self.fds[fd as usize];

        if handle_id == INVALID_HANDLE {
            return Err(HfsError::BadFd);
        }

        Ok(handle_id)
    }

    /// Set cloexec flag
    pub fn set_cloexec(&mut self, fd: i32, cloexec: bool) -> HfsResult<()> {
        if fd < 0 || fd as usize >= MAX_OPEN_FILES {
            return Err(HfsError::BadFd);
        }

        let idx = fd as usize;

        if cloexec {
            self.cloexec[idx / 64] |= 1 << (idx % 64);
        } else {
            self.cloexec[idx / 64] &= !(1 << (idx % 64));
        }

        Ok(())
    }

    /// Get cloexec flag
    pub fn get_cloexec(&self, fd: i32) -> HfsResult<bool> {
        if fd < 0 || fd as usize >= MAX_OPEN_FILES {
            return Err(HfsError::BadFd);
        }

        let idx = fd as usize;
        Ok((self.cloexec[idx / 64] & (1 << (idx % 64))) != 0)
    }

    /// Duplicate fd
    pub fn dup(&mut self, oldfd: i32) -> HfsResult<i32> {
        let handle_id = self.get(oldfd)?;
        self.alloc(handle_id)
    }

    /// Duplicate fd to specific number
    pub fn dup2(&mut self, oldfd: i32, newfd: i32) -> HfsResult<i32> {
        if oldfd == newfd {
            return Ok(newfd);
        }

        let handle_id = self.get(oldfd)?;

        // Close newfd if open
        if self.fds.get(newfd as usize).copied() != Some(INVALID_HANDLE) {
            let _ = self.free(newfd);
        }

        self.alloc_at(newfd, handle_id)?;
        Ok(newfd)
    }

    /// Close all cloexec fds (on exec)
    pub fn close_cloexec(&mut self) -> usize {
        let mut closed = 0;

        for i in 0..self.max_fd {
            if (self.cloexec[i / 64] & (1 << (i % 64))) != 0 {
                if self.fds[i] != INVALID_HANDLE {
                    self.fds[i] = INVALID_HANDLE;
                    self.count -= 1;
                    closed += 1;
                }
                self.cloexec[i / 64] &= !(1 << (i % 64));
            }
        }

        closed
    }

    /// Count open fds
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get highest fd
    pub fn max_fd(&self) -> usize {
        self.max_fd
    }
}

impl Default for FdTable {
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
    fn test_handle_entry() {
        let entry = HandleEntry::new(1, 100, OpenFlags(OpenFlags::O_RDWR));

        assert!(entry.is_valid());
        assert!(entry.can_read());
        assert!(entry.can_write());
        assert!(entry.is_file());
    }

    #[test]
    fn test_handle_table() {
        let mut table = HandleTable::new();

        let id1 = table
            .alloc_file(100, OpenFlags(OpenFlags::O_RDONLY))
            .unwrap();
        let id2 = table.alloc_dir(200).unwrap();

        assert_eq!(table.count(), 2);

        let entry1 = table.get(id1).unwrap();
        assert!(entry1.is_file());

        let entry2 = table.get(id2).unwrap();
        assert!(entry2.is_dir());

        table.set_pos(id1, 1024).unwrap();
        assert_eq!(table.get_pos(id1).unwrap(), 1024);

        table.free(id1).unwrap();
        assert_eq!(table.count(), 1);
    }

    #[test]
    fn test_handle_dup() {
        let mut table = HandleTable::new();

        let id = table
            .alloc_file(100, OpenFlags(OpenFlags::O_RDONLY))
            .unwrap();

        table.dup(id).unwrap();

        let entry = table.get(id).unwrap();
        assert_eq!(entry.refcount, 2);

        // First free decrements refcount
        table.free(id).unwrap();
        let entry = table.get(id).unwrap();
        assert_eq!(entry.refcount, 1);

        // Second free actually frees
        table.free(id).unwrap();
        assert!(table.get(id).is_none());
    }

    #[test]
    fn test_fd_table() {
        let mut fdt = FdTable::new();

        let fd1 = fdt.alloc(100).unwrap();
        let fd2 = fdt.alloc(200).unwrap();

        assert_eq!(fd1, 0);
        assert_eq!(fd2, 1);
        assert_eq!(fdt.count(), 2);

        assert_eq!(fdt.get(fd1).unwrap(), 100);
        assert_eq!(fdt.get(fd2).unwrap(), 200);

        fdt.free(fd1).unwrap();
        assert_eq!(fdt.count(), 1);

        // Reuse fd1
        let fd3 = fdt.alloc(300).unwrap();
        assert_eq!(fd3, 0);
    }

    #[test]
    fn test_fd_dup() {
        let mut fdt = FdTable::new();

        let fd1 = fdt.alloc(100).unwrap();
        let fd2 = fdt.dup(fd1).unwrap();

        assert_eq!(fdt.get(fd1).unwrap(), fdt.get(fd2).unwrap());

        let fd3 = fdt.dup2(fd1, 10).unwrap();
        assert_eq!(fd3, 10);
        assert_eq!(fdt.get(10).unwrap(), 100);
    }

    #[test]
    fn test_cloexec() {
        let mut fdt = FdTable::new();

        let fd = fdt.alloc(100).unwrap();

        assert!(!fdt.get_cloexec(fd).unwrap());

        fdt.set_cloexec(fd, true).unwrap();
        assert!(fdt.get_cloexec(fd).unwrap());

        let closed = fdt.close_cloexec();
        assert_eq!(closed, 1);
        assert!(fdt.get(fd).is_err());
    }
}
