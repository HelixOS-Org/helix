//! # RPC Transport Layer
//!
//! Low-level transport for RPC communication with GSP.

use magma_core::{ByteSize, Error, GpuAddr, Result};
use magma_hal::bar::BarRegion;
use magma_hal::mmio::MmioRegion;

use crate::channel::{ChannelManager, RpcChannel, RpcChannelId};
use crate::gsp::{GspBootParams, GspInfo, GspInitProgress, GspState};
use crate::queue::{CommandQueue, ResponseQueue};

// =============================================================================
// TRANSPORT CONFIGURATION
// =============================================================================

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Command queue size
    pub cmd_queue_size: ByteSize,
    /// Response queue size
    pub rsp_queue_size: ByteSize,
    /// Entry size
    pub entry_size: u32,
    /// Number of channels
    pub num_channels: u32,
    /// RPC timeout (microseconds)
    pub timeout_us: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            cmd_queue_size: ByteSize::from_kib(64),
            rsp_queue_size: ByteSize::from_kib(64),
            entry_size: 4096,
            num_channels: 8,
            timeout_us: 100_000, // 100ms
        }
    }
}

// =============================================================================
// DOORBELL
// =============================================================================

/// Doorbell register for notifying GSP
#[derive(Debug)]
pub struct Doorbell {
    /// MMIO offset
    offset: u32,
    /// Last value written
    last_value: u32,
}

impl Doorbell {
    /// Create new doorbell
    pub const fn new(offset: u32) -> Self {
        Self {
            offset,
            last_value: 0,
        }
    }

    /// Ring the doorbell
    ///
    /// # Safety
    /// mmio must be valid
    pub unsafe fn ring(&mut self, mmio: &mut MmioRegion, value: u32) {
        // SAFETY: Caller guarantees mmio validity
        unsafe {
            mmio.write32(self.offset, value);
        }
        self.last_value = value;
    }

    /// Get doorbell offset
    pub fn offset(&self) -> u32 {
        self.offset
    }
}

// =============================================================================
// SHARED MEMORY REGION
// =============================================================================

/// Shared memory region for RPC
#[derive(Debug)]
pub struct SharedMemory {
    /// GPU address
    pub gpu_addr: GpuAddr,
    /// CPU virtual address
    pub cpu_addr: *mut u8,
    /// Size
    pub size: ByteSize,
}

impl SharedMemory {
    /// Create new shared memory region
    ///
    /// # Safety
    /// - gpu_addr must be valid
    /// - cpu_addr must be a valid pointer to `size` bytes
    pub unsafe fn new(gpu_addr: GpuAddr, cpu_addr: *mut u8, size: ByteSize) -> Self {
        Self {
            gpu_addr,
            cpu_addr,
            size,
        }
    }

    /// Get subregion
    ///
    /// # Safety
    /// offset + sub_size must not exceed size
    pub unsafe fn subregion(&self, offset: usize, sub_size: ByteSize) -> Option<Self> {
        if offset as u64 + sub_size.as_bytes() > self.size.as_bytes() {
            return None;
        }

        Some(Self {
            gpu_addr: self.gpu_addr + offset as u64,
            // SAFETY: Bounds checked above
            cpu_addr: unsafe { self.cpu_addr.add(offset) },
            size: sub_size,
        })
    }
}

// SAFETY: SharedMemory is designed for controlled concurrent access
unsafe impl Send for SharedMemory {}

// =============================================================================
// TRANSPORT
// =============================================================================

/// RPC transport layer
#[derive(Debug)]
pub struct Transport {
    /// Configuration
    config: TransportConfig,
    /// MMIO region for doorbell
    mmio: MmioRegion,
    /// Shared memory for queues
    shared_mem: SharedMemory,
    /// Doorbell
    doorbell: Doorbell,
    /// Channel manager
    channels: ChannelManager,
    /// GSP state
    gsp_state: GspState,
    /// GSP info (populated after init)
    gsp_info: Option<GspInfo>,
}

impl Transport {
    /// GSP doorbell register offset
    const DOORBELL_OFFSET: u32 = 0x2000;

