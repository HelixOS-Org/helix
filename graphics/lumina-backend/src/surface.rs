//! Window Surface Abstraction
//!
//! Platform-agnostic window surface handling.

use alloc::string::String;
use core::sync::atomic::{AtomicU32, Ordering};

use lumina_core::Handle;

use crate::device::TextureFormat;
use crate::swapchain::{CompositeAlpha, PresentMode, SurfaceTransform};

// ============================================================================
// Surface Format
// ============================================================================

/// Surface format with color space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceFormat {
    /// Texture format.
    pub format: TextureFormat,
    /// Color space.
    pub color_space: ColorSpace,
}

impl Default for SurfaceFormat {
    fn default() -> Self {
        Self {
            format: TextureFormat::Bgra8Unorm,
            color_space: ColorSpace::SrgbNonLinear,
        }
    }
}

// ============================================================================
// Color Space
// ============================================================================

/// Color space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// sRGB non-linear.
    SrgbNonLinear,
    /// Extended sRGB linear.
    ExtendedSrgbLinear,
    /// Display P3 non-linear.
    DisplayP3NonLinear,
    /// Display P3 linear.
    DisplayP3Linear,
    /// DCI-P3 non-linear.
    DciP3NonLinear,
    /// BT709 linear.
    Bt709Linear,
    /// BT709 non-linear.
    Bt709NonLinear,
    /// BT2020 linear.
    Bt2020Linear,
    /// HDR10 ST2084.
    Hdr10St2084,
    /// HDR10 HLG.
    Hdr10Hlg,
    /// Dolby Vision.
    DolbyVision,
    /// Adobe RGB linear.
    AdobeRgbLinear,
    /// Adobe RGB non-linear.
    AdobeRgbNonLinear,
    /// Pass-through.
    PassThrough,
}

impl Default for ColorSpace {
    fn default() -> Self {
        ColorSpace::SrgbNonLinear
    }
}

// ============================================================================
// Surface Capabilities
// ============================================================================

/// Surface capabilities.
#[derive(Debug, Clone)]
pub struct SurfaceCapabilities {
    /// Minimum image count.
    pub min_image_count: u32,
    /// Maximum image count (0 = unlimited).
    pub max_image_count: u32,
    /// Current extent.
    pub current_extent: SurfaceExtent,
    /// Minimum extent.
    pub min_extent: SurfaceExtent,
    /// Maximum extent.
    pub max_extent: SurfaceExtent,
    /// Maximum image array layers.
    pub max_image_array_layers: u32,
    /// Supported transforms.
    pub supported_transforms: SurfaceTransform,
    /// Current transform.
    pub current_transform: SurfaceTransform,
    /// Supported composite alpha.
    pub supported_composite_alpha: CompositeAlpha,
    /// Supported present modes.
    pub supported_present_modes: Vec<PresentMode>,
    /// Supported formats.
    pub supported_formats: Vec<SurfaceFormat>,
}

impl Default for SurfaceCapabilities {
    fn default() -> Self {
        Self {
            min_image_count: 2,
            max_image_count: 3,
            current_extent: SurfaceExtent::default(),
            min_extent: SurfaceExtent {
                width: 1,
                height: 1,
            },
            max_extent: SurfaceExtent {
                width: 16384,
                height: 16384,
            },
            max_image_array_layers: 1,
            supported_transforms: SurfaceTransform::IDENTITY,
            current_transform: SurfaceTransform::IDENTITY,
            supported_composite_alpha: CompositeAlpha::OPAQUE,
            supported_present_modes: vec![PresentMode::Fifo],
            supported_formats: vec![SurfaceFormat::default()],
        }
    }
}

impl SurfaceCapabilities {
    /// Check if format is supported.
    pub fn supports_format(&self, format: &SurfaceFormat) -> bool {
        self.supported_formats.contains(format)
    }

    /// Check if present mode is supported.
    pub fn supports_present_mode(&self, mode: PresentMode) -> bool {
        self.supported_present_modes.contains(&mode)
    }

