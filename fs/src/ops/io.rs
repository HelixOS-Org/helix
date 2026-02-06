//! I/O Operations
//!
//! High-level read/write operations with scatter-gather support.

use super::{OpFlags, MAX_IO_SIZE};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum I/O vectors per operation
pub const MAX_IOV: usize = 16;

/// Maximum single I/O size
pub const MAX_SINGLE_IO: usize = 16 * 1024 * 1024; // 16MB

/// Optimal I/O alignment
pub const IO_ALIGNMENT: usize = 4096;

/// Minimum direct I/O size
pub const MIN_DIRECT_IO: usize = 512;

// ============================================================================
// I/O Vector
// ============================================================================

/// I/O vector entry.
#[derive(Clone, Copy, Debug, Default)]
pub struct IoVec {
    /// Buffer pointer (as usize for no_std)
    pub base: usize,
    /// Length
    pub len: usize,
}

impl IoVec {
    /// Create new I/O vector
    pub fn new(base: usize, len: usize) -> Self {
        Self { base, len }
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Is aligned
    pub fn is_aligned(&self, alignment: usize) -> bool {
        self.base % alignment == 0 && self.len % alignment == 0
    }
}

/// I/O vector array.
#[derive(Clone, Copy)]
pub struct IoVecArray {
    /// Vectors
    pub vecs: [IoVec; MAX_IOV],
    /// Count
    pub count: usize,
    /// Total length
    pub total_len: usize,
}

impl IoVecArray {
    /// Create empty array
    pub const fn new() -> Self {
        Self {
            vecs: [IoVec { base: 0, len: 0 }; MAX_IOV],
            count: 0,
            total_len: 0,
        }
    }

    /// Add vector
    pub fn push(&mut self, vec: IoVec) -> bool {
        if self.count >= MAX_IOV {
            return false;
        }
        self.vecs[self.count] = vec;
        self.count += 1;
        self.total_len += vec.len;
        true
    }

    /// Iterate vectors
    pub fn iter(&self) -> impl Iterator<Item = &IoVec> {
        self.vecs[..self.count].iter()
    }

    /// Is aligned
    pub fn is_aligned(&self, alignment: usize) -> bool {
        self.vecs[..self.count]
            .iter()
            .all(|v| v.is_aligned(alignment))
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.total_len == 0
    }
}

impl Default for IoVecArray {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Read Operation
// ============================================================================

/// Read operation parameters.
#[derive(Clone, Copy)]
pub struct ReadParams {
    /// File handle index
    pub handle: usize,
    /// Offset
    pub offset: u64,
    /// Buffer address
    pub buf: usize,
    /// Buffer length
    pub len: usize,
    /// Flags
    pub flags: OpFlags,
}

impl ReadParams {
    /// Create new read params
    pub fn new(handle: usize, offset: u64, buf: usize, len: usize) -> Self {
        Self {
            handle,
            offset,
            buf,
            len,
            flags: OpFlags::default(),
        }
    }

    /// With direct I/O
    pub fn direct(mut self) -> Self {
        self.flags.set(OpFlags::O_DIRECT);
        self
    }

    /// Validate parameters
    pub fn validate(&self) -> HfsResult<()> {
        if self.len == 0 {
            return Err(HfsError::InvalidArgument);
        }
        if self.len > MAX_SINGLE_IO {
            return Err(HfsError::TooBig);
        }
        if self.flags.has(OpFlags::O_DIRECT) {
            if self.offset as usize % IO_ALIGNMENT != 0 {
                return Err(HfsError::InvalidAlignment);
            }
            if self.len % IO_ALIGNMENT != 0 {
                return Err(HfsError::InvalidAlignment);
            }
        }
        Ok(())
    }
}

/// Readv (scatter read) parameters.
#[derive(Clone, Copy)]
pub struct ReadvParams {
    /// File handle index
    pub handle: usize,
    /// Offset
    pub offset: u64,
    /// I/O vectors
    pub iov: IoVecArray,
    /// Flags
    pub flags: OpFlags,
}

impl ReadvParams {
    /// Create new readv params
    pub fn new(handle: usize, offset: u64) -> Self {
        Self {
            handle,
            offset,
            iov: IoVecArray::new(),
            flags: OpFlags::default(),
        }
    }

    /// Add buffer
    pub fn add(&mut self, buf: usize, len: usize) -> bool {
        self.iov.push(IoVec::new(buf, len))
    }

    /// Total length
    pub fn total_len(&self) -> usize {
        self.iov.total_len
    }
}

// ============================================================================
// Write Operation
// ============================================================================

/// Write operation parameters.
#[derive(Clone, Copy)]
pub struct WriteParams {
    /// File handle index
    pub handle: usize,
    /// Offset
    pub offset: u64,
    /// Buffer address
    pub buf: usize,
    /// Buffer length
    pub len: usize,
    /// Flags
    pub flags: OpFlags,
}

impl WriteParams {
    /// Create new write params
    pub fn new(handle: usize, offset: u64, buf: usize, len: usize) -> Self {
        Self {
            handle,
            offset,
            buf,
            len,
            flags: OpFlags::default(),
        }
    }

