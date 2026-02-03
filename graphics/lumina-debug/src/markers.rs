//! Debug Markers
//!
//! Debug markers and regions for GPU debugging.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Debug Color
// ============================================================================

/// A debug color (RGBA).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugColor {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl DebugColor {
    /// Create a new color.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create from u8 values.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create from hex value (0xRRGGBB).
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    /// Convert to array.
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to packed u32.
    pub fn to_u32(&self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        let a = (self.a * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    // Predefined colors
    /// Red.
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Green.
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Blue.
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Yellow.
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Cyan.
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Magenta.
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// White.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// Orange.
    pub const ORANGE: Self = Self::rgb(1.0, 0.5, 0.0);
    /// Purple.
    pub const PURPLE: Self = Self::rgb(0.5, 0.0, 1.0);
    /// Gray.
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
}

impl Default for DebugColor {
    fn default() -> Self {
        Self::WHITE
    }
}

// ============================================================================
// Debug Marker
// ============================================================================

/// A debug marker (point-in-time label).
#[derive(Debug, Clone)]
pub struct DebugMarker {
    /// Label.
    pub label: String,
    /// Color.
    pub color: DebugColor,
    /// Timestamp.
    pub timestamp: u64,
    /// Frame.
    pub frame: u64,
}

impl DebugMarker {
    /// Create a new marker.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: DebugColor::WHITE,
            timestamp: 0,
            frame: 0,
        }
    }

    /// Set color.
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }
}

// ============================================================================
// Debug Region
// ============================================================================

/// A debug region (range of commands).
#[derive(Debug, Clone)]
pub struct DebugRegion {
    /// Label.
    pub label: String,
    /// Color.
    pub color: DebugColor,
    /// Start timestamp.
    pub start_timestamp: u64,
    /// End timestamp.
    pub end_timestamp: u64,
    /// Frame.
    pub frame: u64,
    /// Depth.
    pub depth: u32,
    /// Is active (not yet ended).
    pub active: bool,
}

impl DebugRegion {
    /// Create a new region.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: DebugColor::WHITE,
            start_timestamp: 0,
            end_timestamp: 0,
            frame: 0,
            depth: 0,
            active: true,
        }
    }

    /// Set color.
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }

    /// Get duration.
    pub fn duration(&self) -> u64 {
        self.end_timestamp.saturating_sub(self.start_timestamp)
    }
}

// ============================================================================
// Object Label Type
// ============================================================================

/// Type of object being labeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectLabelType {
    /// Buffer.
    Buffer,
    /// Image/Texture.
    Image,
    /// Sampler.
    Sampler,
    /// Pipeline.
    Pipeline,
    /// Pipeline layout.
    PipelineLayout,
    /// Descriptor set.
    DescriptorSet,
    /// Descriptor set layout.
    DescriptorSetLayout,
    /// Render pass.
    RenderPass,
    /// Framebuffer.
    Framebuffer,
    /// Command buffer.
    CommandBuffer,
    /// Queue.
    Queue,
    /// Fence.
    Fence,
    /// Semaphore.
    Semaphore,
    /// Shader module.
    ShaderModule,
    /// Query pool.
    QueryPool,
    /// Device memory.
    DeviceMemory,
    /// Surface.
    Surface,
    /// Swapchain.
    Swapchain,
    /// Acceleration structure.
    AccelerationStructure,
    /// Unknown.
    Unknown,
}

impl Default for ObjectLabelType {
    fn default() -> Self {
        ObjectLabelType::Unknown
    }
}

// ============================================================================
// Object Label
// ============================================================================

/// Label for a GPU object.
#[derive(Debug, Clone)]
pub struct ObjectLabel {
    /// Object type.
    pub object_type: ObjectLabelType,
    /// Object handle (raw).
    pub handle: u64,
    /// Label.
    pub label: String,
}

impl ObjectLabel {
    /// Create a new object label.
    pub fn new(object_type: ObjectLabelType, handle: u64, label: impl Into<String>) -> Self {
        Self {
            object_type,
            handle,
            label: label.into(),
        }
    }

    /// Create a buffer label.
    pub fn buffer(handle: u64, label: impl Into<String>) -> Self {
        Self::new(ObjectLabelType::Buffer, handle, label)
    }

    /// Create an image label.
    pub fn image(handle: u64, label: impl Into<String>) -> Self {
        Self::new(ObjectLabelType::Image, handle, label)
    }

    /// Create a pipeline label.
    pub fn pipeline(handle: u64, label: impl Into<String>) -> Self {
        Self::new(ObjectLabelType::Pipeline, handle, label)
    }
}

// ============================================================================
// Marker Stack
// ============================================================================

/// Stack of active debug regions.
pub struct MarkerStack {
    /// Active regions.
    regions: Vec<DebugRegion>,
    /// Current depth.
    pub depth: u32,
}

