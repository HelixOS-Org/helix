//! # Cognitive Transfer
//!
//! Data and knowledge transfer between cognitive domains.
//! Supports streaming, batching, and efficient serialization.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TRANSFER TYPES
// ============================================================================

/// Transfer request
#[derive(Debug, Clone)]
pub struct TransferRequest {
    /// Request ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Target domain
    pub target: DomainId,
    /// Transfer type
    pub transfer_type: TransferType,
    /// Data type
    pub data_type: DataType,
    /// Payload size (bytes)
    pub size: usize,
    /// Priority
    pub priority: TransferPriority,
    /// Created time
    pub created: Timestamp,
    /// Deadline
    pub deadline: Option<Timestamp>,
}

/// Transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    /// One-time transfer
    OneShot,
    /// Streaming transfer
    Stream,
    /// Batched transfer
    Batch,
    /// Replicated transfer
    Replicate,
    /// Synchronized transfer
    Sync,
}

/// Data type being transferred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// Raw bytes
    Binary,
    /// Knowledge graph
    Knowledge,
    /// Model weights
    Weights,
    /// Embeddings
    Embeddings,
    /// State snapshot
    State,
    /// Configuration
    Config,
    /// Metrics
    Metrics,
}

/// Transfer priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// Pending
    Pending,
    /// In progress
    InProgress,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Paused
    Paused,
}

/// Transfer chunk
#[derive(Debug, Clone)]
pub struct TransferChunk {
    /// Transfer ID
    pub transfer_id: u64,
    /// Chunk index
    pub index: u32,
    /// Total chunks
    pub total: u32,
    /// Data
    pub data: Vec<u8>,
    /// Checksum
    pub checksum: u32,
}

impl TransferChunk {
    /// Create new chunk
    pub fn new(transfer_id: u64, index: u32, total: u32, data: Vec<u8>) -> Self {
        let checksum = Self::compute_checksum(&data);
        Self {
            transfer_id,
            index,
            total,
            data,
            checksum,
        }
    }

    /// Compute checksum
    fn compute_checksum(data: &[u8]) -> u32 {
        let mut sum: u32 = 0;
        for byte in data {
            sum = sum.wrapping_add(*byte as u32);
        }
        sum
    }

    /// Verify checksum
    pub fn verify(&self) -> bool {
        self.checksum == Self::compute_checksum(&self.data)
    }
}

// ============================================================================
// TRANSFER SESSION
// ============================================================================

/// Active transfer session
#[derive(Debug)]
pub struct TransferSession {
    /// Transfer ID
    pub id: u64,
    /// Request
    pub request: TransferRequest,
    /// Status
    pub status: TransferStatus,
    /// Chunks received
    chunks_received: Vec<Option<Vec<u8>>>,
    /// Total chunks
    pub total_chunks: u32,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Started time
    pub started: Timestamp,
    /// Completed time
    pub completed: Option<Timestamp>,
    /// Error message
    pub error: Option<String>,
    /// Retry count
    pub retries: u32,
}

impl TransferSession {
    /// Create new session
    pub fn new(request: TransferRequest, chunk_size: usize) -> Self {
        let total_chunks = ((request.size + chunk_size - 1) / chunk_size) as u32;

        Self {
            id: request.id,
            request,
            status: TransferStatus::Pending,
            chunks_received: vec![None; total_chunks as usize],
            total_chunks,
            bytes_transferred: 0,
            started: Timestamp::now(),
            completed: None,
            error: None,
            retries: 0,
        }
    }

    /// Receive chunk
    pub fn receive_chunk(&mut self, chunk: TransferChunk) -> Result<(), &'static str> {
        if chunk.transfer_id != self.id {
            return Err("Wrong transfer ID");
        }

        if !chunk.verify() {
            return Err("Checksum mismatch");
        }

        if chunk.index >= self.total_chunks {
            return Err("Invalid chunk index");
        }

        let index = chunk.index as usize;
        self.bytes_transferred += chunk.data.len() as u64;
        self.chunks_received[index] = Some(chunk.data);

        // Check if complete
        if self.is_complete() {
            self.status = TransferStatus::Completed;
            self.completed = Some(Timestamp::now());
        } else {
            self.status = TransferStatus::InProgress;
        }

        Ok(())
    }

    /// Check if all chunks received
    pub fn is_complete(&self) -> bool {
        self.chunks_received.iter().all(|c| c.is_some())
    }

    /// Get missing chunk indices
    pub fn missing_chunks(&self) -> Vec<u32> {
        self.chunks_received
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_none())
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Get progress (0-1)
    pub fn progress(&self) -> f64 {
        let received = self.chunks_received.iter().filter(|c| c.is_some()).count();
        received as f64 / self.total_chunks as f64
    }

    /// Reassemble data
    pub fn reassemble(&self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }

        let mut data = Vec::with_capacity(self.request.size);
        for chunk in &self.chunks_received {
            if let Some(chunk_data) = chunk {
                data.extend_from_slice(chunk_data);
            }
        }

        Some(data)
    }
}