    /// With sync
    pub fn sync(mut self) -> Self {
        self.flags.set(OpFlags::O_SYNC);
        self
    }

    /// With direct I/O
    pub fn direct(mut self) -> Self {
        self.flags.set(OpFlags::O_DIRECT);
        self
    }

    /// Validate parameters
    pub fn validate(&self) -> HfsResult<()> {
        if self.len == 0 {
            return Err(HfsError::InvalidArgument);
        }
        if self.len > MAX_SINGLE_IO {
            return Err(HfsError::TooBig);
        }
        if self.flags.has(OpFlags::O_DIRECT) {
            if self.offset as usize % IO_ALIGNMENT != 0 {
                return Err(HfsError::InvalidAlignment);
            }
            if self.len % IO_ALIGNMENT != 0 {
                return Err(HfsError::InvalidAlignment);
            }
        }
        Ok(())
    }
}

/// Writev (gather write) parameters.
#[derive(Clone, Copy)]
pub struct WritevParams {
    /// File handle index
    pub handle: usize,
    /// Offset
    pub offset: u64,
    /// I/O vectors
    pub iov: IoVecArray,
    /// Flags
    pub flags: OpFlags,
}

impl WritevParams {
    /// Create new writev params
    pub fn new(handle: usize, offset: u64) -> Self {
        Self {
            handle,
            offset,
            iov: IoVecArray::new(),
            flags: OpFlags::default(),
        }
    }

    /// Add buffer
    pub fn add(&mut self, buf: usize, len: usize) -> bool {
        self.iov.push(IoVec::new(buf, len))
    }

    /// Total length
    pub fn total_len(&self) -> usize {
        self.iov.total_len
    }

    /// With sync
    pub fn sync(mut self) -> Self {
        self.flags.set(OpFlags::O_SYNC);
        self
    }
}

// ============================================================================
// I/O Result
// ============================================================================

/// I/O operation result.
#[derive(Clone, Copy, Debug, Default)]
pub struct IoResult {
    /// Bytes transferred
    pub bytes: usize,
    /// Error (if any)
    pub error: Option<HfsError>,
    /// Was from cache
    pub from_cache: bool,
    /// Operation latency (nanoseconds)
    pub latency_ns: u64,
}

impl IoResult {
    /// Success result
    pub fn ok(bytes: usize) -> Self {
        Self {
            bytes,
            error: None,
            from_cache: false,
            latency_ns: 0,
        }
    }

    /// Error result
    pub fn err(error: HfsError) -> Self {
        Self {
            bytes: 0,
            error: Some(error),
            from_cache: false,
            latency_ns: 0,
        }
    }

    /// From cache
    pub fn cached(mut self) -> Self {
        self.from_cache = true;
        self
    }

    /// Is success
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    /// Is error
    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    /// Convert to result
    pub fn to_result(self) -> HfsResult<usize> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(self.bytes),
        }
    }
}

// ============================================================================
// Copy File Range
// ============================================================================

/// Copy file range parameters.
#[derive(Clone, Copy)]
pub struct CopyFileRangeParams {
    /// Source file handle
    pub src_handle: usize,
    /// Source offset
    pub src_offset: u64,
    /// Destination file handle
    pub dst_handle: usize,
    /// Destination offset
    pub dst_offset: u64,
    /// Length to copy
    pub len: usize,
    /// Flags
    pub flags: u32,
}

impl CopyFileRangeParams {
    /// Create new copy file range params
    pub fn new(
        src_handle: usize,
        src_offset: u64,
        dst_handle: usize,
        dst_offset: u64,
        len: usize,
    ) -> Self {
        Self {
            src_handle,
            src_offset,
            dst_handle,
            dst_offset,
            len,
            flags: 0,
        }
    }

    /// Validate
    pub fn validate(&self) -> HfsResult<()> {
        if self.len == 0 {
            return Err(HfsError::InvalidArgument);
        }
        if self.len > MAX_IO_SIZE {
            return Err(HfsError::TooBig);
        }
        Ok(())
    }
}

// ============================================================================
// Seek Operation
// ============================================================================

/// Seek whence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SeekWhence {
    /// From beginning
    Set  = 0,
    /// From current position
    Cur  = 1,
    /// From end
    End  = 2,
    /// Seek to next data
    Data = 3,
    /// Seek to next hole
    Hole = 4,
}

/// Seek parameters.
#[derive(Clone, Copy)]
pub struct SeekParams {
    /// File handle
    pub handle: usize,
    /// Offset
    pub offset: i64,
    /// Whence
    pub whence: SeekWhence,
}

impl SeekParams {
    /// Create new seek params
    pub fn new(handle: usize, offset: i64, whence: SeekWhence) -> Self {
        Self {
            handle,
            offset,
            whence,
        }
    }

