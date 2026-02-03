//! # Remote Debugging
//!
//! Network debugging for remote GPU applications.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

use crate::{CapturedFrame, InspectorError, InspectorErrorKind, InspectorResult};

/// Remote debug server
pub struct RemoteServer {
    port: u16,
    clients: Vec<ClientConnection>,
    protocol_version: u32,
    is_running: bool,
}

impl RemoteServer {
    pub fn new(port: u16) -> InspectorResult<Self> {
        Ok(Self {
            port,
            clients: Vec::new(),
            protocol_version: 1,
            is_running: false,
        })
    }

    /// Start the server
    pub fn start(&mut self) -> InspectorResult<()> {
        if self.is_running {
            return Err(InspectorError::new(
                InspectorErrorKind::RemoteError,
                "Server already running",
            ));
        }

        self.is_running = true;
        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.is_running = false;
        self.clients.clear();
    }

    /// Broadcast frame to all clients
    pub fn broadcast_frame(&mut self, frame: &CapturedFrame) -> InspectorResult<()> {
        let data = serialize_frame(frame);

        for client in &mut self.clients {
            if client.subscribed_to_frames {
                let _ = client.send(&data);
            }
        }

        Ok(())
    }

    /// Handle incoming message
    pub fn handle_message(&mut self, client_id: u64, message: &RemoteMessage) -> Option<RemoteMessage> {
        match message {
            RemoteMessage::Handshake { version } => {
                if *version == self.protocol_version {
                    Some(RemoteMessage::HandshakeAck {
                        accepted: true,
                        server_version: self.protocol_version,
                    })
                } else {
                    Some(RemoteMessage::HandshakeAck {
                        accepted: false,
                        server_version: self.protocol_version,
                    })
                }
            }
            RemoteMessage::Subscribe { frames, resources, metrics } => {
                if let Some(client) = self.clients.iter_mut().find(|c| c.id == client_id) {
                    client.subscribed_to_frames = *frames;
                    client.subscribed_to_resources = *resources;
                    client.subscribed_to_metrics = *metrics;
                }
                Some(RemoteMessage::SubscribeAck)
            }
            RemoteMessage::RequestCapture => {
                // Trigger capture
                Some(RemoteMessage::CaptureTriggered)
            }
            RemoteMessage::Ping => {
                Some(RemoteMessage::Pong)
            }
            _ => None,
        }
    }

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Is server running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

/// Client connection
struct ClientConnection {
    id: u64,
    address: String,
    subscribed_to_frames: bool,
    subscribed_to_resources: bool,
    subscribed_to_metrics: bool,
    send_buffer: Vec<u8>,
}

impl ClientConnection {
    fn send(&mut self, data: &[u8]) -> InspectorResult<()> {
        // Would actually send over network
        self.send_buffer.extend_from_slice(data);
        Ok(())
    }
}

/// Remote debug client
pub struct RemoteClient {
    server_address: String,
    port: u16,
    connected: bool,
    received_frames: Vec<CapturedFrame>,
}

impl RemoteClient {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            server_address: address.into(),
            port,
            connected: false,
            received_frames: Vec::new(),
        }
    }

    /// Connect to server
    pub fn connect(&mut self) -> InspectorResult<()> {
        // Would actually connect over network
        self.connected = true;
        Ok(())
    }

    /// Disconnect from server
    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    /// Subscribe to updates
    pub fn subscribe(&mut self, frames: bool, resources: bool, metrics: bool) -> InspectorResult<()> {
        if !self.connected {
            return Err(InspectorError::new(
                InspectorErrorKind::RemoteError,
                "Not connected",
            ));
        }

        // Would send subscribe message
        let _ = (frames, resources, metrics);
        Ok(())
    }

    /// Request capture
    pub fn request_capture(&mut self) -> InspectorResult<()> {
        if !self.connected {
            return Err(InspectorError::new(
                InspectorErrorKind::RemoteError,
                "Not connected",
            ));
        }

        // Would send capture request
        Ok(())
    }

    /// Poll for updates
    pub fn poll(&mut self) -> Vec<RemoteUpdate> {
        // Would receive from network
        Vec::new()
    }

    /// Get received frames
    pub fn frames(&self) -> &[CapturedFrame] {
        &self.received_frames
    }
}

/// Remote message protocol
#[derive(Debug, Clone)]
pub enum RemoteMessage {
    Handshake { version: u32 },
    HandshakeAck { accepted: bool, server_version: u32 },
    Subscribe { frames: bool, resources: bool, metrics: bool },
    SubscribeAck,
    RequestCapture,
    CaptureTriggered,
    FrameData { data: Vec<u8> },
    ResourceData { data: Vec<u8> },
    MetricsData { data: Vec<u8> },
    Ping,
    Pong,
    Error { code: u32, message: String },
    Disconnect,
}

/// Remote update types
#[derive(Debug, Clone)]
pub enum RemoteUpdate {
    Frame(CapturedFrame),
    Resource { id: u64, data: Vec<u8> },
    Metrics { data: Vec<u8> },
    ConnectionLost,
}

