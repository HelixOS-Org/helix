//! # Program Abstraction
//!
//! High-level program representation for Helix userspace.

use alloc::string::String;
use alloc::vec::Vec;

use super::elf::{LoadedSegment, ParsedElf};
use super::UserResult;

/// Program information
#[derive(Debug, Clone)]
pub struct ProgramInfo {
    /// Program name
    pub name: String,
    /// Entry point address
    pub entry_point: u64,
    /// Base address
    pub base_address: u64,
    /// Is position independent
    pub is_pie: bool,
    /// Total memory size
    pub memory_size: u64,
    /// Number of segments
    pub segment_count: usize,
    /// Has executable segment
    pub has_code: bool,
    /// Has writable segment
    pub has_data: bool,
}

/// A loaded program ready for execution
#[derive(Debug)]
pub struct Program {
    /// Program info
    pub info: ProgramInfo,
    /// Loaded segments
    segments: Vec<LoadedSegment>,
    /// Arguments
    args: Vec<String>,
    /// Environment variables
    env: Vec<(String, String)>,
}

impl Program {
    /// Create program from parsed ELF
    pub fn from_elf(elf: ParsedElf, name: impl Into<String>) -> UserResult<Self> {
        let name = name.into();

        let memory_size: u64 = elf.segments.iter().map(|s| s.size).sum();
        let has_code = elf.segments.iter().any(|s| s.executable);
        let has_data = elf.segments.iter().any(|s| s.writable);

        let info = ProgramInfo {
            name,
            entry_point: elf.entry_point,
            base_address: elf.base_address,
            is_pie: elf.is_pie,
            memory_size,
            segment_count: elf.segments.len(),
            has_code,
            has_data,
        };

        Ok(Self {
            info,
            segments: elf.segments,
            args: Vec::new(),
            env: Vec::new(),
        })
    }

    /// Set program arguments
    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    /// Add environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.push((key.into(), value.into()));
    }

    /// Get segments
    pub fn segments(&self) -> &[LoadedSegment] {
        &self.segments
    }

    /// Get entry point
    pub fn entry_point(&self) -> u64 {
        self.info.entry_point
    }

    /// Get memory requirements
    pub fn memory_requirements(&self) -> MemoryRequirements {
        let mut req = MemoryRequirements::default();

        for seg in &self.segments {
            if seg.executable {
                req.code_size += seg.size;
            } else if seg.writable {
                req.data_size += seg.size;
            } else {
                req.rodata_size += seg.size;
            }
        }

        // Add default stack and heap
        req.stack_size = 8 * 1024 * 1024; // 8 MB
        req.heap_size = 16 * 1024 * 1024; // 16 MB

        req
    }
}

/// Memory requirements for a program
#[derive(Debug, Clone, Default)]
pub struct MemoryRequirements {
    /// Code segment size
    pub code_size: u64,
    /// Data segment size (writable)
    pub data_size: u64,
    /// Read-only data size
    pub rodata_size: u64,
    /// Stack size
    pub stack_size: u64,
    /// Heap size
    pub heap_size: u64,
}

impl MemoryRequirements {
    /// Total memory needed
    pub fn total(&self) -> u64 {
        self.code_size + self.data_size + self.rodata_size + self.stack_size + self.heap_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_requirements() {
        let req = MemoryRequirements {
            code_size: 1024,
            data_size: 512,
            rodata_size: 256,
            stack_size: 4096,
            heap_size: 8192,
        };

        assert_eq!(req.total(), 1024 + 512 + 256 + 4096 + 8192);
    }
}
