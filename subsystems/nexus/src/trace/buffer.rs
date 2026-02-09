//! Lock-free ring buffer for trace records

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::record::TraceRecord;

// ============================================================================
// RING BUFFER
// ============================================================================

/// Lock-free ring buffer for trace records
#[repr(align(64))]
pub struct TraceRingBuffer {
    /// Buffer
    buffer: Vec<TraceRecord>,
    /// Capacity
    capacity: usize,
    /// Write position
    write_pos: AtomicU64,
    /// Read position
    read_pos: AtomicU64,
    /// Records written
    written: AtomicU64,
    /// Records dropped
    dropped: AtomicU64,
}

impl TraceRingBuffer {
    /// Create a new ring buffer
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize(capacity, unsafe { core::mem::zeroed() });

        Self {
            buffer,
            capacity,
            write_pos: AtomicU64::new(0),
            read_pos: AtomicU64::new(0),
            written: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Write a record
    pub fn write(&mut self, record: TraceRecord) -> bool {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) as usize;
        let index = pos % self.capacity;

        // Check if we're overwriting unread data
        let read_pos = self.read_pos.load(Ordering::Relaxed) as usize;
        if pos >= read_pos + self.capacity {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            // Move read position forward
            self.read_pos.fetch_add(1, Ordering::Relaxed);
        }

        self.buffer[index] = record;
        self.written.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Read a record (returns None if buffer is empty)
    pub fn read(&mut self) -> Option<TraceRecord> {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Relaxed);

        if read_pos >= write_pos {
            return None;
        }

        let index = read_pos as usize % self.capacity;
        let record = self.buffer[index];
        self.read_pos.fetch_add(1, Ordering::Relaxed);
        Some(record)
    }

    /// Get number of unread records
    #[inline]
    pub fn len(&self) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);
        (write - read) as usize
    }

    /// Is buffer empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get total written
    #[inline(always)]
    pub fn total_written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    /// Get total dropped
    #[inline(always)]
    pub fn total_dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Clear the buffer
    #[inline(always)]
    pub fn clear(&mut self) {
        let write = self.write_pos.load(Ordering::Relaxed);
        self.read_pos.store(write, Ordering::Relaxed);
    }
}
