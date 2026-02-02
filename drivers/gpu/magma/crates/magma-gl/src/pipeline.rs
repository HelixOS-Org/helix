//! # Pipeline Management
//!
//! Vulkan graphics and compute pipeline creation and caching.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::context::{GlContext, ProgramObject};
use crate::enums::*;
use crate::state::{
    BlendState, DepthState, RasterState, StencilState, VertexAttribState, MAX_VERTEX_ATTRIBS,
};
use crate::texture::SamplerState;

// =============================================================================
// PIPELINE STATE HASH
// =============================================================================

/// Key for pipeline cache lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// Program handle
    pub program: u32,
    /// Render pass hash
    pub render_pass_hash: u64,
    /// Vertex input hash
    pub vertex_input_hash: u64,
    /// Rasterization state hash
    pub raster_hash: u64,
    /// Blend state hash
    pub blend_hash: u64,
    /// Depth-stencil state hash
    pub depth_stencil_hash: u64,
    /// Primitive topology
    pub topology: u32,
    /// Sample count
    pub samples: u32,
}

impl PipelineKey {
    /// Create pipeline key from current GL state
    pub fn from_state(
        program: u32,
        render_pass_hash: u64,
        raster: &RasterState,
        blend: &BlendState,
        depth: &DepthState,
        stencil: &StencilState,
        attribs: &[VertexAttribState; MAX_VERTEX_ATTRIBS],
        topology: GLenum,
        samples: u32,
    ) -> Self {
        Self {
            program,
            render_pass_hash,
            vertex_input_hash: hash_vertex_input(attribs),
            raster_hash: hash_raster_state(raster),
            blend_hash: hash_blend_state(blend),
            depth_stencil_hash: hash_depth_stencil_state(depth, stencil),
            topology: gl_topology_to_vk(topology),
            samples,
        }
    }
}

/// Hash vertex input state
fn hash_vertex_input(attribs: &[VertexAttribState; MAX_VERTEX_ATTRIBS]) -> u64 {
    let mut hash: u64 = 0;
    for (i, attrib) in attribs.iter().enumerate() {
        if attrib.enabled {
            hash ^= 1 << i;
            hash ^= (attrib.size as u64) << (16 + i * 4);
            hash ^= (attrib.type_ as u64) << 32;
            hash = hash.wrapping_mul(0x517cc1b727220a95);
        }
    }
    hash
}

/// Hash rasterization state
fn hash_raster_state(raster: &RasterState) -> u64 {
    let mut hash: u64 = 0;
    if raster.cull_enabled {
        hash |= 1;
    }
    hash |= (raster.cull_face as u64) << 8;
    hash |= (raster.front_face as u64) << 16;
    hash |= (raster.polygon_mode as u64) << 24;
    if raster.polygon_offset_fill {
        hash |= 1 << 32;
    }
    if raster.scissor_enabled {
        hash |= 1 << 33;
    }
    if raster.discard {
        hash |= 1 << 34;
    }
    hash
}

/// Hash blend state
fn hash_blend_state(blend: &BlendState) -> u64 {
    let mut hash: u64 = 0;
    if blend.enabled {
        hash |= 1;
    }
    hash |= (blend.factor.src_rgb as u64) << 8;
    hash |= (blend.factor.dst_rgb as u64) << 16;
    hash |= (blend.factor.src_alpha as u64) << 24;
    hash |= (blend.factor.dst_alpha as u64) << 32;
    hash |= (blend.equation.rgb as u64) << 40;
    hash |= (blend.equation.alpha as u64) << 48;
    for (i, &enabled) in blend.write_mask.iter().enumerate() {
        if enabled {
            hash |= 1 << (56 + i);
        }
    }
    hash
}

