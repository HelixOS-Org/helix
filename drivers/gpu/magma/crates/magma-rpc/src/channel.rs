//! # RPC Channels
//!
//! Logical RPC channels for different subsystems.

use magma_core::{Error, Result};

use crate::message::{RpcMessage, RpcFunction, RpcResult};
use crate::queue::{CommandQueue, ResponseQueue};

// =============================================================================
// CHANNEL ID
// =============================================================================

/// RPC channel identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RpcChannelId(pub u32);

impl RpcChannelId {
    /// System channel (init, shutdown, etc.)
    pub const SYSTEM: Self = Self(0);
    /// Memory management channel
    pub const MEMORY: Self = Self(1);
    /// Engine management channel
    pub const ENGINE: Self = Self(2);
    /// Graphics channel
    pub const GRAPHICS: Self = Self(3);
    /// Compute channel
    pub const COMPUTE: Self = Self(4);
    /// Display channel
    pub const DISPLAY: Self = Self(5);
    /// Telemetry channel
    pub const TELEMETRY: Self = Self(6);
    /// Event/interrupt channel
    pub const EVENT: Self = Self(7);
}

// =============================================================================
// RPC CHANNEL
// =============================================================================

/// An RPC channel for GSP communication
#[derive(Debug)]
pub struct RpcChannel {
    /// Channel ID
    id: RpcChannelId,
    /// Command queue
    cmd_queue: CommandQueue,
    /// Response queue
    rsp_queue: ResponseQueue,
    /// Timeout for RPC calls (microseconds)
    timeout_us: u64,
}

impl RpcChannel {
    /// Default timeout (100ms)
    pub const DEFAULT_TIMEOUT_US: u64 = 100_000;

    /// Create a new RPC channel
    pub fn new(
        id: RpcChannelId,
        cmd_queue: CommandQueue,
        rsp_queue: ResponseQueue,
    ) -> Self {
        Self {
            id,
            cmd_queue,
            rsp_queue,
            timeout_us: Self::DEFAULT_TIMEOUT_US,
        }
    }

    /// Get channel ID
    pub fn id(&self) -> RpcChannelId {
        self.id
    }

    /// Set timeout
    pub fn set_timeout(&mut self, timeout_us: u64) {
        self.timeout_us = timeout_us;
    }

    /// Send a synchronous RPC request
    pub fn call(&self, function: RpcFunction, payload: alloc::vec::Vec<u8>) -> RpcResult {
        // Get sequence number
        let sequence = self.cmd_queue.next_sequence();

        // Create request message
        let request = RpcMessage::new_request(function, sequence, payload);

        // Enqueue request
        if let Err(_e) = self.cmd_queue.enqueue(&request) {
            return RpcResult::Error(crate::message::RpcStatus::OutOfMemory);
        }

        // TODO: Ring doorbell to notify GSP

        // Wait for response
        match self.rsp_queue.poll_response(sequence, self.timeout_us) {
            Ok(response) => {
                if response.header.status == 0 {
                    if response.payload.is_empty() {
                        RpcResult::Success(None)
                    } else {
                        RpcResult::Success(Some(response.payload))
                    }
                } else {
                    RpcResult::Error(crate::message::RpcStatus::from(response.header.status))
                }
            }
            Err(Error::Timeout) => RpcResult::Timeout,
            Err(_) => RpcResult::Error(crate::message::RpcStatus::Error),
        }
    }

    /// Send an async RPC request (no response expected)
    pub fn send_async(&self, function: RpcFunction, payload: alloc::vec::Vec<u8>) -> Result<()> {
        let sequence = self.cmd_queue.next_sequence();
        let mut request = RpcMessage::new_request(function, sequence, payload);
        request.header.flags |= crate::message::RpcFlags::ASYNC.bits();
        self.cmd_queue.enqueue(&request)
    }

    // =========================================================================
    // Convenience methods for common operations
    // =========================================================================

    /// System heartbeat
    pub fn heartbeat(&self) -> bool {
        matches!(
            self.call(RpcFunction::SystemHeartbeat, alloc::vec::Vec::new()),
            RpcResult::Success(_)
        )
    }

    /// Get GSP version
    pub fn get_version(&self) -> Option<alloc::vec::Vec<u8>> {
        match self.call(RpcFunction::SystemGetVersion, alloc::vec::Vec::new()) {
            RpcResult::Success(data) => data,
            _ => None,
        }
    }
}

// =============================================================================
// CHANNEL MANAGER
// =============================================================================

/// Manages multiple RPC channels
#[derive(Debug)]
pub struct ChannelManager {
    /// Active channels
    channels: alloc::vec::Vec<RpcChannel>,
}

impl ChannelManager {
    /// Create new channel manager
    pub fn new() -> Self {
        Self {
            channels: alloc::vec::Vec::new(),
        }
    }

    /// Add a channel
    pub fn add_channel(&mut self, channel: RpcChannel) {
        self.channels.push(channel);
    }

    /// Get channel by ID
    pub fn get(&self, id: RpcChannelId) -> Option<&RpcChannel> {
        self.channels.iter().find(|c| c.id == id)
    }

    /// Get mutable channel by ID
    pub fn get_mut(&mut self, id: RpcChannelId) -> Option<&mut RpcChannel> {
        self.channels.iter_mut().find(|c| c.id == id)
    }

    /// Get system channel
    pub fn system(&self) -> Option<&RpcChannel> {
        self.get(RpcChannelId::SYSTEM)
    }

    /// Get memory channel
    pub fn memory(&self) -> Option<&RpcChannel> {
        self.get(RpcChannelId::MEMORY)
    }

    /// Get graphics channel
    pub fn graphics(&self) -> Option<&RpcChannel> {
        self.get(RpcChannelId::GRAPHICS)
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
