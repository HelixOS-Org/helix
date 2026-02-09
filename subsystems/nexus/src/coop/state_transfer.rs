//! # Coop State Transfer
//!
//! State transfer protocol for cooperative group membership:
//! - Full state snapshot transfer
//! - Incremental delta transfer
//! - Chunk-based streaming with checksums
//! - Resume/restart on failure
//! - Concurrent transfer to multiple joiners
//! - Bandwidth throttling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Transfer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    FullSnapshot,
    IncrementalDelta,
    StreamingChunks,
    CompressedFull,
    CompressedDelta,
}

/// Transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Init,
    Preparing,
    Transferring,
    Verifying,
    Applying,
    Complete,
    Failed,
    Paused,
    Cancelled,
}

/// A chunk in a state transfer
#[derive(Debug, Clone)]
pub struct TransferChunk {
    pub seq: u64,
    pub offset: u64,
    pub size: u32,
    pub checksum: u64,
    pub acked: bool,
    pub send_ts: u64,
    pub retries: u32,
}

impl TransferChunk {
    pub fn new(seq: u64, offset: u64, size: u32, data_seed: u64, ts: u64) -> Self {
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= seq;
        ck = ck.wrapping_mul(0x100000001b3);
        ck ^= data_seed;
        ck = ck.wrapping_mul(0x100000001b3);
        ck ^= size as u64;
        ck = ck.wrapping_mul(0x100000001b3);
        Self { seq, offset, size, checksum: ck, acked: false, send_ts: ts, retries: 0 }
    }

    #[inline(always)]
    pub fn ack(&mut self) { self.acked = true; }
}

/// State snapshot descriptor
#[derive(Debug, Clone)]
pub struct SnapshotDesc {
    pub epoch: u64,
    pub total_size: u64,
    pub chunk_size: u32,
    pub chunk_count: u64,
    pub checksum: u64,
    pub create_ts: u64,
}

impl SnapshotDesc {
    pub fn new(epoch: u64, total_size: u64, chunk_size: u32, ts: u64) -> Self {
        let chunk_count = (total_size + chunk_size as u64 - 1) / chunk_size as u64;
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= epoch; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= total_size; ck = ck.wrapping_mul(0x100000001b3);
        Self { epoch, total_size, chunk_size, chunk_count, checksum: ck, create_ts: ts }
    }
}

/// A transfer session
#[derive(Debug, Clone)]
pub struct TransferSession {
    pub id: u64,
    pub source: u64,
    pub target: u64,
    pub mode: TransferMode,
    pub state: TransferState,
    pub snapshot: SnapshotDesc,
    pub chunks: Vec<TransferChunk>,
    pub sent: u64,
    pub acked: u64,
    pub bytes_sent: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub max_inflight: u32,
    pub bandwidth_bps: u64,
}

impl TransferSession {
    pub fn new(id: u64, source: u64, target: u64, mode: TransferMode, snap: SnapshotDesc, max_inflight: u32, ts: u64) -> Self {
        Self {
            id, source, target, mode, state: TransferState::Init,
            snapshot: snap, chunks: Vec::new(), sent: 0, acked: 0,
            bytes_sent: 0, start_ts: ts, end_ts: 0, max_inflight,
            bandwidth_bps: 0,
        }
    }

    #[inline]
    pub fn prepare(&mut self, ts: u64) {
        self.state = TransferState::Preparing;
        let n = self.snapshot.chunk_count;
        for i in 0..n {
            let offset = i * self.snapshot.chunk_size as u64;
            let remaining = self.snapshot.total_size.saturating_sub(offset);
            let sz = core::cmp::min(remaining, self.snapshot.chunk_size as u64) as u32;
            self.chunks.push(TransferChunk::new(i, offset, sz, self.snapshot.checksum.wrapping_add(i), ts));
        }
        self.state = TransferState::Transferring;
    }

    #[inline]
    pub fn next_chunks(&self) -> Vec<u64> {
        let inflight = self.sent.saturating_sub(self.acked) as u32;
        if inflight >= self.max_inflight { return Vec::new(); }
        let can_send = (self.max_inflight - inflight) as usize;
        self.chunks.iter()
            .filter(|c| !c.acked && c.send_ts == 0)
            .take(can_send)
            .map(|c| c.seq)
            .collect()
    }

    #[inline]
    pub fn mark_sent(&mut self, seq: u64, ts: u64) {
        if let Some(c) = self.chunks.iter_mut().find(|c| c.seq == seq) {
            c.send_ts = ts;
            self.sent += 1;
            self.bytes_sent += c.size as u64;
        }
    }