/// Hash depth-stencil state
fn hash_depth_stencil_state(depth: &DepthState, stencil: &StencilState) -> u64 {
    let mut hash: u64 = 0;
    if depth.test_enabled {
        hash |= 1;
    }
    if depth.write_enabled {
        hash |= 2;
    }
    hash |= (depth.func as u64) << 8;
    if stencil.enabled {
        hash |= 1 << 16;
    }
    hash |= (stencil.front.func.func as u64) << 24;
    hash |= (stencil.back.func.func as u64) << 32;
    hash
}

// =============================================================================
// TOPOLOGY TRANSLATION
// =============================================================================

/// Translate GL primitive type to Vulkan topology
pub fn gl_topology_to_vk(mode: GLenum) -> u32 {
    match mode {
        GL_POINTS => 0,                   // VK_PRIMITIVE_TOPOLOGY_POINT_LIST
        GL_LINES => 1,                    // VK_PRIMITIVE_TOPOLOGY_LINE_LIST
        GL_LINE_LOOP => 2,                // VK_PRIMITIVE_TOPOLOGY_LINE_STRIP (approximate)
        GL_LINE_STRIP => 2,               // VK_PRIMITIVE_TOPOLOGY_LINE_STRIP
        GL_TRIANGLES => 3,                // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST
        GL_TRIANGLE_STRIP => 4,           // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_STRIP
        GL_TRIANGLE_FAN => 5,             // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_FAN
        GL_LINES_ADJACENCY => 6,          // VK_PRIMITIVE_TOPOLOGY_LINE_LIST_WITH_ADJACENCY
        GL_LINE_STRIP_ADJACENCY => 7,     // VK_PRIMITIVE_TOPOLOGY_LINE_STRIP_WITH_ADJACENCY
        GL_TRIANGLES_ADJACENCY => 8,      // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST_WITH_ADJACENCY
        GL_TRIANGLE_STRIP_ADJACENCY => 9, // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_STRIP_WITH_ADJACENCY
        GL_PATCHES => 10,                 // VK_PRIMITIVE_TOPOLOGY_PATCH_LIST
        _ => 3,                           // Default to triangles
    }
}

/// Check if topology needs special handling
pub fn topology_needs_restart(mode: GLenum) -> bool {
    matches!(mode, GL_LINE_LOOP | GL_TRIANGLE_FAN)
}

// =============================================================================
// BLEND TRANSLATION
// =============================================================================

/// Translate GL blend factor to Vulkan
pub fn gl_blend_factor_to_vk(factor: GLenum) -> u32 {
    match factor {
        GL_ZERO => 0,                      // VK_BLEND_FACTOR_ZERO
        GL_ONE => 1,                       // VK_BLEND_FACTOR_ONE
        GL_SRC_COLOR => 2,                 // VK_BLEND_FACTOR_SRC_COLOR
        GL_ONE_MINUS_SRC_COLOR => 3,       // VK_BLEND_FACTOR_ONE_MINUS_SRC_COLOR
        GL_DST_COLOR => 4,                 // VK_BLEND_FACTOR_DST_COLOR
        GL_ONE_MINUS_DST_COLOR => 5,       // VK_BLEND_FACTOR_ONE_MINUS_DST_COLOR
        GL_SRC_ALPHA => 6,                 // VK_BLEND_FACTOR_SRC_ALPHA
        GL_ONE_MINUS_SRC_ALPHA => 7,       // VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA
        GL_DST_ALPHA => 8,                 // VK_BLEND_FACTOR_DST_ALPHA
        GL_ONE_MINUS_DST_ALPHA => 9,       // VK_BLEND_FACTOR_ONE_MINUS_DST_ALPHA
        GL_CONSTANT_COLOR => 10,           // VK_BLEND_FACTOR_CONSTANT_COLOR
        GL_ONE_MINUS_CONSTANT_COLOR => 11, // VK_BLEND_FACTOR_ONE_MINUS_CONSTANT_COLOR
        GL_CONSTANT_ALPHA => 12,           // VK_BLEND_FACTOR_CONSTANT_ALPHA
        GL_ONE_MINUS_CONSTANT_ALPHA => 13, // VK_BLEND_FACTOR_ONE_MINUS_CONSTANT_ALPHA
        GL_SRC_ALPHA_SATURATE => 14,       // VK_BLEND_FACTOR_SRC_ALPHA_SATURATE
        _ => 0,
    }
}