    /// Get preferred format.
    pub fn preferred_format(&self) -> SurfaceFormat {
        // Prefer sRGB
        for format in &self.supported_formats {
            if format.color_space == ColorSpace::SrgbNonLinear {
                if matches!(
                    format.format,
                    TextureFormat::Bgra8Unorm
                        | TextureFormat::Rgba8Unorm
                        | TextureFormat::Bgra8UnormSrgb
                        | TextureFormat::Rgba8UnormSrgb
                ) {
                    return *format;
                }
            }
        }
        self.supported_formats.first().copied().unwrap_or_default()
    }

    /// Get preferred present mode.
    pub fn preferred_present_mode(&self) -> PresentMode {
        // Prefer mailbox (triple buffering)
        if self.supported_present_modes.contains(&PresentMode::Mailbox) {
            return PresentMode::Mailbox;
        }
        // Fall back to FIFO (vsync)
        PresentMode::Fifo
    }

    /// Clamp image count.
    pub fn clamp_image_count(&self, desired: u32) -> u32 {
        let min = self.min_image_count;
        let max = if self.max_image_count == 0 {
            u32::MAX
        } else {
            self.max_image_count
        };
        desired.clamp(min, max)
    }

    /// Clamp extent.
    pub fn clamp_extent(&self, desired: SurfaceExtent) -> SurfaceExtent {
        SurfaceExtent {
            width: desired
                .width
                .clamp(self.min_extent.width, self.max_extent.width),
            height: desired
                .height
                .clamp(self.min_extent.height, self.max_extent.height),
        }
    }
}

// ============================================================================
// Surface Extent
// ============================================================================

