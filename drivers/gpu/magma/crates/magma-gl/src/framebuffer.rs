//! # Framebuffer Management
//!
//! OpenGL framebuffer objects translated to Vulkan framebuffers and render passes.

use alloc::vec::Vec;

use crate::context::{FramebufferAttachment, FramebufferObject};
use crate::enums::*;
use crate::texture;
use crate::types::*;

// =============================================================================
// ATTACHMENT DESCRIPTION
// =============================================================================

/// Vulkan attachment load operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp {
    /// Load existing content
    Load,
    /// Clear to value
    Clear,
    /// Don't care about previous content
    DontCare,
}

/// Vulkan attachment store operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    /// Store content
    Store,
    /// Don't care about storing
    DontCare,
}

/// Render pass attachment description
#[derive(Debug, Clone)]
pub struct AttachmentDesc {
    /// Vulkan format
    pub format: u32,
    /// Sample count
    pub samples: u32,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Stencil load operation
    pub stencil_load_op: LoadOp,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Initial layout
    pub initial_layout: u32,
    /// Final layout
    pub final_layout: u32,
}

impl Default for AttachmentDesc {
    fn default() -> Self {
        Self {
            format: 0,
            samples: 1,
            load_op: LoadOp::DontCare,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: 0, // VK_IMAGE_LAYOUT_UNDEFINED
            final_layout: 2,   // VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
        }
    }
}

// =============================================================================
// RENDER PASS
// =============================================================================

/// Render pass description for caching
#[derive(Debug, Clone, Default)]
pub struct RenderPassDesc {
    /// Color attachments
    pub color_attachments: Vec<AttachmentDesc>,
    /// Depth-stencil attachment
    pub depth_stencil_attachment: Option<AttachmentDesc>,
    /// Subpass count (always 1 for GL compatibility)
    pub subpass_count: u32,
}

impl RenderPassDesc {
    /// Create from framebuffer state
    pub fn from_framebuffer(fbo: &FramebufferObject, formats: &[u32]) -> Self {
        let mut desc = RenderPassDesc {
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
            subpass_count: 1,
        };

        // Add color attachments
        for (i, attachment) in fbo.color_attachments.iter().enumerate() {
            if attachment.name != 0 {
                let mut att_desc = AttachmentDesc::default();
                if i < formats.len() {
                    att_desc.format = formats[i];
                }
                att_desc.final_layout = 2; // VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                desc.color_attachments.push(att_desc);
            }
        }

        // Add depth-stencil attachment
        if fbo.depth_attachment.name != 0 || fbo.stencil_attachment.name != 0 {
            let mut att_desc = AttachmentDesc::default();
            att_desc.final_layout = 3; // VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL
            desc.depth_stencil_attachment = Some(att_desc);
        }

        desc
    }

    /// Calculate hash for caching
    pub fn hash(&self) -> u64 {
        // Simple hash for now
        let mut h: u64 = 0;
        for (i, att) in self.color_attachments.iter().enumerate() {
            h ^= (att.format as u64) << (i * 8);
            h ^= (att.samples as u64) << 32;
        }
        if let Some(ref ds) = self.depth_stencil_attachment {
            h ^= (ds.format as u64) << 40;
        }
        h
    }
}

// =============================================================================
// FRAMEBUFFER COMPLETENESS
// =============================================================================