/// Translate GL blend equation to Vulkan
pub fn gl_blend_op_to_vk(equation: GLenum) -> u32 {
    match equation {
        GL_FUNC_ADD => 0,              // VK_BLEND_OP_ADD
        GL_FUNC_SUBTRACT => 1,         // VK_BLEND_OP_SUBTRACT
        GL_FUNC_REVERSE_SUBTRACT => 2, // VK_BLEND_OP_REVERSE_SUBTRACT
        GL_MIN => 3,                   // VK_BLEND_OP_MIN
        GL_MAX => 4,                   // VK_BLEND_OP_MAX
        _ => 0,
    }
}

// =============================================================================
// COMPARISON FUNCTION TRANSLATION
// =============================================================================

/// Translate GL compare function to Vulkan
pub fn gl_compare_op_to_vk(func: GLenum) -> u32 {
    match func {
        GL_NEVER => 0,    // VK_COMPARE_OP_NEVER
        GL_LESS => 1,     // VK_COMPARE_OP_LESS
        GL_EQUAL => 2,    // VK_COMPARE_OP_EQUAL
        GL_LEQUAL => 3,   // VK_COMPARE_OP_LESS_OR_EQUAL
        GL_GREATER => 4,  // VK_COMPARE_OP_GREATER
        GL_NOTEQUAL => 5, // VK_COMPARE_OP_NOT_EQUAL
        GL_GEQUAL => 6,   // VK_COMPARE_OP_GREATER_OR_EQUAL
        GL_ALWAYS => 7,   // VK_COMPARE_OP_ALWAYS
        _ => 1,           // Default to LESS
    }
}

// =============================================================================
// STENCIL OPERATION TRANSLATION
// =============================================================================

/// GL stencil operation constants
const GL_STENCIL_KEEP: GLenum = 0x1E00;
const GL_STENCIL_ZERO: GLenum = 0x0000;
const GL_STENCIL_REPLACE: GLenum = 0x1E01;
const GL_STENCIL_INCR: GLenum = 0x1E02;
const GL_STENCIL_INCR_WRAP: GLenum = 0x8507;
const GL_STENCIL_DECR: GLenum = 0x1E03;
const GL_STENCIL_DECR_WRAP: GLenum = 0x8508;
const GL_STENCIL_INVERT: GLenum = 0x150A;

/// Translate GL stencil operation to Vulkan
pub fn gl_stencil_op_to_vk(op: GLenum) -> u32 {
    match op {
        GL_STENCIL_KEEP => 0,      // VK_STENCIL_OP_KEEP
        GL_STENCIL_ZERO => 1,      // VK_STENCIL_OP_ZERO
        GL_STENCIL_REPLACE => 2,   // VK_STENCIL_OP_REPLACE
        GL_STENCIL_INCR => 3,      // VK_STENCIL_OP_INCREMENT_AND_CLAMP
        GL_STENCIL_DECR => 4,      // VK_STENCIL_OP_DECREMENT_AND_CLAMP
        GL_STENCIL_INVERT => 5,    // VK_STENCIL_OP_INVERT
        GL_STENCIL_INCR_WRAP => 6, // VK_STENCIL_OP_INCREMENT_AND_WRAP
        GL_STENCIL_DECR_WRAP => 7, // VK_STENCIL_OP_DECREMENT_AND_WRAP
        _ => 0,
    }
}

// =============================================================================
// CULL MODE TRANSLATION
// =============================================================================