fn serialize_frame(frame: &CapturedFrame) -> Vec<u8> {
    let mut data = Vec::new();

    // Magic and version
    data.extend_from_slice(b"LFRM");
    data.extend_from_slice(&1u32.to_le_bytes());

    // Frame ID
    data.extend_from_slice(&frame.frame_id.to_le_bytes());
    data.extend_from_slice(&frame.timestamp.to_le_bytes());

    // Command buffer count
    data.extend_from_slice(&(frame.frame.command_buffers.len() as u32).to_le_bytes());

    for cb in &frame.frame.command_buffers {
        data.extend_from_slice(&cb.id.to_le_bytes());
        data.extend_from_slice(&(cb.commands.len() as u32).to_le_bytes());
    }

    data
}

/// Streaming protocol for large data
pub struct StreamingProtocol {
    chunk_size: usize,
    pending_transfers: BTreeMap<u64, PendingTransfer>,
    next_transfer_id: u64,
}

impl StreamingProtocol {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            pending_transfers: BTreeMap::new(),
            next_transfer_id: 1,
        }
    }

    /// Start a streaming transfer
    pub fn start_transfer(&mut self, data: Vec<u8>) -> u64 {
        let id = self.next_transfer_id;
        self.next_transfer_id += 1;

        let chunks = data.chunks(self.chunk_size)
            .map(|c| c.to_vec())
            .collect();

        self.pending_transfers.insert(id, PendingTransfer {
            chunks,
            current_chunk: 0,
        });

        id
    }

    /// Get next chunk to send
    pub fn next_chunk(&mut self, transfer_id: u64) -> Option<StreamChunk> {
        let transfer = self.pending_transfers.get_mut(&transfer_id)?;

        if transfer.current_chunk >= transfer.chunks.len() {
            self.pending_transfers.remove(&transfer_id);
            return None;
        }

        let chunk = StreamChunk {
            transfer_id,
            chunk_index: transfer.current_chunk as u32,
            total_chunks: transfer.chunks.len() as u32,
            data: transfer.chunks[transfer.current_chunk].clone(),
        };

        transfer.current_chunk += 1;
        Some(chunk)
    }
}

/// Pending transfer
struct PendingTransfer {
    chunks: Vec<Vec<u8>>,
    current_chunk: usize,
}

/// Stream chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub transfer_id: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub data: Vec<u8>,
}

/// Receiving assembler
pub struct StreamAssembler {
    transfers: BTreeMap<u64, AssemblingTransfer>,
}

impl StreamAssembler {
    pub fn new() -> Self {
        Self {
            transfers: BTreeMap::new(),
        }
    }

    /// Receive a chunk
    pub fn receive_chunk(&mut self, chunk: StreamChunk) -> Option<Vec<u8>> {
        let transfer = self.transfers.entry(chunk.transfer_id)
            .or_insert_with(|| AssemblingTransfer {
                chunks: vec![None; chunk.total_chunks as usize],
                received_count: 0,
                total_chunks: chunk.total_chunks,
            });

        if chunk.chunk_index as usize >= transfer.chunks.len() {
            return None;
        }

        if transfer.chunks[chunk.chunk_index as usize].is_none() {
            transfer.chunks[chunk.chunk_index as usize] = Some(chunk.data);
            transfer.received_count += 1;
        }

        if transfer.received_count == transfer.total_chunks {
            let data: Vec<u8> = transfer.chunks.iter()
                .filter_map(|c| c.as_ref())
                .flat_map(|c| c.iter())
                .copied()
                .collect();

            self.transfers.remove(&chunk.transfer_id);
            Some(data)
        } else {
            None
        }
    }
}

impl Default for StreamAssembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Assembling transfer
struct AssemblingTransfer {
    chunks: Vec<Option<Vec<u8>>>,
    received_count: u32,
    total_chunks: u32,
}

/// Compression for remote transfer
pub struct RemoteCompression;

impl RemoteCompression {
    /// Compress data for transfer
    pub fn compress(data: &[u8]) -> Vec<u8> {
        // Simple RLE compression for demonstration
        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let byte = data[i];
            let mut count = 1u8;

            while i + count as usize < data.len()
                && data[i + count as usize] == byte
                && count < 255
            {
                count += 1;
            }

            if count >= 3 {
                result.push(0xFF); // Escape
                result.push(count);
                result.push(byte);
                i += count as usize;
            } else {
                if byte == 0xFF {
                    result.push(0xFF);
                    result.push(1);
                    result.push(byte);
                } else {
                    result.push(byte);
                }
                i += 1;
            }
        }

        result
    }

    /// Decompress data
    pub fn decompress(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0xFF && i + 2 < data.len() {
                let count = data[i + 1];
                let byte = data[i + 2];
                for _ in 0..count {
                    result.push(byte);
                }
                i += 3;
            } else {
                result.push(data[i]);
                i += 1;
            }
        }

        result
    }
}
