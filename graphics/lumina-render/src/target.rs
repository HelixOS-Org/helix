//! Render Targets - Flexible Output Configuration
//!
//! This module provides render target management with support for:
//! - Multiple render targets (MRT)
//! - MSAA with automatic resolve
//! - Depth/stencil configurations
//! - Dynamic render target sizing

use alloc::{string::String, vec::Vec};
use core::fmt;

use crate::pass::{ClearValue, LoadOp, StoreOp};
use crate::resource::{SampleCount, TextureDesc, TextureFormat, TextureHandle};

/// Render target description.
#[derive(Debug, Clone)]
pub struct RenderTargetDesc {
    /// Color attachments.
    pub color_attachments: Vec<AttachmentDesc>,
    /// Depth attachment.
    pub depth_attachment: Option<AttachmentDesc>,
    /// Stencil attachment.
    pub stencil_attachment: Option<AttachmentDesc>,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Array layers.
    pub layers: u32,
    /// Sample count.
    pub samples: SampleCount,
    /// Name for debugging.
    pub name: String,
}

impl Default for RenderTargetDesc {
    fn default() -> Self {
        Self {
            color_attachments: Vec::new(),
            depth_attachment: None,
            stencil_attachment: None,
            width: 0,
            height: 0,
            layers: 1,
            samples: SampleCount::X1,
            name: String::new(),
        }
    }
}

impl RenderTargetDesc {
    /// Create a new render target description.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Add a color attachment.
    pub fn with_color(mut self, format: TextureFormat) -> Self {
        self.color_attachments.push(AttachmentDesc {
            format,
            samples: self.samples,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            resolve_target: None,
        });
        self
    }

    /// Add an HDR color attachment.
    pub fn with_hdr_color(mut self) -> Self {
        self.color_attachments.push(AttachmentDesc {
            format: TextureFormat::RGBA16Float,
            samples: self.samples,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            resolve_target: None,
        });
        self
    }

    /// Add a depth attachment.
    pub fn with_depth(mut self, format: TextureFormat) -> Self {
        self.depth_attachment = Some(AttachmentDesc {
            format,
            samples: self.samples,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearValue::Depth(1.0),
            resolve_target: None,
        });
        self
    }

    /// Add a depth-stencil attachment.
    pub fn with_depth_stencil(mut self) -> Self {
        self.depth_attachment = Some(AttachmentDesc {
            format: TextureFormat::D24UnormS8Uint,
            samples: self.samples,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearValue::DepthStencil(1.0, 0),
            resolve_target: None,
        });
        self
    }

    /// Set MSAA samples.
    pub fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        // Update all attachments
        for att in &mut self.color_attachments {
            att.samples = samples;
        }
        if let Some(ref mut att) = self.depth_attachment {
            att.samples = samples;
        }
        self
    }

    /// Set array layers.
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }

    /// Set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Create GBuffer render target.
    pub fn gbuffer(width: u32, height: u32) -> Self {
        Self::new(width, height)
            .with_color(TextureFormat::RGBA16Float) // Albedo + Metallic
            .with_color(TextureFormat::RGBA16Float) // Normal + Roughness
            .with_color(TextureFormat::RGBA16Float) // Emission + AO
            .with_color(TextureFormat::RG16Float)   // Motion vectors
            .with_depth(TextureFormat::D32Float)
            .with_name("GBuffer")
    }

    /// Create shadow map render target.
    pub fn shadow_map(size: u32) -> Self {
        Self::new(size, size)
            .with_depth(TextureFormat::D32Float)
            .with_name("ShadowMap")
    }

    /// Create shadow cascade render target.
    pub fn shadow_cascade(size: u32, cascades: u32) -> Self {
        Self::new(size, size)
            .with_depth(TextureFormat::D32Float)
            .with_layers(cascades)
            .with_name("ShadowCascade")
    }

    /// Get total attachment count.
    pub fn attachment_count(&self) -> usize {
        self.color_attachments.len()
            + self.depth_attachment.as_ref().map_or(0, |_| 1)
            + self.stencil_attachment.as_ref().map_or(0, |_| 1)
    }
}