/// Check framebuffer completeness status
pub fn check_framebuffer_status(
    fbo: &FramebufferObject,
    texture_exists: impl Fn(u32) -> bool,
    renderbuffer_exists: impl Fn(u32) -> bool,
) -> GLenum {
    // Check if FBO 0 (default framebuffer)
    if fbo.name == 0 {
        return GL_FRAMEBUFFER_COMPLETE;
    }

    let mut has_attachment = false;
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut samples: Option<u32> = None;

    // Check color attachments
    for attachment in &fbo.color_attachments {
        if attachment.name == 0 {
            continue;
        }

        has_attachment = true;

        // Verify attachment exists
        if attachment.is_texture {
            if !texture_exists(attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        } else {
            if !renderbuffer_exists(attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        }

        // TODO: Check dimensions match
        // TODO: Check sample counts match
    }

    // Check depth attachment
    if fbo.depth_attachment.name != 0 {
        has_attachment = true;
        if fbo.depth_attachment.is_texture {
            if !texture_exists(fbo.depth_attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        } else {
            if !renderbuffer_exists(fbo.depth_attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        }
    }

    // Check stencil attachment
    if fbo.stencil_attachment.name != 0 {
        has_attachment = true;
        if fbo.stencil_attachment.is_texture {
            if !texture_exists(fbo.stencil_attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        } else {
            if !renderbuffer_exists(fbo.stencil_attachment.name) {
                return GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT;
            }
        }
    }

    // Must have at least one attachment
    if !has_attachment {
        return GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT;
    }

    GL_FRAMEBUFFER_COMPLETE
}

// =============================================================================
// BLIT OPERATIONS
// =============================================================================

/// Blit filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlitFilter {
    /// Nearest neighbor
    Nearest,
    /// Linear interpolation
    Linear,
}

impl From<GLenum> for BlitFilter {
    fn from(filter: GLenum) -> Self {
        match filter {
            GL_NEAREST => BlitFilter::Nearest,
            GL_LINEAR => BlitFilter::Linear,
            _ => BlitFilter::Nearest,
        }
    }
}

/// Blit region
#[derive(Debug, Clone, Copy)]
pub struct BlitRegion {
    /// Source X0
    pub src_x0: i32,
    /// Source Y0
    pub src_y0: i32,
    /// Source X1
    pub src_x1: i32,
    /// Source Y1
    pub src_y1: i32,
    /// Destination X0
    pub dst_x0: i32,
    /// Destination Y0
    pub dst_y0: i32,
    /// Destination X1
    pub dst_x1: i32,
    /// Destination Y1
    pub dst_y1: i32,
}

impl BlitRegion {
    /// Check if source region is flipped
    pub fn src_flipped(&self) -> (bool, bool) {
        (self.src_x0 > self.src_x1, self.src_y0 > self.src_y1)
    }

    /// Check if destination region is flipped
    pub fn dst_flipped(&self) -> (bool, bool) {
        (self.dst_x0 > self.dst_x1, self.dst_y0 > self.dst_y1)
    }

    /// Convert to Vulkan blit regions
    pub fn to_vk_regions(&self) -> (VkOffset3D, VkOffset3D, VkOffset3D, VkOffset3D) {
        (
            VkOffset3D {
                x: self.src_x0,
                y: self.src_y0,
                z: 0,
            },
            VkOffset3D {
                x: self.src_x1,
                y: self.src_y1,
                z: 1,
            },
            VkOffset3D {
                x: self.dst_x0,
                y: self.dst_y0,
                z: 0,
            },
            VkOffset3D {
                x: self.dst_x1,
                y: self.dst_y1,
                z: 1,
            },
        )
    }
}

/// Vulkan offset (placeholder)
#[derive(Debug, Clone, Copy)]
pub struct VkOffset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

// =============================================================================
// CLEAR OPERATIONS
// =============================================================================

/// Clear value union
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    /// Color clear (RGBA float)
    ColorFloat([f32; 4]),
    /// Color clear (RGBA int)
    ColorInt([i32; 4]),
    /// Color clear (RGBA uint)
    ColorUint([u32; 4]),
    /// Depth-stencil clear
    DepthStencil { depth: f32, stencil: u32 },
}

impl ClearValue {
    /// Create color clear from GL clear color
    pub fn from_color(r: f32, g: f32, b: f32, a: f32) -> Self {
        ClearValue::ColorFloat([r, g, b, a])
    }

    /// Create depth-stencil clear
    pub fn from_depth_stencil(depth: f64, stencil: i32) -> Self {
        ClearValue::DepthStencil {
            depth: depth as f32,
            stencil: stencil as u32,
        }
    }
}

// =============================================================================
// READ PIXELS
// =============================================================================

/// Read pixels parameters
#[derive(Debug, Clone)]
pub struct ReadPixelsParams {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Pixel format
    pub format: GLenum,
    /// Pixel type
    pub type_: GLenum,
    /// Pack alignment
    pub pack_alignment: u32,
    /// Pack row length (0 = width)
    pub pack_row_length: u32,
    /// Pack skip rows
    pub pack_skip_rows: u32,
    /// Pack skip pixels
    pub pack_skip_pixels: u32,
}

impl ReadPixelsParams {
    /// Calculate row stride in bytes
    pub fn row_stride(&self) -> usize {
        let row_length = if self.pack_row_length > 0 {
            self.pack_row_length
        } else {
            self.width
        };

        let pixel_size = texture::texture_size(1, 1, 1, self.format, self.type_);
        let row_bytes = row_length as usize * pixel_size;

        // Align to pack_alignment
        let alignment = self.pack_alignment as usize;
        (row_bytes + alignment - 1) & !(alignment - 1)
    }

    /// Calculate total buffer size needed
    pub fn buffer_size(&self) -> usize {
        let stride = self.row_stride();
        stride * self.height as usize
    }
}

// =============================================================================
// DRAW BUFFERS
// =============================================================================

/// Draw buffer configuration
#[derive(Debug, Clone)]
pub struct DrawBufferConfig {
    /// Active color attachments
    pub attachments: Vec<u32>,
}

impl DrawBufferConfig {
    /// Create from GL draw buffers array
    pub fn from_gl_draw_buffers(buffers: &[GLenum]) -> Self {
        let attachments = buffers
            .iter()
            .filter_map(|&buf| {
                if buf >= GL_COLOR_ATTACHMENT0 && buf < GL_COLOR_ATTACHMENT0 + 8 {
                    Some(buf - GL_COLOR_ATTACHMENT0)
                } else if buf == 0 {
                    None // GL_NONE
                } else {
                    None
                }
            })
            .collect();

        Self { attachments }
    }

    /// Get number of active draw buffers
    pub fn count(&self) -> usize {
        self.attachments.len()
    }
}

// =============================================================================
// INVALIDATE
// =============================================================================

/// Framebuffer invalidation for tile-based GPUs
pub fn invalidate_attachments(attachments: &[GLenum]) -> Vec<u32> {
    attachments
        .iter()
        .filter_map(|&att| {
            if att >= GL_COLOR_ATTACHMENT0 && att < GL_COLOR_ATTACHMENT0 + 8 {
                Some(att - GL_COLOR_ATTACHMENT0)
            } else if att == GL_DEPTH_ATTACHMENT {
                Some(0x1000) // Special marker for depth
            } else if att == GL_STENCIL_ATTACHMENT {
                Some(0x1001) // Special marker for stencil
            } else if att == GL_DEPTH_STENCIL_ATTACHMENT {
                Some(0x1002) // Special marker for depth-stencil
            } else {
                None
            }
        })
        .collect()
}
