//! # Syscall Argument Validation
//!
//! Utilities for validating syscall arguments.

use super::SyscallError;

/// Validate a user pointer
pub fn validate_user_ptr(ptr: u64, size: usize, _writable: bool) -> Result<(), SyscallError> {
    if ptr == 0 {
        return Err(SyscallError::BadAddress);
    }

    // Check alignment for common cases
    if size > 0 && ptr % core::mem::align_of::<u8>() as u64 != 0 {
        return Err(SyscallError::BadAddress);
    }

    // Check for overflow
    if ptr.checked_add(size as u64).is_none() {
        return Err(SyscallError::BadAddress);
    }

    // TODO: Check if the address is in valid user space
    // TODO: Check if the pages are mapped with appropriate permissions

    Ok(())
}

/// Validate a user string (null-terminated)
pub fn validate_user_string(ptr: u64, _max_len: usize) -> Result<usize, SyscallError> {
    validate_user_ptr(ptr, 1, false)?;

    // TODO: Actually scan the string to find its length
    // For now, just return success

    Ok(0)
}

/// Validate a file descriptor
pub fn validate_fd(fd: i32) -> Result<(), SyscallError> {
    if fd < 0 {
        return Err(SyscallError::BadFd);
    }

    // TODO: Check if the fd is valid for the current process

    Ok(())
}

/// Validate flags against allowed mask
pub fn validate_flags(flags: u64, allowed: u64) -> Result<(), SyscallError> {
    if flags & !allowed != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    Ok(())
}

/// Validate a buffer for reading
pub fn validate_read_buffer(ptr: u64, size: usize) -> Result<&'static [u8], SyscallError> {
    validate_user_ptr(ptr, size, false)?;

    // Safety: We've validated the pointer
    // In a real implementation, we'd need to ensure the memory is mapped
    unsafe { Ok(core::slice::from_raw_parts(ptr as *const u8, size)) }
}

/// Validate a buffer for writing
pub fn validate_write_buffer(ptr: u64, size: usize) -> Result<&'static mut [u8], SyscallError> {
    validate_user_ptr(ptr, size, true)?;

    // Safety: We've validated the pointer
    // In a real implementation, we'd need to ensure the memory is mapped
    unsafe { Ok(core::slice::from_raw_parts_mut(ptr as *mut u8, size)) }
}

/// Argument validator builder
pub struct ArgValidator {
    errors: alloc::vec::Vec<SyscallError>,
}

impl ArgValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            errors: alloc::vec::Vec::new(),
        }
    }

    /// Validate a pointer argument
    pub fn ptr(mut self, ptr: u64, size: usize, writable: bool) -> Self {
        if let Err(e) = validate_user_ptr(ptr, size, writable) {
            self.errors.push(e);
        }
        self
    }

    /// Validate a file descriptor
    pub fn fd(mut self, fd: i32) -> Self {
        if let Err(e) = validate_fd(fd) {
            self.errors.push(e);
        }
        self
    }

    /// Validate flags
    pub fn flags(mut self, flags: u64, allowed: u64) -> Self {
        if let Err(e) = validate_flags(flags, allowed) {
            self.errors.push(e);
        }
        self
    }

    /// Validate a range
    pub fn range<T: Ord>(mut self, value: T, min: T, max: T) -> Self {
        if value < min || value > max {
            self.errors.push(SyscallError::InvalidArgument);
        }
        self
    }

    /// Check validation result
    pub fn check(self) -> Result<(), SyscallError> {
        if let Some(e) = self.errors.into_iter().next() {
            Err(e)
        } else {
            Ok(())
        }
    }
}

impl Default for ArgValidator {
    fn default() -> Self {
        Self::new()
    }
}