/// Translate GL cull face to Vulkan
pub fn gl_cull_mode_to_vk(face: GLenum, enabled: bool) -> u32 {
    if !enabled {
        return 0; // VK_CULL_MODE_NONE
    }
    match face {
        GL_FRONT => 1,          // VK_CULL_MODE_FRONT_BIT
        GL_BACK => 2,           // VK_CULL_MODE_BACK_BIT
        GL_FRONT_AND_BACK => 3, // VK_CULL_MODE_FRONT_AND_BACK
        _ => 0,
    }
}

/// Translate GL front face to Vulkan
pub fn gl_front_face_to_vk(mode: GLenum) -> u32 {
    match mode {
        GL_CCW => 0, // VK_FRONT_FACE_COUNTER_CLOCKWISE
        GL_CW => 1,  // VK_FRONT_FACE_CLOCKWISE
        _ => 0,
    }
}

// =============================================================================
// POLYGON MODE TRANSLATION
// =============================================================================

/// GL polygon mode constants
const GL_POLYGON_FILL: GLenum = 0x1B02;
const GL_POLYGON_LINE: GLenum = 0x1B01;
const GL_POLYGON_POINT: GLenum = 0x1B00;

/// Translate GL polygon mode to Vulkan
pub fn gl_polygon_mode_to_vk(mode: GLenum) -> u32 {
    match mode {
        GL_POLYGON_FILL => 0,  // VK_POLYGON_MODE_FILL
        GL_POLYGON_LINE => 1,  // VK_POLYGON_MODE_LINE
        GL_POLYGON_POINT => 2, // VK_POLYGON_MODE_POINT
        _ => 0,
    }
}

// =============================================================================
// PIPELINE CACHE
// =============================================================================

/// Cached pipeline entry
#[derive(Debug)]
pub struct CachedPipeline {
    /// Vulkan pipeline handle
    pub vk_pipeline: u64,
    /// Pipeline layout handle
    pub vk_pipeline_layout: u64,
    /// Last used frame number (for LRU eviction)
    pub last_used_frame: u64,
}

/// Pipeline cache
pub struct PipelineCache {
    /// Graphics pipelines
    graphics: BTreeMap<u64, CachedPipeline>,
    /// Compute pipelines
    compute: BTreeMap<u32, CachedPipeline>,
    /// Current frame number
    frame_number: u64,
    /// Maximum cached pipelines
    max_cached: usize,
}

impl PipelineCache {
    /// Create a new pipeline cache
    pub fn new(max_cached: usize) -> Self {
        Self {
            graphics: BTreeMap::new(),
            compute: BTreeMap::new(),
            frame_number: 0,
            max_cached,
        }
    }

    /// Increment frame counter
    pub fn next_frame(&mut self) {
        self.frame_number += 1;
    }

    /// Get or create graphics pipeline
    pub fn get_graphics_pipeline(&mut self, key: &PipelineKey) -> Option<&CachedPipeline> {
        let hash = Self::hash_key(key);
        if let Some(pipeline) = self.graphics.get_mut(&hash) {
            pipeline.last_used_frame = self.frame_number;
            return Some(pipeline);
        }
        None
    }

    /// Insert graphics pipeline
    pub fn insert_graphics_pipeline(
        &mut self,
        key: &PipelineKey,
        vk_pipeline: u64,
        vk_pipeline_layout: u64,
    ) {
        let hash = Self::hash_key(key);

        // Evict if at capacity
        if self.graphics.len() >= self.max_cached {
            self.evict_oldest_graphics();
        }

        self.graphics.insert(hash, CachedPipeline {
            vk_pipeline,
            vk_pipeline_layout,
            last_used_frame: self.frame_number,
        });
    }

    /// Get or create compute pipeline
    pub fn get_compute_pipeline(&mut self, program: u32) -> Option<&CachedPipeline> {
        if let Some(pipeline) = self.compute.get_mut(&program) {
            pipeline.last_used_frame = self.frame_number;
            return Some(pipeline);
        }
        None
    }

