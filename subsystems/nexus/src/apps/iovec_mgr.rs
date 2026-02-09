// SPDX-License-Identifier: GPL-2.0
//! Apps iovec_mgr — scatter-gather I/O vector management.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Single I/O vector element
#[derive(Debug, Clone, Copy)]
pub struct IoVec {
    pub base: u64,
    pub len: usize,
}

impl IoVec {
    pub fn new(base: u64, len: usize) -> Self { Self { base, len } }
    #[inline(always)]
    pub fn end(&self) -> u64 { self.base + self.len as u64 }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    #[inline(always)]
    pub fn overlaps(&self, other: &IoVec) -> bool {
        self.base < other.end() && other.base < self.end()
    }
}

/// I/O vector array (iov)
#[derive(Debug, Clone)]
pub struct IoVecArray {
    pub vecs: Vec<IoVec>,
    pub max_segs: usize,
}

impl IoVecArray {
    pub fn new(max_segs: usize) -> Self {
        Self { vecs: Vec::new(), max_segs }
    }

    #[inline]
    pub fn push(&mut self, iov: IoVec) -> bool {
        if self.vecs.len() >= self.max_segs { return false; }
        self.vecs.push(iov);
        true
    }

    #[inline(always)]
    pub fn total_bytes(&self) -> usize {
        self.vecs.iter().map(|v| v.len).sum()
    }

    #[inline(always)]
    pub fn segment_count(&self) -> usize { self.vecs.len() }

    #[inline]
    pub fn is_contiguous(&self) -> bool {
        for i in 1..self.vecs.len() {
            if self.vecs[i].base != self.vecs[i - 1].end() { return false; }
        }
        true
    }

    #[inline]
    pub fn has_overlaps(&self) -> bool {
        for i in 0..self.vecs.len() {
            for j in (i + 1)..self.vecs.len() {
                if self.vecs[i].overlaps(&self.vecs[j]) { return true; }
            }
        }
        false
    }

    pub fn coalesce(&mut self) -> usize {
        if self.vecs.len() < 2 { return 0; }
        let mut merged = 0usize;
        let mut i = 0;
        while i + 1 < self.vecs.len() {
            if self.vecs[i].end() == self.vecs[i + 1].base {
                self.vecs[i].len += self.vecs[i + 1].len;
                self.vecs.remove(i + 1);
                merged += 1;
            } else {
                i += 1;
            }
        }
        merged
    }

    pub fn truncate_to(&mut self, max_bytes: usize) {
        let mut remaining = max_bytes;
        let mut keep = 0;
        for v in &mut self.vecs {
            if remaining == 0 { break; }
            if v.len <= remaining {
                remaining -= v.len;
                keep += 1;
            } else {
                v.len = remaining;
                remaining = 0;
                keep += 1;
            }
        }
        self.vecs.truncate(keep);
    }

    #[inline]
    pub fn largest_segment(&self) -> Option<(usize, usize)> {
        self.vecs.iter().enumerate()
            .max_by_key(|(_, v)| v.len)
            .map(|(i, v)| (i, v.len))
    }
}

/// I/O operation type for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoVecOpType {
    Readv,
    Writev,
    PReadv,
    PWritev,
    SendMsg,
    RecvMsg,
    ProcessVmReadv,
    ProcessVmWritev,
    VmSplice,
}

/// Pending iovec operation
#[derive(Debug)]
pub struct IoVecOp {
    pub id: u64,
    pub op_type: IoVecOpType,
    pub pid: u32,
    pub fd: i32,
    pub array: IoVecArray,
    pub offset: Option<u64>,
    pub submitted: u64,
    pub completed_bytes: usize,
    pub is_complete: bool,
}

/// Iovec manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IoVecMgrStats {
    pub total_ops: u64,
    pub total_bytes_transferred: u64,
    pub total_segments: u64,
    pub coalesced_segments: u64,
    pub ops_by_type: ArrayMap<u64, 32>,
    pub avg_segments_per_op: f64,
}

/// Main iovec manager
pub struct AppIoVecMgr {
    pending_ops: BTreeMap<u64, IoVecOp>,
    next_id: u64,
    max_iov_segs: usize,
    total_ops: u64,
    total_bytes: u64,
    total_segments: u64,
    coalesced: u64,
    ops_by_type: ArrayMap<u64, 32>,
}

impl AppIoVecMgr {
    pub fn new(max_segs: usize) -> Self {
        Self {
            pending_ops: BTreeMap::new(), next_id: 1,
            max_iov_segs: max_segs, total_ops: 0,
            total_bytes: 0, total_segments: 0, coalesced: 0,
            ops_by_type: ArrayMap::new(0),
        }
    }

    pub fn submit(&mut self, pid: u32, fd: i32, op_type: IoVecOpType,
                   mut array: IoVecArray, offset: Option<u64>, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.total_ops += 1;
        self.total_segments += array.segment_count() as u64;
        self.ops_by_type.add(op_type as usize, 1);

        let merged = array.coalesce();
        self.coalesced += merged as u64;

        let op = IoVecOp {
            id, op_type, pid, fd, array, offset,
            submitted: now, completed_bytes: 0, is_complete: false,
        };
        self.pending_ops.insert(id, op);
        id
    }

    #[inline]
    pub fn complete(&mut self, id: u64, bytes: usize) -> bool {
        if let Some(op) = self.pending_ops.get_mut(&id) {
            op.completed_bytes = bytes;
            op.is_complete = true;
            self.total_bytes += bytes as u64;
            true
        } else { false }
    }

    pub fn remove_completed(&mut self) -> Vec<IoVecOp> {
        let completed: Vec<u64> = self.pending_ops.iter()
            .filter(|(_, op)| op.is_complete)
            .map(|(&id, _)| id)
            .collect();
        let mut result = Vec::new();
        for id in completed {
            if let Some(op) = self.pending_ops.remove(&id) {
                result.push(op);
            }
        }
        result
    }

    #[inline(always)]
    pub fn pending_count(&self) -> usize { self.pending_ops.len() }

    pub fn stats(&self) -> IoVecMgrStats {
        let avg = if self.total_ops > 0 {
            self.total_segments as f64 / self.total_ops as f64
        } else { 0.0 };
        IoVecMgrStats {
            total_ops: self.total_ops,
            total_bytes_transferred: self.total_bytes,
            total_segments: self.total_segments,
            coalesced_segments: self.coalesced,
            ops_by_type: self.ops_by_type.clone(),
            avg_segments_per_op: avg,
        }
    }
}