impl MarkerStack {
    /// Create a new marker stack.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            depth: 0,
        }
    }

    /// Push a region.
    pub fn push(&mut self, mut region: DebugRegion) {
        region.depth = self.depth;
        self.regions.push(region);
        self.depth += 1;
    }

    /// Pop a region.
    pub fn pop(&mut self) -> Option<DebugRegion> {
        if let Some(mut region) = self.regions.pop() {
            self.depth = self.depth.saturating_sub(1);
            region.active = false;
            Some(region)
        } else {
            None
        }
    }

    /// Get current region.
    pub fn current(&self) -> Option<&DebugRegion> {
        self.regions.last()
    }

    /// Get current region (mutable).
    pub fn current_mut(&mut self) -> Option<&mut DebugRegion> {
        self.regions.last_mut()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Get depth.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Clear all regions.
    pub fn clear(&mut self) {
        self.regions.clear();
        self.depth = 0;
    }
}

impl Default for MarkerStack {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Marker Manager
// ============================================================================

/// Manager for debug markers and labels.
pub struct MarkerManager {
    /// Is enabled.
    pub enabled: bool,
    /// Marker stack.
    stack: MarkerStack,
    /// Recent markers.
    markers: Vec<DebugMarker>,
    /// Completed regions.
    regions: Vec<DebugRegion>,
    /// Object labels.
    labels: Vec<ObjectLabel>,
    /// Maximum markers to keep.
    pub max_markers: usize,
    /// Maximum regions to keep.
    pub max_regions: usize,
    /// Current frame.
    current_frame: u64,
    /// Current timestamp.
    current_timestamp: u64,
}

impl MarkerManager {
    /// Create a new marker manager.
    pub fn new() -> Self {
        Self {
            enabled: true,
            stack: MarkerStack::new(),
            markers: Vec::new(),
            regions: Vec::new(),
            labels: Vec::new(),
            max_markers: 1000,
            max_regions: 1000,
            current_frame: 0,
            current_timestamp: 0,
        }
    }

    /// Enable markers.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable markers.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set frame.
    pub fn set_frame(&mut self, frame: u64) {
        self.current_frame = frame;
    }

    /// Set timestamp.
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.current_timestamp = timestamp;
    }

    /// Insert a marker.
    pub fn marker(&mut self, label: impl Into<String>, color: DebugColor) {
        if !self.enabled {
            return;
        }

        let mut marker = DebugMarker::new(label);
        marker.color = color;
        marker.frame = self.current_frame;
        marker.timestamp = self.current_timestamp;

        if self.markers.len() >= self.max_markers {
            self.markers.remove(0);
        }
        self.markers.push(marker);
    }

    /// Begin a region.
    pub fn begin_region(&mut self, label: impl Into<String>, color: DebugColor) {
        if !self.enabled {
            return;
        }

        let mut region = DebugRegion::new(label);
        region.color = color;
        region.frame = self.current_frame;
        region.start_timestamp = self.current_timestamp;
        self.stack.push(region);
    }

    /// End current region.
    pub fn end_region(&mut self) {
        if !self.enabled {
            return;
        }

        if let Some(mut region) = self.stack.pop() {
            region.end_timestamp = self.current_timestamp;

            if self.regions.len() >= self.max_regions {
                self.regions.remove(0);
            }
            self.regions.push(region);
        }
    }

    /// Set object label.
    pub fn set_label(&mut self, label: ObjectLabel) {
        if !self.enabled {
            return;
        }

        // Check if exists and update
        for existing in &mut self.labels {
            if existing.object_type == label.object_type && existing.handle == label.handle {
                existing.label = label.label;
                return;
            }
        }

        // Add new
        self.labels.push(label);
    }

    /// Get object label.
    pub fn get_label(&self, object_type: ObjectLabelType, handle: u64) -> Option<&str> {
        self.labels
            .iter()
            .find(|l| l.object_type == object_type && l.handle == handle)
            .map(|l| l.label.as_str())
    }

    /// Remove object label.
    pub fn remove_label(&mut self, object_type: ObjectLabelType, handle: u64) {
        self.labels
            .retain(|l| !(l.object_type == object_type && l.handle == handle));
    }

    /// Get recent markers.
    pub fn recent_markers(&self, count: usize) -> impl Iterator<Item = &DebugMarker> {
        self.markers.iter().rev().take(count)
    }

    /// Get recent regions.
    pub fn recent_regions(&self, count: usize) -> impl Iterator<Item = &DebugRegion> {
        self.regions.iter().rev().take(count)
    }

    /// Get all labels.
    pub fn labels(&self) -> &[ObjectLabel] {
        &self.labels
    }

    /// Get current depth.
    pub fn depth(&self) -> u32 {
        self.stack.depth()
    }

    /// Clear markers and regions.
    pub fn clear(&mut self) {
        self.markers.clear();
        self.regions.clear();
        self.stack.clear();
    }

    /// Clear labels.
    pub fn clear_labels(&mut self) {
        self.labels.clear();
    }
}

impl Default for MarkerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Scoped Region
// ============================================================================

/// RAII scoped debug region.
pub struct ScopedRegion<'a> {
    manager: &'a mut MarkerManager,
}

impl<'a> ScopedRegion<'a> {
    /// Create a new scoped region.
    pub fn new(manager: &'a mut MarkerManager, label: &str, color: DebugColor) -> Self {
        manager.begin_region(label, color);
        Self { manager }
    }
}

impl<'a> Drop for ScopedRegion<'a> {
    fn drop(&mut self) {
        self.manager.end_region();
    }
}
