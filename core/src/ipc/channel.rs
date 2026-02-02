//! # Channels
//!
//! Low-level bounded channels for inter-module communication.
//!
//! ## Features
//!
//! - Bounded MPSC (multi-producer, single-consumer) channels
//! - Non-blocking send/receive
//! - Channel statistics

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use spin::Mutex;

use super::{IpcError, IpcResult};

// =============================================================================
// Ring Buffer
// =============================================================================

/// A simple ring buffer for channel storage
struct RingBuffer<T> {
    /// Storage
    buffer: Vec<Option<T>>,
    /// Read position
    read_pos: usize,
    /// Write position
    write_pos: usize,
    /// Number of items
    count: usize,
    /// Capacity
    capacity: usize,
}

impl<T> RingBuffer<T> {
    fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }
        Self {
            buffer,
            read_pos: 0,
            write_pos: 0,
            count: 0,
            capacity,
        }
    }

    fn push(&mut self, item: T) -> Result<(), T> {
        if self.count >= self.capacity {
            return Err(item);
        }

        self.buffer[self.write_pos] = Some(item);
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.count += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        let item = self.buffer[self.read_pos].take();
        self.read_pos = (self.read_pos + 1) % self.capacity;
        self.count -= 1;
        item
    }

    fn len(&self) -> usize {
        self.count
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn is_full(&self) -> bool {
        self.count >= self.capacity
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

// =============================================================================
// Channel Inner
// =============================================================================

/// Shared channel state
struct ChannelInner<T> {
    /// The buffer
    buffer: Mutex<RingBuffer<T>>,
    /// Is the channel closed?
    closed: AtomicBool,
    /// Number of senders
    sender_count: AtomicUsize,
    /// Messages sent
    sent: AtomicUsize,
    /// Messages received
    received: AtomicUsize,
}

impl<T> ChannelInner<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Mutex::new(RingBuffer::new(capacity)),
            closed: AtomicBool::new(false),
            sender_count: AtomicUsize::new(1),
            sent: AtomicUsize::new(0),
            received: AtomicUsize::new(0),
        }
    }

    fn send(&self, item: T) -> IpcResult<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(IpcError::ChannelClosed);
        }

        let mut buffer = self.buffer.lock();
        match buffer.push(item) {
            Ok(()) => {
                self.sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            },
            Err(_) => Err(IpcError::ChannelFull),
        }
    }

    fn try_recv(&self) -> IpcResult<T> {
        let mut buffer = self.buffer.lock();

        match buffer.pop() {
            Some(item) => {
                self.received.fetch_add(1, Ordering::Relaxed);
                Ok(item)
            },
            None => {
                if self.closed.load(Ordering::Acquire)
                    && self.sender_count.load(Ordering::Acquire) == 0
                {
                    Err(IpcError::ChannelClosed)
                } else {
                    Err(IpcError::Timeout) // No data available
                }
            },
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    fn len(&self) -> usize {
        self.buffer.lock().len()
    }

    fn capacity(&self) -> usize {
        self.buffer.lock().capacity()
    }
}

// =============================================================================
// Sender
// =============================================================================

/// Sending half of a channel
pub struct Sender<T> {
    inner: Arc<ChannelInner<T>>,
}

impl<T> Sender<T> {
    /// Send an item through the channel
    pub fn send(&self, item: T) -> IpcResult<()> {
        self.inner.send(item)
    }

    /// Try to send, returns immediately
    pub fn try_send(&self, item: T) -> IpcResult<()> {
        self.inner.send(item)
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Get number of items in channel
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if channel is empty
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Get channel capacity
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.sender_count.fetch_add(1, Ordering::AcqRel);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let prev = self.inner.sender_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // Last sender, close the channel
            self.inner.close();
        }
    }
}

// =============================================================================
// Receiver
// =============================================================================

/// Receiving half of a channel
pub struct Receiver<T> {
    inner: Arc<ChannelInner<T>>,
}

impl<T> Receiver<T> {
    /// Try to receive an item (non-blocking)
    pub fn try_recv(&self) -> IpcResult<T> {
        self.inner.try_recv()
    }

    /// Receive all available items
    pub fn drain(&self) -> Vec<T> {
        let mut items = Vec::new();
        while let Ok(item) = self.try_recv() {
            items.push(item);
        }
        items
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Get number of items in channel
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if channel is empty
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Get channel capacity
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Close the channel
    pub fn close(&self) {
        self.inner.close();
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.close();
    }
}

// =============================================================================
// Channel Creation
// =============================================================================

/// Create a bounded channel with the specified capacity
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(ChannelInner::new(capacity));
    (
        Sender {
            inner: Arc::clone(&inner),
        },
        Receiver { inner },
    )
}

/// Create a channel with default capacity (32 items)
pub fn default_channel<T>() -> (Sender<T>, Receiver<T>) {
    channel(32)
}

// =============================================================================
// One-shot Channel
// =============================================================================

/// A one-shot channel that can only send a single value
pub struct OneShotSender<T> {
    inner: Arc<Mutex<Option<T>>>,
    sent: Arc<AtomicBool>,
}

impl<T> OneShotSender<T> {
    /// Send a value (can only be called once)
    pub fn send(self, value: T) -> IpcResult<()> {
        if self.sent.swap(true, Ordering::AcqRel) {
            return Err(IpcError::ChannelClosed);
        }

        let mut guard = self.inner.lock();
        *guard = Some(value);
        Ok(())
    }
}

/// A one-shot channel receiver
pub struct OneShotReceiver<T> {
    inner: Arc<Mutex<Option<T>>>,
    sent: Arc<AtomicBool>,
}

impl<T> OneShotReceiver<T> {
    /// Try to receive the value
    pub fn try_recv(&self) -> IpcResult<T> {
        if !self.sent.load(Ordering::Acquire) {
            return Err(IpcError::Timeout); // Not sent yet
        }

        let mut guard = self.inner.lock();
        guard.take().ok_or(IpcError::ChannelClosed)
    }

    /// Check if value is ready
    pub fn is_ready(&self) -> bool {
        self.sent.load(Ordering::Acquire)
    }
}

/// Create a one-shot channel
pub fn oneshot<T>() -> (OneShotSender<T>, OneShotReceiver<T>) {
    let inner = Arc::new(Mutex::new(None));
    let sent = Arc::new(AtomicBool::new(false));
    (
        OneShotSender {
            inner: Arc::clone(&inner),
            sent: Arc::clone(&sent),
        },
        OneShotReceiver { inner, sent },
    )
}

// =============================================================================
// Channel Statistics
// =============================================================================

/// Channel statistics
#[derive(Debug, Clone, Default)]
pub struct ChannelStats {
    /// Messages sent
    pub sent: usize,
    /// Messages received
    pub received: usize,
    /// Current queue length
    pub queue_length: usize,
    /// Capacity
    pub capacity: usize,
}

impl<T> Sender<T> {
    /// Get channel statistics
    pub fn stats(&self) -> ChannelStats {
        ChannelStats {
            sent: self.inner.sent.load(Ordering::Relaxed),
            received: self.inner.received.load(Ordering::Relaxed),
            queue_length: self.inner.len(),
            capacity: self.inner.capacity(),
        }
    }
}

impl<T> Receiver<T> {
    /// Get channel statistics
    pub fn stats(&self) -> ChannelStats {
        ChannelStats {
            sent: self.inner.sent.load(Ordering::Relaxed),
            received: self.inner.received.load(Ordering::Relaxed),
            queue_length: self.inner.len(),
            capacity: self.inner.capacity(),
        }
    }
}