    /// Calculate new position
    pub fn calculate(&self, current_pos: u64, file_size: u64) -> HfsResult<u64> {
        match self.whence {
            SeekWhence::Set => {
                if self.offset < 0 {
                    return Err(HfsError::InvalidArgument);
                }
                Ok(self.offset as u64)
            },
            SeekWhence::Cur => {
                if self.offset < 0 {
                    let abs = (-self.offset) as u64;
                    if abs > current_pos {
                        return Err(HfsError::InvalidArgument);
                    }
                    Ok(current_pos - abs)
                } else {
                    Ok(current_pos + self.offset as u64)
                }
            },
            SeekWhence::End => {
                if self.offset < 0 {
                    let abs = (-self.offset) as u64;
                    if abs > file_size {
                        return Err(HfsError::InvalidArgument);
                    }
                    Ok(file_size - abs)
                } else {
                    Ok(file_size + self.offset as u64)
                }
            },
            SeekWhence::Data | SeekWhence::Hole => {
                // Would need extent map to implement properly
                Err(HfsError::NotSupported)
            },
        }
    }
}

// ============================================================================
// Sync Operation
// ============================================================================

/// Sync scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncScope {
    /// Sync data only
    Data  = 0,
    /// Sync data and metadata
    Full  = 1,
    /// Range sync
    Range = 2,
}

/// Sync parameters.
#[derive(Clone, Copy)]
pub struct SyncParams {
    /// File handle
    pub handle: usize,
    /// Scope
    pub scope: SyncScope,
    /// Range start (for Range scope)
    pub start: u64,
    /// Range length (for Range scope)
    pub len: u64,
}

impl SyncParams {
    /// Create data sync
    pub fn data(handle: usize) -> Self {
        Self {
            handle,
            scope: SyncScope::Data,
            start: 0,
            len: 0,
        }
    }

    /// Create full sync
    pub fn full(handle: usize) -> Self {
        Self {
            handle,
            scope: SyncScope::Full,
            start: 0,
            len: 0,
        }
    }

    /// Create range sync
    pub fn range(handle: usize, start: u64, len: u64) -> Self {
        Self {
            handle,
            scope: SyncScope::Range,
            start,
            len,
        }
    }
}

// ============================================================================
// I/O Scheduler Hints
// ============================================================================

/// I/O priority class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IoPrioClass {
    /// Real-time I/O
    RealTime   = 0,
    /// Best-effort I/O (default)
    BestEffort = 1,
    /// Idle I/O
    Idle       = 2,
}

/// I/O priority.
#[derive(Clone, Copy, Debug)]
pub struct IoPrio {
    /// Priority class
    pub class: IoPrioClass,
    /// Priority level (0-7, 0 = highest)
    pub level: u8,
}

impl IoPrio {
    /// Create new priority
    pub fn new(class: IoPrioClass, level: u8) -> Self {
        Self {
            class,
            level: level.min(7),
        }
    }

    /// Real-time priority
    pub fn realtime(level: u8) -> Self {
        Self::new(IoPrioClass::RealTime, level)
    }

    /// Best-effort priority
    pub fn best_effort(level: u8) -> Self {
        Self::new(IoPrioClass::BestEffort, level)
    }

    /// Idle priority
    pub fn idle() -> Self {
        Self::new(IoPrioClass::Idle, 7)
    }
}

impl Default for IoPrio {
    fn default() -> Self {
        Self::best_effort(4)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iovec() {
        let vec = IoVec::new(0x1000, 4096);
        assert!(vec.is_aligned(4096));
        assert!(!vec.is_empty());
    }

    #[test]
    fn test_iovec_array() {
        let mut arr = IoVecArray::new();

        assert!(arr.push(IoVec::new(0x1000, 4096)));
        assert!(arr.push(IoVec::new(0x2000, 4096)));

        assert_eq!(arr.count, 2);
        assert_eq!(arr.total_len, 8192);
    }

    #[test]
    fn test_read_params_validate() {
        let params = ReadParams::new(0, 0, 0x1000, 4096);
        assert!(params.validate().is_ok());

        // Zero length
        let params = ReadParams::new(0, 0, 0x1000, 0);
        assert!(params.validate().is_err());

        // Too big
        let params = ReadParams::new(0, 0, 0x1000, MAX_SINGLE_IO + 1);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_seek_calculate() {
        let params = SeekParams::new(0, 100, SeekWhence::Set);
        assert_eq!(params.calculate(50, 1000).unwrap(), 100);

        let params = SeekParams::new(0, 50, SeekWhence::Cur);
        assert_eq!(params.calculate(100, 1000).unwrap(), 150);

        let params = SeekParams::new(0, -50, SeekWhence::Cur);
        assert_eq!(params.calculate(100, 1000).unwrap(), 50);

        let params = SeekParams::new(0, -100, SeekWhence::End);
        assert_eq!(params.calculate(0, 1000).unwrap(), 900);
    }

    #[test]
    fn test_io_result() {
        let result = IoResult::ok(1024);
        assert!(result.is_ok());
        assert_eq!(result.bytes, 1024);

        let result = IoResult::err(HfsError::IoError);
        assert!(result.is_err());
    }

    #[test]
    fn test_io_prio() {
        let prio = IoPrio::realtime(0);
        assert_eq!(prio.class, IoPrioClass::RealTime);
        assert_eq!(prio.level, 0);

        let prio = IoPrio::idle();
        assert_eq!(prio.class, IoPrioClass::Idle);
    }
}