/// Surface extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceExtent {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl SurfaceExtent {
    /// Create a new extent.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Get area.
    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Get aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Check if extent is zero.
    pub fn is_zero(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

// ============================================================================
// Surface Handle
// ============================================================================

/// Handle to a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceHandle(Handle<Surface>);

impl SurfaceHandle {
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
// Native Window Handle
// ============================================================================

/// Native window handle.
#[derive(Debug, Clone, Copy)]
pub enum NativeWindowHandle {
    /// X11 window (display, window).
    X11 { display: *mut (), window: u64 },
    /// Wayland surface (display, surface).
    Wayland { display: *mut (), surface: *mut () },
    /// Windows HWND.
    Win32 { hwnd: *mut (), hinstance: *mut () },
    /// macOS NSView.
    Cocoa { ns_view: *mut () },
    /// iOS UIView.
    UiKit { ui_view: *mut () },
    /// Android ANativeWindow.
    Android { a_native_window: *mut () },
    /// Web canvas.
    Web { canvas_id: u32 },
    /// Null/headless.
    Null,
}

// Safety: Pointers are only stored, not dereferenced.
unsafe impl Send for NativeWindowHandle {}
unsafe impl Sync for NativeWindowHandle {}

impl Default for NativeWindowHandle {
    fn default() -> Self {
        NativeWindowHandle::Null
    }
}

// ============================================================================
// Surface Description
// ============================================================================

/// Description for surface creation.
#[derive(Debug, Clone)]
pub struct SurfaceDesc {
    /// Native window handle.
    pub window: NativeWindowHandle,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for SurfaceDesc {
    fn default() -> Self {
        Self {
            window: NativeWindowHandle::Null,
            width: 800,
            height: 600,
            label: None,
        }
    }
}

// ============================================================================
// Surface State
// ============================================================================

/// Surface state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceState {
    /// Surface is ready.
    Ready,
    /// Surface needs resize.
    NeedsResize,
    /// Surface is minimized.
    Minimized,
    /// Surface is lost.
    Lost,
}

// ============================================================================
// Surface
// ============================================================================

/// A window surface.
pub struct Surface {
    /// Handle.
    pub handle: SurfaceHandle,
    /// Native window.
    pub window: NativeWindowHandle,
    /// Current width.
    pub width: u32,
    /// Current height.
    pub height: u32,
    /// Surface state.
    pub state: SurfaceState,
    /// Capabilities.
    pub capabilities: SurfaceCapabilities,
    /// Debug label.
    pub label: Option<String>,
}

impl Surface {
    /// Get current extent.
    pub fn extent(&self) -> SurfaceExtent {
        SurfaceExtent {
            width: self.width,
            height: self.height,
        }
    }

    /// Check if surface needs resize.
    pub fn needs_resize(&self) -> bool {
        self.state == SurfaceState::NeedsResize
    }

    /// Check if surface is valid.
    pub fn is_valid(&self) -> bool {
        self.state != SurfaceState::Lost && !self.extent().is_zero()
    }

    /// Mark as resized.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.state = SurfaceState::NeedsResize;
    }

    /// Mark as ready.
    pub fn mark_ready(&mut self) {
        self.state = SurfaceState::Ready;
    }

    /// Mark as lost.
    pub fn mark_lost(&mut self) {
        self.state = SurfaceState::Lost;
    }

    /// Mark as minimized.
    pub fn mark_minimized(&mut self) {
        self.state = SurfaceState::Minimized;
    }
}

// ============================================================================
// Surface Manager
// ============================================================================

/// Manages window surfaces.
pub struct SurfaceManager {
    /// Surfaces.
    surfaces: Vec<Option<Surface>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Surface count.
    count: AtomicU32,
}

impl SurfaceManager {
    /// Create a new surface manager.
    pub fn new() -> Self {
        Self {
            surfaces: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            count: AtomicU32::new(0),
        }
    }

    /// Create a surface.
    pub fn create(&mut self, desc: &SurfaceDesc) -> SurfaceHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.surfaces.len() as u32;
            self.surfaces.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = SurfaceHandle::new(index, generation);
        let surface = Surface {
            handle,
            window: desc.window,
            width: desc.width,
            height: desc.height,
            state: SurfaceState::Ready,
            capabilities: SurfaceCapabilities::default(),
            label: desc.label.clone(),
        };

        self.surfaces[index as usize] = Some(surface);
        self.count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Get a surface.
    pub fn get(&self, handle: SurfaceHandle) -> Option<&Surface> {
        let index = handle.index() as usize;
        self.surfaces.get(index)?.as_ref()
    }

    /// Get a surface mutably.
    pub fn get_mut(&mut self, handle: SurfaceHandle) -> Option<&mut Surface> {
        let index = handle.index() as usize;
        self.surfaces.get_mut(index)?.as_mut()
    }

    /// Destroy a surface.
    pub fn destroy(&mut self, handle: SurfaceHandle) {
        let index = handle.index() as usize;
        if index < self.surfaces.len() {
            if self.surfaces[index].take().is_some() {
                self.count.fetch_sub(1, Ordering::Relaxed);
            }
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
        }
    }

    /// Update surface capabilities.
    pub fn update_capabilities(
        &mut self,
        handle: SurfaceHandle,
        capabilities: SurfaceCapabilities,
    ) {
        if let Some(surface) = self.get_mut(handle) {
            surface.capabilities = capabilities;
        }
    }

    /// Resize a surface.
    pub fn resize(&mut self, handle: SurfaceHandle, width: u32, height: u32) {
        if let Some(surface) = self.get_mut(handle) {
            surface.resize(width, height);
        }
    }

    /// Get surface count.
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get all valid surfaces.
    pub fn iter_valid(&self) -> impl Iterator<Item = &Surface> {
        self.surfaces
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|s| s.is_valid())
    }
}

impl Default for SurfaceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Display Mode
// ============================================================================

/// Display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayMode {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Refresh rate in Hz.
    pub refresh_rate: u32,
}

impl DisplayMode {
    /// Create a new display mode.
    pub fn new(width: u32, height: u32, refresh_rate: u32) -> Self {
        Self {
            width,
            height,
            refresh_rate,
        }
    }
}

// ============================================================================
// Display Info
// ============================================================================

/// Display information.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Display name.
    pub name: String,
    /// Display modes.
    pub modes: Vec<DisplayMode>,
    /// Current mode index.
    pub current_mode: usize,
    /// Primary display.
    pub is_primary: bool,
    /// Position X.
    pub position_x: i32,
    /// Position Y.
    pub position_y: i32,
    /// Physical width in mm.
    pub physical_width_mm: u32,
    /// Physical height in mm.
    pub physical_height_mm: u32,
    /// DPI scale factor.
    pub scale_factor: f32,
}

impl DisplayInfo {
    /// Get current mode.
    pub fn current_mode(&self) -> Option<&DisplayMode> {
        self.modes.get(self.current_mode)
    }

    /// Get DPI.
    pub fn dpi(&self) -> f32 {
        96.0 * self.scale_factor
    }
}