    /// Insert compute pipeline
    pub fn insert_compute_pipeline(
        &mut self,
        program: u32,
        vk_pipeline: u64,
        vk_pipeline_layout: u64,
    ) {
        self.compute.insert(program, CachedPipeline {
            vk_pipeline,
            vk_pipeline_layout,
            last_used_frame: self.frame_number,
        });
    }

    /// Evict oldest graphics pipeline
    fn evict_oldest_graphics(&mut self) {
        if let Some((&oldest_key, _)) = self.graphics.iter().min_by_key(|(_, p)| p.last_used_frame)
        {
            // TODO: Destroy Vulkan pipeline
            self.graphics.remove(&oldest_key);
        }
    }

    /// Hash pipeline key to u64
    fn hash_key(key: &PipelineKey) -> u64 {
        let mut hash: u64 = key.program as u64;
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.render_pass_hash);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.vertex_input_hash);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.raster_hash);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.blend_hash);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.depth_stencil_hash);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.topology as u64);
        hash = hash
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(key.samples as u64);
        hash
    }

    /// Clear all cached pipelines
    pub fn clear(&mut self) {
        // TODO: Destroy all Vulkan pipelines
        self.graphics.clear();
        self.compute.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.graphics.len(), self.compute.len())
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new(1024) // Default to 1024 cached pipelines
    }
}

// =============================================================================
// DYNAMIC STATE
// =============================================================================

/// Vulkan dynamic states used by GL
#[derive(Debug, Clone, Copy)]
pub struct DynamicStates {
    /// Viewport is dynamic
    pub viewport: bool,
    /// Scissor is dynamic
    pub scissor: bool,
    /// Line width is dynamic
    pub line_width: bool,
    /// Depth bias is dynamic
    pub depth_bias: bool,
    /// Blend constants are dynamic
    pub blend_constants: bool,
    /// Depth bounds are dynamic
    pub depth_bounds: bool,
    /// Stencil compare mask is dynamic
    pub stencil_compare_mask: bool,
    /// Stencil write mask is dynamic
    pub stencil_write_mask: bool,
    /// Stencil reference is dynamic
    pub stencil_reference: bool,
}

impl Default for DynamicStates {
    fn default() -> Self {
        Self {
            viewport: true,
            scissor: true,
            line_width: true,
            depth_bias: true,
            blend_constants: true,
            depth_bounds: false, // Not commonly used
            stencil_compare_mask: true,
            stencil_write_mask: true,
            stencil_reference: true,
        }
    }
}

impl DynamicStates {
    /// Get list of Vulkan dynamic states
    pub fn to_vk_states(&self) -> Vec<u32> {
        let mut states = Vec::new();
        if self.viewport {
            states.push(0); // VK_DYNAMIC_STATE_VIEWPORT
        }
        if self.scissor {
            states.push(1); // VK_DYNAMIC_STATE_SCISSOR
        }
        if self.line_width {
            states.push(2); // VK_DYNAMIC_STATE_LINE_WIDTH
        }
        if self.depth_bias {
            states.push(3); // VK_DYNAMIC_STATE_DEPTH_BIAS
        }
        if self.blend_constants {
            states.push(4); // VK_DYNAMIC_STATE_BLEND_CONSTANTS
        }
        if self.depth_bounds {
            states.push(5); // VK_DYNAMIC_STATE_DEPTH_BOUNDS
        }
        if self.stencil_compare_mask {
            states.push(6); // VK_DYNAMIC_STATE_STENCIL_COMPARE_MASK
        }
        if self.stencil_write_mask {
            states.push(7); // VK_DYNAMIC_STATE_STENCIL_WRITE_MASK
        }
        if self.stencil_reference {
            states.push(8); // VK_DYNAMIC_STATE_STENCIL_REFERENCE
        }
        states
    }
}