    /// Create new transport
    ///
    /// # Safety
    /// - bar must be a valid mapped BAR region
    /// - shared_mem must be valid DMA memory
    pub unsafe fn new(
        config: TransportConfig,
        bar: BarRegion,
        shared_mem: SharedMemory,
    ) -> Result<Self> {
        let mmio = MmioRegion::from_bar(bar);
        let doorbell = Doorbell::new(Self::DOORBELL_OFFSET);

        Ok(Self {
            config,
            mmio,
            shared_mem,
            doorbell,
            channels: ChannelManager::new(),
            gsp_state: GspState::Uninitialized,
            gsp_info: None,
        })
    }

    /// Get current GSP state
    pub fn gsp_state(&self) -> GspState {
        self.gsp_state
    }

    /// Get GSP info (if initialized)
    pub fn gsp_info(&self) -> Option<&GspInfo> {
        self.gsp_info.as_ref()
    }

    /// Get channel manager
    pub fn channels(&self) -> &ChannelManager {
        &self.channels
    }

    /// Get mutable channel manager
    pub fn channels_mut(&mut self) -> &mut ChannelManager {
        &mut self.channels
    }

    /// Initialize transport and GSP
    pub fn init(&mut self, _boot_params: &GspBootParams) -> Result<GspInitProgress> {
        let mut progress = GspInitProgress::new();

        self.gsp_state = GspState::Loading;

        // Step 1: Validate firmware
        // (In real implementation, verify signature, etc.)
        progress.next();

        // Step 2: Setup WPR
        // (Configure write-protected region)
        progress.next();

        // Step 3-4: Load firmware
        self.gsp_state = GspState::Booting;
        progress.next();
        progress.next();

        // Step 5-6: Configure and start Falcon
        progress.next();
        progress.next();

        // Step 7: Wait for boot
        progress.next();

        // Step 8: Setup RPC channels
        self.setup_channels()?;
        progress.next();

        // Step 9: Verify ready
        self.gsp_state = GspState::Ready;
        progress.next();

        // Complete
        progress.next();

        Ok(progress)
    }

    /// Setup RPC channels
    fn setup_channels(&mut self) -> Result<()> {
        let queue_size = self.config.cmd_queue_size;
        let entry_size = self.config.entry_size;

        // Calculate offsets
        let channel_size = queue_size.as_bytes() * 2; // cmd + rsp

        for i in 0..self.config.num_channels {
            let offset = i as usize * channel_size as usize;

            // SAFETY: bounds checked against shared_mem size
            let cmd_region = unsafe { self.shared_mem.subregion(offset, queue_size) }
                .ok_or(Error::OutOfMemory)?;

            let rsp_region = unsafe {
                self.shared_mem
                    .subregion(offset + queue_size.as_bytes() as usize, queue_size)
            }
            .ok_or(Error::OutOfMemory)?;

            // Create queues
            // SAFETY: Regions are from valid shared memory
            let cmd_queue = unsafe {
                CommandQueue::new(
                    cmd_region.gpu_addr,
                    cmd_region.cpu_addr,
                    queue_size,
                    entry_size,
                )?
            };

            let rsp_queue = unsafe {
                ResponseQueue::new(
                    rsp_region.gpu_addr,
                    rsp_region.cpu_addr,
                    queue_size,
                    entry_size,
                )?
            };

            // Create channel
            let channel_id = RpcChannelId(i);
            let channel = RpcChannel::new(channel_id, cmd_queue, rsp_queue);

            self.channels.add_channel(channel);
        }

        Ok(())
    }

    /// Ring doorbell to notify GSP
    pub fn notify(&mut self, channel_id: RpcChannelId) -> Result<()> {
        if self.gsp_state != GspState::Ready {
            return Err(Error::NotInitialized);
        }

        // SAFETY: mmio is valid after construction
        unsafe {
            self.doorbell.ring(&mut self.mmio, channel_id.0);
        }

        Ok(())
    }

    /// Shutdown transport
    pub fn shutdown(&mut self) -> Result<()> {
        if self.gsp_state == GspState::Uninitialized {
            return Ok(());
        }

        self.gsp_state = GspState::ShuttingDown;

        // Send shutdown RPC
        if let Some(system_ch) = self.channels.system() {
            let _ = system_ch.call(
                crate::message::RpcFunction::SystemShutdown,
                alloc::vec::Vec::new(),
            );
        }

        self.gsp_state = GspState::Uninitialized;
        Ok(())
    }
}
