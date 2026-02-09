//! Event Ring Buffer
//!
//! Ring buffer for efficient event storage.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::EventData;

/// Ring buffer for event storage
#[repr(align(64))]
pub struct EventRingBuffer {
    /// Buffer data
    data: Vec<EventData>,
    /// Buffer capacity
    capacity: usize,
    /// Write position
    write_pos: usize,
    /// Read position
    read_pos: usize,
    /// Events written
    events_written: AtomicU64,
    /// Events lost (overwritten before read)
    events_lost: AtomicU64,
    /// Is full
    is_full: bool,
}

impl EventRingBuffer {
    /// Create new ring buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
            write_pos: 0,
            read_pos: 0,
            events_written: AtomicU64::new(0),
            events_lost: AtomicU64::new(0),
            is_full: false,
        }
    }

    /// Write event to buffer
    pub fn write(&mut self, event: EventData) {
        if self.data.len() < self.capacity {
            self.data.push(event);
            self.write_pos += 1;
        } else {
            // Overwrite oldest
            if self.is_full && self.read_pos == self.write_pos {
                self.events_lost.fetch_add(1, Ordering::Relaxed);
                self.read_pos = (self.read_pos + 1) % self.capacity;
            }
            self.data[self.write_pos] = event;
            self.write_pos = (self.write_pos + 1) % self.capacity;
            self.is_full = true;
        }
        self.events_written.fetch_add(1, Ordering::Relaxed);
    }

    /// Read event from buffer
    pub fn read(&mut self) -> Option<&EventData> {
        if !self.is_full && self.read_pos >= self.write_pos {
            return None;
        }
        if self.is_full
            && self.read_pos == self.write_pos
            && self.events_written.load(Ordering::Relaxed) == 0
        {
            return None;
        }

        let event = &self.data[self.read_pos];
        self.read_pos = (self.read_pos + 1) % self.capacity;
        if self.read_pos == self.write_pos {
            self.is_full = false;
        }
        Some(event)
    }

    /// Peek at next event without consuming
    #[inline]
    pub fn peek(&self) -> Option<&EventData> {
        if !self.is_full && self.read_pos >= self.write_pos {
            return None;
        }
        Some(&self.data[self.read_pos])
    }

    /// Get events written count
    #[inline(always)]
    pub fn events_written(&self) -> u64 {
        self.events_written.load(Ordering::Relaxed)
    }

    /// Get events lost count
    #[inline(always)]
    pub fn events_lost(&self) -> u64 {
        self.events_lost.load(Ordering::Relaxed)
    }

    /// Get available events count
    #[inline]
    pub fn available(&self) -> usize {
        if self.is_full {
            self.capacity
        } else if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.capacity - self.read_pos + self.write_pos
        }
    }

    /// Check if buffer is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    /// Check if buffer is full
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.is_full
    }

    /// Clear buffer
    #[inline]
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.is_full = false;
        self.data.clear();
    }

    /// Get capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.available()
    }
}