// ============================================================================
// TRANSFER MANAGER
// ============================================================================

/// Transfer manager
pub struct TransferManager {
    /// Active sessions
    sessions: BTreeMap<u64, TransferSession>,
    /// Pending requests
    pending: Vec<TransferRequest>,
    /// Next transfer ID
    next_id: AtomicU64,
    /// Configuration
    config: TransferConfig,
    /// Statistics
    stats: TransferStats,
}

/// Transfer configuration
#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// Chunk size
    pub chunk_size: usize,
    /// Maximum concurrent transfers
    pub max_concurrent: usize,
    /// Maximum retries
    pub max_retries: u32,
    /// Transfer timeout (ns)
    pub timeout_ns: u64,
    /// Maximum pending requests
    pub max_pending: usize,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: 65536,
            max_concurrent: 100,
            max_retries: 3,
            timeout_ns: 300_000_000_000, // 5 minutes
            max_pending: 1000,
        }
    }
}

/// Transfer statistics
#[derive(Debug, Clone, Default)]
pub struct TransferStats {
    /// Total transfers initiated
    pub total_initiated: u64,
    /// Total transfers completed
    pub total_completed: u64,
    /// Total transfers failed
    pub total_failed: u64,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Active transfers
    pub active_transfers: u64,
    /// Average transfer rate (bytes/s)
    pub avg_rate_bps: f64,
}

impl TransferManager {
    /// Create new transfer manager
    pub fn new(config: TransferConfig) -> Self {
        Self {
            sessions: BTreeMap::new(),
            pending: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: TransferStats::default(),
        }
    }

    /// Initiate a transfer
    pub fn initiate(
        &mut self,
        source: DomainId,
        target: DomainId,
        transfer_type: TransferType,
        data_type: DataType,
        size: usize,
        priority: TransferPriority,
        deadline: Option<Timestamp>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = TransferRequest {
            id,
            source,
            target,
            transfer_type,
            data_type,
            size,
            priority,
            created: Timestamp::now(),
            deadline,
        };

        // Check if we can start immediately
        if self.sessions.len() < self.config.max_concurrent {
            let session = TransferSession::new(request, self.config.chunk_size);
            self.sessions.insert(id, session);
        } else {
            self.pending.push(request);
        }

        self.stats.total_initiated += 1;
        self.stats.active_transfers = self.sessions.len() as u64;

        id
    }

    /// Create chunks from data
    pub fn create_chunks(&self, transfer_id: u64, data: &[u8]) -> Vec<TransferChunk> {
        let chunk_size = self.config.chunk_size;
        let total_chunks = ((data.len() + chunk_size - 1) / chunk_size) as u32;

        data.chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk_data)| {
                TransferChunk::new(transfer_id, i as u32, total_chunks, chunk_data.to_vec())
            })
            .collect()
    }

    /// Receive a chunk
    pub fn receive_chunk(&mut self, chunk: TransferChunk) -> Result<bool, &'static str> {
        let session = self
            .sessions
            .get_mut(&chunk.transfer_id)
            .ok_or("Unknown transfer")?;

        session.receive_chunk(chunk)?;

        if session.is_complete() {
            self.stats.total_bytes += session.bytes_transferred;
            self.stats.total_completed += 1;

            // Start pending transfer if any
            self.start_pending();

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Start next pending transfer
    fn start_pending(&mut self) {
        if self.sessions.len() >= self.config.max_concurrent {
            return;
        }

        // Sort pending by priority
        self.pending.sort_by(|a, b| b.priority.cmp(&a.priority));

        if let Some(request) = self.pending.pop() {
            let session = TransferSession::new(request, self.config.chunk_size);
            self.sessions.insert(session.id, session);
        }

        self.stats.active_transfers = self.sessions.len() as u64;
    }

    /// Get session
    pub fn get_session(&self, id: u64) -> Option<&TransferSession> {
        self.sessions.get(&id)
    }

    /// Get session data
    pub fn get_data(&self, id: u64) -> Option<Vec<u8>> {
        self.sessions.get(&id).and_then(|s| s.reassemble())
    }

    /// Cancel transfer
    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.status = TransferStatus::Cancelled;
            self.sessions.remove(&id);
            self.start_pending();
            true
        } else {
            self.pending.retain(|r| r.id != id)
        }
    }

    /// Pause transfer
    pub fn pause(&mut self, id: u64) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.status = TransferStatus::Paused;
        }
    }

    /// Resume transfer
    pub fn resume(&mut self, id: u64) {
        if let Some(session) = self.sessions.get_mut(&id) {
            if session.status == TransferStatus::Paused {
                session.status = TransferStatus::InProgress;
            }
        }
    }

    /// Retry failed chunks
    pub fn retry(&mut self, id: u64) -> Option<Vec<u32>> {
        let session = self.sessions.get_mut(&id)?;

        if session.retries >= self.config.max_retries {
            session.status = TransferStatus::Failed;
            session.error = Some("Max retries exceeded".into());
            self.stats.total_failed += 1;
            return None;
        }

        session.retries += 1;
        Some(session.missing_chunks())
    }

    /// Cleanup timed out transfers
    pub fn cleanup(&mut self) {
        let now = Timestamp::now();
        let timeout = self.config.timeout_ns;

        let timed_out: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                s.status == TransferStatus::InProgress && now.elapsed_since(s.started) > timeout
            })
            .map(|(id, _)| *id)
            .collect();

        for id in timed_out {
            if let Some(session) = self.sessions.get_mut(&id) {
                session.status = TransferStatus::Failed;
                session.error = Some("Transfer timeout".into());
            }
            self.sessions.remove(&id);
            self.stats.total_failed += 1;
        }

        // Remove completed sessions older than 1 minute
        let completed: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                s.status == TransferStatus::Completed
                    && s.completed
                        .map(|c| now.elapsed_since(c) > 60_000_000_000)
                        .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();

        for id in completed {
            self.sessions.remove(&id);
        }

        self.stats.active_transfers = self.sessions.len() as u64;
    }

    /// Get transfers for domain
    pub fn transfers_for(&self, domain: DomainId) -> Vec<&TransferSession> {
        self.sessions
            .values()
            .filter(|s| s.request.source == domain || s.request.target == domain)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &TransferStats {
        &self.stats
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new(TransferConfig::default())
    }
}

