//! Event Data
//!
//! Event data structures for trace events.

use alloc::vec::Vec;

use super::{EventId, TracepointId};

/// Event data
#[derive(Debug, Clone)]
pub struct EventData {
    /// Event ID
    pub event_id: EventId,
    /// Source tracepoint
    pub tracepoint_id: TracepointId,
    /// Timestamp
    pub timestamp: u64,
    /// CPU where event occurred
    pub cpu: u32,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Raw data bytes
    pub data: Vec<u8>,
}

impl EventData {
    /// Create new event data
    pub fn new(
        event_id: EventId,
        tracepoint_id: TracepointId,
        timestamp: u64,
        cpu: u32,
        pid: u64,
        tid: u64,
    ) -> Self {
        Self {
            event_id,
            tracepoint_id,
            timestamp,
            cpu,
            pid,
            tid,
            data: Vec::new(),
        }
    }

    /// Create with data
    #[inline(always)]
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Read u8 at offset
    #[inline(always)]
    pub fn read_u8(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// Read u16 at offset
    #[inline]
    pub fn read_u16(&self, offset: usize) -> Option<u16> {
        if offset + 2 <= self.data.len() {
            Some(u16::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
            ]))
        } else {
            None
        }
    }

    /// Read u32 at offset
    pub fn read_u32(&self, offset: usize) -> Option<u32> {
        if offset + 4 <= self.data.len() {
            let bytes = [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ];
            Some(u32::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Read u64 at offset
    pub fn read_u64(&self, offset: usize) -> Option<u64> {
        if offset + 8 <= self.data.len() {
            let bytes = [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
                self.data[offset + 4],
                self.data[offset + 5],
                self.data[offset + 6],
                self.data[offset + 7],
            ];
            Some(u64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Read i8 at offset
    #[inline(always)]
    pub fn read_i8(&self, offset: usize) -> Option<i8> {
        self.read_u8(offset).map(|v| v as i8)
    }

    /// Read i16 at offset
    #[inline(always)]
    pub fn read_i16(&self, offset: usize) -> Option<i16> {
        self.read_u16(offset).map(|v| v as i16)
    }

    /// Read i32 at offset
    #[inline(always)]
    pub fn read_i32(&self, offset: usize) -> Option<i32> {
        self.read_u32(offset).map(|v| v as i32)
    }

    /// Read i64 at offset
    #[inline(always)]
    pub fn read_i64(&self, offset: usize) -> Option<i64> {
        self.read_u64(offset).map(|v| v as i64)
    }

    /// Get data size
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.data.len()
    }
}