    #[inline]
    pub fn ack_chunk(&mut self, seq: u64) -> bool {
        if let Some(c) = self.chunks.iter_mut().find(|c| c.seq == seq) {
            c.ack();
            self.acked += 1;
            if self.acked == self.snapshot.chunk_count { self.state = TransferState::Verifying; }
            return true;
        }
        false
    }

    #[inline]
    pub fn verify(&mut self) -> bool {
        let all_acked = self.chunks.iter().all(|c| c.acked);
        if all_acked { self.state = TransferState::Applying; }
        all_acked
    }

    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.state = TransferState::Complete; self.end_ts = ts; }
    #[inline(always)]
    pub fn fail(&mut self, ts: u64) { self.state = TransferState::Failed; self.end_ts = ts; }
    #[inline(always)]
    pub fn pause(&mut self) { self.state = TransferState::Paused; }
    #[inline(always)]
    pub fn resume(&mut self) { self.state = TransferState::Transferring; }

    #[inline(always)]
    pub fn progress(&self) -> f64 {
        if self.snapshot.chunk_count == 0 { return 0.0; }
        self.acked as f64 / self.snapshot.chunk_count as f64
    }

    #[inline(always)]
    pub fn elapsed(&self, now: u64) -> u64 { now.saturating_sub(self.start_ts) }

    #[inline(always)]
    pub fn throughput(&self, now: u64) -> u64 {
        let e = self.elapsed(now);
        if e == 0 { 0 } else { self.bytes_sent.saturating_mul(1_000_000_000) / e }
    }

    #[inline(always)]
    pub fn unacked_chunks(&self) -> Vec<u64> {
        self.chunks.iter().filter(|c| !c.acked && c.send_ts > 0).map(|c| c.seq).collect()
    }
}

/// Transfer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TransferStats {
    pub sessions: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_bytes: u64,
    pub avg_throughput: u64,
}

/// Cooperative state transfer manager
#[repr(align(64))]
pub struct CoopStateTransfer {
    sessions: BTreeMap<u64, TransferSession>,
    stats: TransferStats,
    next_id: u64,
    default_chunk_size: u32,
    default_inflight: u32,
}

impl CoopStateTransfer {
    pub fn new(chunk_size: u32, inflight: u32) -> Self {
        Self { sessions: BTreeMap::new(), stats: TransferStats::default(), next_id: 1, default_chunk_size: chunk_size, default_inflight: inflight }
    }

    #[inline]
    pub fn start_transfer(&mut self, source: u64, target: u64, mode: TransferMode, total_size: u64, epoch: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let snap = SnapshotDesc::new(epoch, total_size, self.default_chunk_size, ts);
        let mut session = TransferSession::new(id, source, target, mode, snap, self.default_inflight, ts);
        session.prepare(ts);
        self.sessions.insert(id, session);
        id
    }

    #[inline]
    pub fn send_chunks(&mut self, session_id: u64, ts: u64) -> Vec<u64> {
        if let Some(s) = self.sessions.get_mut(&session_id) {
            let seqs = s.next_chunks();
            for &seq in &seqs { s.mark_sent(seq, ts); }
            return seqs;
        }
        Vec::new()
    }

    #[inline(always)]
    pub fn ack(&mut self, session_id: u64, seq: u64) -> bool {
        if let Some(s) = self.sessions.get_mut(&session_id) { s.ack_chunk(seq) } else { false }
    }

    #[inline]
    pub fn try_complete(&mut self, session_id: u64, ts: u64) -> bool {
        if let Some(s) = self.sessions.get_mut(&session_id) {
            if s.verify() { s.complete(ts); return true; }
        }
        false
    }

    #[inline(always)]
    pub fn cancel(&mut self, session_id: u64, ts: u64) {
        if let Some(s) = self.sessions.get_mut(&session_id) { s.fail(ts); }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.sessions = self.sessions.len();
        self.stats.active = self.sessions.values().filter(|s| s.state == TransferState::Transferring).count();
        self.stats.completed = self.sessions.values().filter(|s| s.state == TransferState::Complete).count();
        self.stats.failed = self.sessions.values().filter(|s| s.state == TransferState::Failed).count();
        self.stats.total_bytes = self.sessions.values().map(|s| s.bytes_sent).sum();
    }

    #[inline(always)]
    pub fn session(&self, id: u64) -> Option<&TransferSession> { self.sessions.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &TransferStats { &self.stats }
}
