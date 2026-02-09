// SPDX-License-Identifier: GPL-2.0
//! Holistic ftrace_mgr — function tracing manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventType {
    FunctionEntry,
    FunctionExit,
    FunctionGraph,
    Tracepoint,
    Kprobe,
    Kretprobe,
    Uprobe,
}

/// Trace filter
#[derive(Debug, Clone)]
pub struct TraceFilter {
    pub id: u64,
    pub function_hash: u64,
    pub enabled: bool,
    pub hit_count: u64,
}

/// Trace event
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub event_type: TraceEventType,
    pub cpu: u32,
    pub pid: u64,
    pub timestamp: u64,
    pub function_addr: u64,
    pub parent_addr: u64,
    pub depth: u32,
    pub duration_ns: u64,
}

/// Per-CPU trace buffer
#[derive(Debug)]
#[repr(align(64))]
pub struct TraceBuffer {
    pub cpu: u32,
    pub events: VecDeque<TraceEvent>,
    pub capacity: usize,
    pub overruns: u64,
    pub total_events: u64,
}

impl TraceBuffer {
    pub fn new(cpu: u32, capacity: usize) -> Self {
        Self { cpu, events: VecDeque::new(), capacity, overruns: 0, total_events: 0 }
    }

    #[inline]
    pub fn write(&mut self, event: TraceEvent) {
        self.total_events += 1;
        if self.events.len() >= self.capacity { self.events.pop_front(); self.overruns += 1; }
        self.events.push_back(event);
    }

    #[inline(always)]
    pub fn drain(&mut self) -> Vec<TraceEvent> { self.events.drain(..).collect() }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { self.events.len() as f64 / self.capacity as f64 }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FtraceMgrStats {
    pub total_buffers: u32,
    pub total_events: u64,
    pub total_overruns: u64,
    pub active_filters: u32,
    pub buffer_utilization: f64,
}

/// Main ftrace manager
pub struct HolisticFtraceMgr {
    buffers: BTreeMap<u32, TraceBuffer>,
    filters: BTreeMap<u64, TraceFilter>,
    enabled: bool,
    next_filter_id: u64,
    buffer_size: usize,
}

impl HolisticFtraceMgr {
    pub fn new(buffer_size: usize) -> Self {
        Self { buffers: BTreeMap::new(), filters: BTreeMap::new(), enabled: false, next_filter_id: 1, buffer_size }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: u32) { self.buffers.insert(cpu, TraceBuffer::new(cpu, self.buffer_size)); }
    #[inline(always)]
    pub fn enable(&mut self) { self.enabled = true; }
    #[inline(always)]
    pub fn disable(&mut self) { self.enabled = false; }

    #[inline]
    pub fn add_filter(&mut self, function_hash: u64) -> u64 {
        let id = self.next_filter_id; self.next_filter_id += 1;
        self.filters.insert(id, TraceFilter { id, function_hash, enabled: true, hit_count: 0 });
        id
    }

    #[inline]
    pub fn trace(&mut self, event: TraceEvent) {
        if !self.enabled { return; }
        let cpu = event.cpu;
        if let Some(buf) = self.buffers.get_mut(&cpu) { buf.write(event); }
    }

    #[inline]
    pub fn stats(&self) -> FtraceMgrStats {
        let events: u64 = self.buffers.values().map(|b| b.total_events).sum();
        let overruns: u64 = self.buffers.values().map(|b| b.overruns).sum();
        let active = self.filters.values().filter(|f| f.enabled).count() as u32;
        let utils: Vec<f64> = self.buffers.values().map(|b| b.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        FtraceMgrStats { total_buffers: self.buffers.len() as u32, total_events: events, total_overruns: overruns, active_filters: active, buffer_utilization: avg }
    }
}
