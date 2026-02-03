//! Swapchain Management
//!
//! Surface presentation and swapchain management for window rendering.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::device::TextureFormat;
use crate::texture::{Texture, TextureUsage};

// ============================================================================
// Present Mode
// ============================================================================

/// Presentation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// No synchronization (may tear).
    Immediate,
    /// V-Sync with mailbox (low latency).
    Mailbox,
    /// V-Sync with FIFO (guaranteed no tear).
    Fifo,
    /// V-Sync with FIFO relaxed.
    FifoRelaxed,
}

impl PresentMode {
    /// Check if mode prevents tearing.
    pub fn prevents_tearing(&self) -> bool {
        !matches!(self, PresentMode::Immediate)
    }

    /// Get typical latency frames.
    pub fn typical_latency(&self) -> u32 {
        match self {
            PresentMode::Immediate => 0,
            PresentMode::Mailbox => 1,
            PresentMode::Fifo => 2,
            PresentMode::FifoRelaxed => 2,
        }
    }
}

impl Default for PresentMode {
    fn default() -> Self {
        PresentMode::Fifo
    }
}

// ============================================================================
// Composite Alpha
// ============================================================================

/// Alpha compositing mode for swapchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositeAlpha {
    /// Alpha channel is ignored.
    Opaque,
    /// Alpha channel is pre-multiplied.
    PreMultiplied,
    /// Alpha channel is post-multiplied.
    PostMultiplied,
    /// Alpha is inherited from surface.
    Inherit,
}

impl Default for CompositeAlpha {
    fn default() -> Self {
        CompositeAlpha::Opaque
    }
}

// ============================================================================
// Surface Transform
// ============================================================================

bitflags! {
    /// Surface transform flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SurfaceTransform: u32 {
        /// Identity (no transform).
        const IDENTITY = 1 << 0;
        /// 90 degree rotation.
        const ROTATE_90 = 1 << 1;
        /// 180 degree rotation.
        const ROTATE_180 = 1 << 2;
        /// 270 degree rotation.
        const ROTATE_270 = 1 << 3;
        /// Horizontal mirror.
        const HORIZONTAL_MIRROR = 1 << 4;
        /// Horizontal mirror + 90 rotation.
        const HORIZONTAL_MIRROR_ROTATE_90 = 1 << 5;
        /// Horizontal mirror + 180 rotation.
        const HORIZONTAL_MIRROR_ROTATE_180 = 1 << 6;
        /// Horizontal mirror + 270 rotation.
        const HORIZONTAL_MIRROR_ROTATE_270 = 1 << 7;
        /// Inherit from surface.
        const INHERIT = 1 << 8;
    }
}

impl Default for SurfaceTransform {
    fn default() -> Self {
        SurfaceTransform::IDENTITY
    }
}

// ============================================================================
// Swapchain Image
// ============================================================================

/// Handle to a swapchain image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SwapchainImageHandle(Handle<SwapchainImage>);

/// A swapchain image.
#[derive(Debug)]
pub struct SwapchainImage {
    /// Handle.
    pub handle: SwapchainImageHandle,
    /// Image index.
    pub index: u32,
    /// Texture.
    pub texture: Texture,
    /// Acquired frame.
    pub acquired_frame: AtomicU64,
    /// In use.
    pub in_use: AtomicU32,
}

impl SwapchainImage {
    /// Create a new swapchain image.
    pub fn new(handle: SwapchainImageHandle, index: u32, texture: Texture) -> Self {
        Self {
            handle,
            index,
            texture,
            acquired_frame: AtomicU64::new(0),
            in_use: AtomicU32::new(0),
        }
    }

    /// Mark as acquired.
    pub fn acquire(&self, frame: u64) {
        self.acquired_frame.store(frame, Ordering::Release);
        self.in_use.store(1, Ordering::Release);
    }

    /// Mark as released.
    pub fn release(&self) {
        self.in_use.store(0, Ordering::Release);
    }

    /// Check if in use.
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::Acquire) != 0
    }
}

// ============================================================================
// Swapchain Description
// ============================================================================

/// Description for swapchain creation.
#[derive(Debug, Clone)]
pub struct SwapchainDesc {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Format.
    pub format: TextureFormat,
    /// Image count.
    pub image_count: u32,
    /// Present mode.
    pub present_mode: PresentMode,
    /// Composite alpha.
    pub composite_alpha: CompositeAlpha,
    /// Surface transform.
    pub transform: SurfaceTransform,
    /// Image usage.
    pub usage: TextureUsage,
    /// Old swapchain (for resize).
    pub old_swapchain: Option<SwapchainHandle>,
}

impl Default for SwapchainDesc {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            format: TextureFormat::Bgra8UnormSrgb,
            image_count: 3,
            present_mode: PresentMode::Fifo,
            composite_alpha: CompositeAlpha::Opaque,
            transform: SurfaceTransform::IDENTITY,
            usage: TextureUsage::COLOR_ATTACHMENT,
            old_swapchain: None,
        }
    }
}