/// Attachment description.
#[derive(Debug, Clone)]
pub struct AttachmentDesc {
    /// Format.
    pub format: TextureFormat,
    /// Sample count.
    pub samples: SampleCount,
    /// Load operation.
    pub load_op: LoadOp,
    /// Store operation.
    pub store_op: StoreOp,
    /// Clear value.
    pub clear_value: ClearValue,
    /// MSAA resolve target.
    pub resolve_target: Option<TextureHandle>,
}

impl AttachmentDesc {
    /// Set load operation.
    pub fn with_load(mut self, op: LoadOp) -> Self {
        self.load_op = op;
        self
    }

    /// Set store operation.
    pub fn with_store(mut self, op: StoreOp) -> Self {
        self.store_op = op;
        self
    }

    /// Set clear value.
    pub fn with_clear(mut self, value: ClearValue) -> Self {
        self.clear_value = value;
        self
    }

    /// Set MSAA resolve target.
    pub fn with_resolve(mut self, target: TextureHandle) -> Self {
        self.resolve_target = Some(target);
        self
    }
}

/// Runtime render target.
pub struct RenderTarget {
    /// Description.
    pub desc: RenderTargetDesc,
    /// Color attachment handles.
    pub color_handles: Vec<TextureHandle>,
    /// Depth attachment handle.
    pub depth_handle: Option<TextureHandle>,
    /// Stencil attachment handle.
    pub stencil_handle: Option<TextureHandle>,
    /// Resolve target handles.
    pub resolve_handles: Vec<Option<TextureHandle>>,
    /// Current width.
    pub width: u32,
    /// Current height.
    pub height: u32,
}

impl RenderTarget {
    /// Create a new render target.
    pub fn new(desc: RenderTargetDesc) -> Self {
        let color_count = desc.color_attachments.len();
        Self {
            width: desc.width,
            height: desc.height,
            desc,
            color_handles: Vec::with_capacity(color_count),
            depth_handle: None,
            stencil_handle: None,
            resolve_handles: vec![None; color_count],
        }
    }

    /// Check if render target needs resize.
    pub fn needs_resize(&self, width: u32, height: u32) -> bool {
        self.width != width || self.height != height
    }

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        // Actual resource recreation would happen here
    }

    /// Get color attachment at index.
    pub fn color(&self, index: usize) -> Option<TextureHandle> {
        self.color_handles.get(index).copied()
    }

    /// Get depth attachment.
    pub fn depth(&self) -> Option<TextureHandle> {
        self.depth_handle
    }

    /// Check if has MSAA.
    pub fn is_msaa(&self) -> bool {
        self.desc.samples != SampleCount::X1
    }

    /// Get sample count.
    pub fn samples(&self) -> SampleCount {
        self.desc.samples
    }
}

/// Attachment operations.
#[derive(Debug, Clone, Copy)]
pub struct AttachmentOp {
    /// Load operation.
    pub load: LoadOp,
    /// Store operation.
    pub store: StoreOp,
}

impl AttachmentOp {
    /// Clear and store.
    pub const CLEAR_STORE: Self = Self {
        load: LoadOp::Clear,
        store: StoreOp::Store,
    };

    /// Load and store.
    pub const LOAD_STORE: Self = Self {
        load: LoadOp::Load,
        store: StoreOp::Store,
    };

    /// Don't care.
    pub const DONT_CARE: Self = Self {
        load: LoadOp::DontCare,
        store: StoreOp::DontCare,
    };

    /// Clear and don't store.
    pub const CLEAR_DISCARD: Self = Self {
        load: LoadOp::Clear,
        store: StoreOp::DontCare,
    };
}

impl Default for AttachmentOp {
    fn default() -> Self {
        Self::CLEAR_STORE
    }
}

/// Attachment reference in a render pass.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Texture handle.
    pub handle: TextureHandle,
    /// Operations.
    pub ops: AttachmentOp,
    /// Clear value.
    pub clear: ClearValue,
    /// Array layer.
    pub layer: u32,
    /// Mip level.
    pub level: u32,
    /// Resolve target.
    pub resolve: Option<TextureHandle>,
}

impl Attachment {
    /// Create a color attachment.
    pub fn color(handle: TextureHandle) -> Self {
        Self {
            handle,
            ops: AttachmentOp::CLEAR_STORE,
            clear: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            layer: 0,
            level: 0,
            resolve: None,
        }
    }

    /// Create a depth attachment.
    pub fn depth(handle: TextureHandle) -> Self {
        Self {
            handle,
            ops: AttachmentOp::CLEAR_STORE,
            clear: ClearValue::Depth(1.0),
            layer: 0,
            level: 0,
            resolve: None,
        }
    }