// ============================================================================
// STREAMING
// ============================================================================

/// Stream handle for continuous data transfer
pub struct StreamHandle {
    /// Stream ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Target domain
    pub target: DomainId,
    /// Buffer
    buffer: Vec<u8>,
    /// Buffer capacity
    capacity: usize,
    /// Bytes streamed
    pub bytes_streamed: u64,
    /// Active
    pub active: bool,
}

impl StreamHandle {
    /// Create new stream handle
    pub fn new(id: u64, source: DomainId, target: DomainId, capacity: usize) -> Self {
        Self {
            id,
            source,
            target,
            buffer: Vec::with_capacity(capacity),
            capacity,
            bytes_streamed: 0,
            active: true,
        }
    }

    /// Write data to stream
    pub fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        if !self.active {
            return Err("Stream closed");
        }

        let available = self.capacity - self.buffer.len();
        let to_write = data.len().min(available);

        self.buffer.extend_from_slice(&data[..to_write]);
        self.bytes_streamed += to_write as u64;

        Ok(to_write)
    }

    /// Read data from stream
    pub fn read(&mut self, max_bytes: usize) -> Vec<u8> {
        let to_read = max_bytes.min(self.buffer.len());
        self.buffer.drain(..to_read).collect()
    }

    /// Check if buffer has data
    pub fn has_data(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get buffer fill level
    pub fn fill_level(&self) -> f64 {
        self.buffer.len() as f64 / self.capacity as f64
    }

    /// Close stream
    pub fn close(&mut self) {
        self.active = false;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let manager = TransferManager::default();
        let data = vec![0u8; 200000]; // 200KB

        let chunks = manager.create_chunks(1, &data);

        // Should have multiple chunks
        assert!(chunks.len() > 1);

        // All should verify
        assert!(chunks.iter().all(|c| c.verify()));
    }

    #[test]
    fn test_transfer_session() {
        let request = TransferRequest {
            id: 1,
            source: DomainId::new(1),
            target: DomainId::new(2),
            transfer_type: TransferType::OneShot,
            data_type: DataType::Binary,
            size: 100,
            priority: TransferPriority::Normal,
            created: Timestamp::now(),
            deadline: None,
        };

        let mut session = TransferSession::new(request, 50);
        assert_eq!(session.total_chunks, 2);

        // Send chunks
        let chunk1 = TransferChunk::new(1, 0, 2, vec![0u8; 50]);
        let chunk2 = TransferChunk::new(1, 1, 2, vec![0u8; 50]);

        session.receive_chunk(chunk1).unwrap();
        assert!(!session.is_complete());

        session.receive_chunk(chunk2).unwrap();
        assert!(session.is_complete());
    }

    #[test]
    fn test_transfer_manager() {
        let mut manager = TransferManager::default();

        let id = manager.initiate(
            DomainId::new(1),
            DomainId::new(2),
            TransferType::OneShot,
            DataType::Binary,
            1000,
            TransferPriority::Normal,
            None,
        );

        assert!(manager.get_session(id).is_some());
        assert_eq!(manager.stats().total_initiated, 1);
    }

    #[test]
    fn test_stream() {
        let mut stream = StreamHandle::new(1, DomainId::new(1), DomainId::new(2), 1000);

        let written = stream.write(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(written, 5);

        let data = stream.read(3);
        assert_eq!(data, vec![1, 2, 3]);

        let data = stream.read(10);
        assert_eq!(data, vec![4, 5]);
    }
}