// ============================================================================
// Swapchain Handle
// ============================================================================

/// Handle to a swapchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SwapchainHandle(Handle<Swapchain>);

impl SwapchainHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Swapchain Status
// ============================================================================

/// Swapchain status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapchainStatus {
    /// Swapchain is optimal.
    Optimal,
    /// Swapchain is suboptimal (should recreate).
    Suboptimal,
    /// Swapchain is out of date (must recreate).
    OutOfDate,
    /// Surface was lost.
    SurfaceLost,
}

// ============================================================================
// Acquire Result
// ============================================================================

/// Result of acquiring a swapchain image.
#[derive(Debug)]
pub struct AcquireResult {
    /// Image index.
    pub image_index: u32,
    /// Swapchain status.
    pub status: SwapchainStatus,
    /// Semaphore signaled when image is ready.
    pub image_ready_semaphore: u64,
}

// ============================================================================
// Swapchain
// ============================================================================

/// A swapchain for presentation.
pub struct Swapchain {
    /// Handle.
    pub handle: SwapchainHandle,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Format.
    pub format: TextureFormat,
    /// Present mode.
    pub present_mode: PresentMode,
    /// Images.
    pub images: Vec<SwapchainImage>,
    /// Current image index.
    current_image: AtomicU32,
    /// Frame count.
    frame_count: AtomicU64,
    /// Status.
    status: SwapchainStatus,
}

impl Swapchain {
    /// Create a new swapchain.
    pub fn new(handle: SwapchainHandle, desc: &SwapchainDesc) -> Self {
        Self {
            handle,
            width: desc.width,
            height: desc.height,
            format: desc.format,
            present_mode: desc.present_mode,
            images: Vec::new(),
            current_image: AtomicU32::new(0),
            frame_count: AtomicU64::new(0),
            status: SwapchainStatus::Optimal,
        }
    }

    /// Get image count.
    pub fn image_count(&self) -> u32 {
        self.images.len() as u32
    }

    /// Get current image index.
    pub fn current_image_index(&self) -> u32 {
        self.current_image.load(Ordering::Acquire)
    }

    /// Get current image.
    pub fn current_image(&self) -> Option<&SwapchainImage> {
        let index = self.current_image_index() as usize;
        self.images.get(index)
    }

    /// Get image by index.
    pub fn image(&self, index: u32) -> Option<&SwapchainImage> {
        self.images.get(index as usize)
    }

    /// Get frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Get status.
    pub fn status(&self) -> SwapchainStatus {
        self.status
    }

    /// Get aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Acquire next image (simulated).
    pub fn acquire_next_image(&self, timeout_ns: u64) -> Result<AcquireResult, SwapchainStatus> {
        if self.status != SwapchainStatus::Optimal {
            return Err(self.status);
        }

        let image_count = self.images.len() as u32;
        let current = self.current_image.fetch_add(1, Ordering::AcqRel);
        let next = current % image_count;

        self.frame_count.fetch_add(1, Ordering::Relaxed);

        Ok(AcquireResult {
            image_index: next,
            status: SwapchainStatus::Optimal,
            image_ready_semaphore: 0,
        })
    }

    /// Present (simulated).
    pub fn present(&self, image_index: u32) -> SwapchainStatus {
        if let Some(image) = self.images.get(image_index as usize) {
            image.release();
        }
        self.status
    }

    /// Check if resize is needed.
    pub fn needs_resize(&self, width: u32, height: u32) -> bool {
        self.width != width || self.height != height
    }
}

// ============================================================================
// Swapchain Manager
// ============================================================================

/// Manages swapchains.
pub struct SwapchainManager {
    /// Active swapchains.
    swapchains: Vec<Swapchain>,
    /// Next handle index.
    next_index: AtomicU32,
}

impl SwapchainManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            swapchains: Vec::new(),
            next_index: AtomicU32::new(0),
        }
    }

    /// Create a swapchain.
    pub fn create(&mut self, desc: &SwapchainDesc) -> SwapchainHandle {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let handle = SwapchainHandle::new(index, 0);
        let swapchain = Swapchain::new(handle, desc);

        self.swapchains.push(swapchain);
        handle
    }

    /// Get swapchain.
    pub fn get(&self, handle: SwapchainHandle) -> Option<&Swapchain> {
        self.swapchains.iter().find(|s| s.handle == handle)
    }

    /// Get mutable swapchain.
    pub fn get_mut(&mut self, handle: SwapchainHandle) -> Option<&mut Swapchain> {
        self.swapchains.iter_mut().find(|s| s.handle == handle)
    }

    /// Destroy swapchain.
    pub fn destroy(&mut self, handle: SwapchainHandle) {
        self.swapchains.retain(|s| s.handle != handle);
    }

    /// Swapchain count.
    pub fn count(&self) -> usize {
        self.swapchains.len()
    }
}

impl Default for SwapchainManager {
    fn default() -> Self {
        Self::new()
    }
}