    /// With operations.
    pub fn with_ops(mut self, ops: AttachmentOp) -> Self {
        self.ops = ops;
        self
    }

    /// With clear value.
    pub fn with_clear(mut self, clear: ClearValue) -> Self {
        self.clear = clear;
        self
    }

    /// With array layer.
    pub fn with_layer(mut self, layer: u32) -> Self {
        self.layer = layer;
        self
    }

    /// With mip level.
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    /// With resolve target.
    pub fn with_resolve(mut self, target: TextureHandle) -> Self {
        self.resolve = Some(target);
        self
    }
}

/// Render target pool for efficient reuse.
pub struct RenderTargetPool {
    /// Available render targets.
    available: Vec<RenderTarget>,
    /// In-use render targets.
    in_use: Vec<RenderTarget>,
    /// Frame counter.
    frame_count: u64,
}

impl RenderTargetPool {
    /// Create a new pool.
    pub fn new() -> Self {
        Self {
            available: Vec::new(),
            in_use: Vec::new(),
            frame_count: 0,
        }
    }

    /// Acquire a render target.
    pub fn acquire(&mut self, desc: &RenderTargetDesc) -> RenderTarget {
        // Try to find compatible target
        if let Some(pos) = self.find_compatible(desc) {
            let mut target = self.available.remove(pos);
            if target.needs_resize(desc.width, desc.height) {
                target.resize(desc.width, desc.height);
            }
            self.in_use.push(target);
            return self.in_use.last().cloned().unwrap();
        }

        // Create new
        let target = RenderTarget::new(desc.clone());
        self.in_use.push(target);
        self.in_use.last().cloned().unwrap()
    }

    /// Release all in-use targets back to pool.
    pub fn release_all(&mut self) {
        self.available.append(&mut self.in_use);
    }

    /// Advance to next frame.
    pub fn next_frame(&mut self) {
        self.frame_count += 1;
        self.release_all();

        // Cleanup old targets
        const MAX_AGE: u64 = 60;
        self.available.retain(|_| true); // Keep all for now
    }

    fn find_compatible(&self, desc: &RenderTargetDesc) -> Option<usize> {
        self.available.iter().position(|t| {
            t.desc.color_attachments.len() == desc.color_attachments.len()
                && t.desc.depth_attachment.is_some() == desc.depth_attachment.is_some()
                && t.desc.samples == desc.samples
                && t.desc.layers == desc.layers
                && Self::formats_compatible(&t.desc, desc)
        })
    }

    fn formats_compatible(a: &RenderTargetDesc, b: &RenderTargetDesc) -> bool {
        for (att_a, att_b) in a.color_attachments.iter().zip(&b.color_attachments) {
            if att_a.format != att_b.format {
                return false;
            }
        }

        match (&a.depth_attachment, &b.depth_attachment) {
            (Some(da), Some(db)) => da.format == db.format,
            (None, None) => true,
            _ => false,
        }
    }
}

impl Default for RenderTargetPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RenderTarget {
    fn clone(&self) -> Self {
        Self {
            desc: self.desc.clone(),
            color_handles: self.color_handles.clone(),
            depth_handle: self.depth_handle,
            stencil_handle: self.stencil_handle,
            resolve_handles: self.resolve_handles.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_target_desc() {
        let desc = RenderTargetDesc::new(1920, 1080)
            .with_color(TextureFormat::RGBA8Unorm)
            .with_depth(TextureFormat::D32Float);

        assert_eq!(desc.color_attachments.len(), 1);
        assert!(desc.depth_attachment.is_some());
    }

    #[test]
    fn test_gbuffer() {
        let gbuffer = RenderTargetDesc::gbuffer(1920, 1080);
        assert_eq!(gbuffer.color_attachments.len(), 4);
        assert!(gbuffer.depth_attachment.is_some());
    }

    #[test]
    fn test_render_target_pool() {
        let mut pool = RenderTargetPool::new();
        let desc = RenderTargetDesc::new(1920, 1080)
            .with_color(TextureFormat::RGBA8Unorm);

        let _target = pool.acquire(&desc);
        assert_eq!(pool.in_use.len(), 1);

        pool.release_all();
        assert_eq!(pool.available.len(), 1);
    }
}
