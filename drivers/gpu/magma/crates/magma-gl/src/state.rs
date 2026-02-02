//! # OpenGL State Machine
//!
//! Tracks OpenGL state and generates Vulkan commands when state changes.
//! Uses dirty flags to batch state changes efficiently.

use crate::enums::*;
use crate::types::*;
use alloc::vec::Vec;
use bitflags::bitflags;

// =============================================================================
// DIRTY FLAGS
// =============================================================================

bitflags! {
    /// Flags indicating which state categories have changed since last flush.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DirtyFlags: u32 {
        /// Viewport has changed
        const VIEWPORT = 1 << 0;
        /// Scissor has changed
        const SCISSOR = 1 << 1;
        /// Blend state has changed
        const BLEND = 1 << 2;
        /// Depth state has changed
        const DEPTH = 1 << 3;
        /// Stencil state has changed
        const STENCIL = 1 << 4;
        /// Rasterization state has changed
        const RASTER = 1 << 5;
        /// Vertex input state has changed
        const VERTEX_INPUT = 1 << 6;
        /// Active program has changed
        const PROGRAM = 1 << 7;
        /// Bound textures have changed
        const TEXTURES = 1 << 8;
        /// Bound buffers have changed
        const BUFFERS = 1 << 9;
        /// Framebuffer has changed
        const FRAMEBUFFER = 1 << 10;
        /// Clear color has changed
        const CLEAR_COLOR = 1 << 11;
        /// Clear depth has changed
        const CLEAR_DEPTH = 1 << 12;
        /// Clear stencil has changed
        const CLEAR_STENCIL = 1 << 13;
        /// Polygon mode has changed
        const POLYGON_MODE = 1 << 14;
        /// Line width has changed
        const LINE_WIDTH = 1 << 15;
        /// Point size has changed
        const POINT_SIZE = 1 << 16;
    }
}

impl Default for DirtyFlags {
    fn default() -> Self {
        DirtyFlags::empty()
    }
}

// =============================================================================
// VIEWPORT STATE
// =============================================================================

/// Viewport definition
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Near depth
    pub near: f32,
    /// Far depth
    pub far: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            near: 0.0,
            far: 1.0,
        }
    }
}

/// Scissor rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScissorRect {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Default for ScissorRect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: u32::MAX,
            height: u32::MAX,
        }
    }
}

// =============================================================================
// BLEND STATE
// =============================================================================

/// Blend function factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendFactor {
    /// Source RGB factor
    pub src_rgb: GLenum,
    /// Destination RGB factor
    pub dst_rgb: GLenum,
    /// Source alpha factor
    pub src_alpha: GLenum,
    /// Destination alpha factor
    pub dst_alpha: GLenum,
}

impl Default for BlendFactor {
    fn default() -> Self {
        Self {
            src_rgb: GL_ONE,
            dst_rgb: GL_ZERO,
            src_alpha: GL_ONE,
            dst_alpha: GL_ZERO,
        }
    }
}

/// Blend equation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendEquation {
    /// RGB equation
    pub rgb: GLenum,
    /// Alpha equation
    pub alpha: GLenum,
}

impl Default for BlendEquation {
    fn default() -> Self {
        Self {
            rgb: GL_FUNC_ADD,
            alpha: GL_FUNC_ADD,
        }
    }
}

/// Complete blend state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendState {
    /// Whether blending is enabled
    pub enabled: bool,
    /// Blend factors
    pub factor: BlendFactor,
    /// Blend equations
    pub equation: BlendEquation,
    /// Blend color constant
    pub color: [f32; 4],
    /// Color write mask (RGBA)
    pub write_mask: [bool; 4],
}

impl Default for BlendState {
    fn default() -> Self {
        Self {
            enabled: false,
            factor: BlendFactor::default(),
            equation: BlendEquation::default(),
            color: [0.0, 0.0, 0.0, 0.0],
            write_mask: [true, true, true, true],
        }
    }
}

// =============================================================================
// DEPTH STATE
// =============================================================================

/// Depth test state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthState {
    /// Whether depth testing is enabled
    pub test_enabled: bool,
    /// Whether depth writing is enabled
    pub write_enabled: bool,
    /// Depth comparison function
    pub func: GLenum,
    /// Depth range near
    pub range_near: f64,
    /// Depth range far
    pub range_far: f64,
    /// Depth clamp enabled
    pub clamp_enabled: bool,
}

impl Default for DepthState {
    fn default() -> Self {
        Self {
            test_enabled: false,
            write_enabled: true,
            func: GL_LESS,
            range_near: 0.0,
            range_far: 1.0,
            clamp_enabled: false,
        }
    }
}

// =============================================================================
// STENCIL STATE
// =============================================================================

/// Stencil operation set
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilOp {
    /// Stencil fail operation
    pub sfail: GLenum,
    /// Depth fail operation
    pub dpfail: GLenum,
    /// Depth pass operation
    pub dppass: GLenum,
}

impl Default for StencilOp {
    fn default() -> Self {
        Self {
            sfail: GL_KEEP,
            dpfail: GL_KEEP,
            dppass: GL_KEEP,
        }
    }
}

/// Stencil keep operation constant
const GL_KEEP: GLenum = 0x1E00;

/// Stencil function parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilFunc {
    /// Comparison function
    pub func: GLenum,
    /// Reference value
    pub ref_value: i32,
    /// Comparison mask
    pub mask: u32,
}

impl Default for StencilFunc {
    fn default() -> Self {
        Self {
            func: GL_ALWAYS,
            ref_value: 0,
            mask: u32::MAX,
        }
    }
}

/// Complete stencil state for one face
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilFaceState {
    /// Stencil operations
    pub op: StencilOp,
    /// Stencil function
    pub func: StencilFunc,
    /// Write mask
    pub write_mask: u32,
}

impl Default for StencilFaceState {
    fn default() -> Self {
        Self {
            op: StencilOp::default(),
            func: StencilFunc::default(),
            write_mask: u32::MAX,
        }
    }
}

/// Complete stencil state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilState {
    /// Whether stencil testing is enabled
    pub enabled: bool,
    /// Front face state
    pub front: StencilFaceState,
    /// Back face state
    pub back: StencilFaceState,
}

impl Default for StencilState {
    fn default() -> Self {
        Self {
            enabled: false,
            front: StencilFaceState::default(),
            back: StencilFaceState::default(),
        }
    }
}

// =============================================================================
// RASTERIZATION STATE
// =============================================================================

/// Polygon mode constant
const GL_FILL: GLenum = 0x1B02;
const GL_LINE: GLenum = 0x1B01;
const GL_POINT: GLenum = 0x1B00;

/// Rasterization state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterState {
    /// Whether face culling is enabled
    pub cull_enabled: bool,
    /// Which face to cull
    pub cull_face: GLenum,
    /// Front face winding order
    pub front_face: GLenum,
    /// Polygon mode
    pub polygon_mode: GLenum,
    /// Polygon offset enabled
    pub polygon_offset_fill: bool,
    /// Polygon offset line enabled
    pub polygon_offset_line: bool,
    /// Polygon offset point enabled
    pub polygon_offset_point: bool,
    /// Polygon offset factor
    pub polygon_offset_factor: f32,
    /// Polygon offset units
    pub polygon_offset_units: f32,
    /// Line width
    pub line_width: f32,
    /// Point size
    pub point_size: f32,
    /// Line smooth enabled
    pub line_smooth: bool,
    /// Multisample enabled
    pub multisample: bool,
    /// Scissor test enabled
    pub scissor_enabled: bool,
    /// Rasterizer discard enabled
    pub discard: bool,
}

impl Default for RasterState {
    fn default() -> Self {
        Self {
            cull_enabled: false,
            cull_face: GL_BACK,
            front_face: GL_CCW,
            polygon_mode: GL_FILL,
            polygon_offset_fill: false,
            polygon_offset_line: false,
            polygon_offset_point: false,
            polygon_offset_factor: 0.0,
            polygon_offset_units: 0.0,
            line_width: 1.0,
            point_size: 1.0,
            line_smooth: false,
            multisample: true,
            scissor_enabled: false,
            discard: false,
        }
    }
}

// =============================================================================
// CLEAR STATE
// =============================================================================

/// Clear state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearState {
    /// Clear color (RGBA)
    pub color: [f32; 4],
    /// Clear depth
    pub depth: f64,
    /// Clear stencil
    pub stencil: i32,
}

impl Default for ClearState {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            depth: 1.0,
            stencil: 0,
        }
    }
}

// =============================================================================
// BUFFER BINDINGS
// =============================================================================

/// Maximum number of vertex buffer binding points
pub const MAX_VERTEX_ATTRIBS: usize = 16;
/// Maximum number of texture units
pub const MAX_TEXTURE_UNITS: usize = 32;
/// Maximum number of uniform buffer binding points
pub const MAX_UNIFORM_BUFFER_BINDINGS: usize = 36;
/// Maximum number of SSBO binding points
pub const MAX_SSBO_BINDINGS: usize = 16;

/// Buffer binding state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BufferBinding {
    /// Bound buffer handle
    pub buffer: BufferHandle,
    /// Offset into buffer
    pub offset: usize,
    /// Size of binding (0 = whole buffer)
    pub size: usize,
}

/// Buffer binding points
#[derive(Debug, Clone, Default)]
pub struct BufferBindings {
    /// Array buffer binding
    pub array_buffer: BufferHandle,
    /// Element array buffer binding
    pub element_array_buffer: BufferHandle,
    /// Uniform buffer bindings
    pub uniform_buffers: [BufferBinding; MAX_UNIFORM_BUFFER_BINDINGS],
    /// Shader storage buffer bindings
    pub shader_storage_buffers: [BufferBinding; MAX_SSBO_BINDINGS],
    /// Copy read buffer
    pub copy_read_buffer: BufferHandle,
    /// Copy write buffer
    pub copy_write_buffer: BufferHandle,
    /// Pixel pack buffer
    pub pixel_pack_buffer: BufferHandle,
    /// Pixel unpack buffer
    pub pixel_unpack_buffer: BufferHandle,
    /// Draw indirect buffer
    pub draw_indirect_buffer: BufferHandle,
    /// Dispatch indirect buffer
    pub dispatch_indirect_buffer: BufferHandle,
}

// =============================================================================
// TEXTURE BINDINGS
// =============================================================================

/// Texture unit binding
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextureUnitBinding {
    /// Bound 1D texture
    pub texture_1d: TextureHandle,
    /// Bound 2D texture
    pub texture_2d: TextureHandle,
    /// Bound 3D texture
    pub texture_3d: TextureHandle,
    /// Bound cube map texture
    pub texture_cube_map: TextureHandle,
    /// Bound 1D array texture
    pub texture_1d_array: TextureHandle,
    /// Bound 2D array texture
    pub texture_2d_array: TextureHandle,
    /// Bound 2D multisample texture
    pub texture_2d_multisample: TextureHandle,
    /// Bound 2D multisample array texture
    pub texture_2d_multisample_array: TextureHandle,
    /// Bound rectangle texture
    pub texture_rectangle: TextureHandle,
    /// Bound buffer texture
    pub texture_buffer: TextureHandle,
    /// Bound cube map array texture
    pub texture_cube_map_array: TextureHandle,
}

/// Texture binding state
#[derive(Debug, Clone)]
pub struct TextureBindings {
    /// Active texture unit
    pub active_unit: u32,
    /// Texture units
    pub units: [TextureUnitBinding; MAX_TEXTURE_UNITS],
}

impl Default for TextureBindings {
    fn default() -> Self {
        Self {
            active_unit: 0,
            units: [TextureUnitBinding::default(); MAX_TEXTURE_UNITS],
        }
    }
}

// =============================================================================
// VERTEX ARRAY STATE
// =============================================================================

/// Vertex attribute state
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VertexAttribState {
    /// Whether this attribute is enabled
    pub enabled: bool,
    /// Number of components
    pub size: i32,
    /// Component type
    pub type_: GLenum,
    /// Whether to normalize integer values
    pub normalized: bool,
    /// Stride between consecutive attributes
    pub stride: i32,
    /// Offset in buffer
    pub offset: usize,
    /// Source buffer binding index
    pub buffer_binding: u32,
    /// Attribute divisor for instancing
    pub divisor: u32,
}

/// Vertex array object state
#[derive(Debug, Clone)]
pub struct VaoState {
    /// Currently bound VAO
    pub bound_vao: VaoHandle,
    /// Element buffer bound to VAO
    pub element_buffer: BufferHandle,
    /// Vertex attribute states
    pub attribs: [VertexAttribState; MAX_VERTEX_ATTRIBS],
    /// Buffer bindings per attrib
    pub attrib_buffers: [BufferHandle; MAX_VERTEX_ATTRIBS],
}

impl Default for VaoState {
    fn default() -> Self {
        Self {
            bound_vao: VaoHandle::default(),
            element_buffer: BufferHandle::default(),
            attribs: [VertexAttribState::default(); MAX_VERTEX_ATTRIBS],
            attrib_buffers: [BufferHandle::default(); MAX_VERTEX_ATTRIBS],
        }
    }
}

// =============================================================================
// PROGRAM STATE
// =============================================================================

/// Program binding state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProgramState {
    /// Currently bound program
    pub current_program: ProgramHandle,
}

// =============================================================================
// FRAMEBUFFER STATE
// =============================================================================

/// Framebuffer binding state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FramebufferState {
    /// Read framebuffer binding
    pub read_framebuffer: FramebufferHandle,
    /// Draw framebuffer binding
    pub draw_framebuffer: FramebufferHandle,
    /// Renderbuffer binding
    pub renderbuffer: RenderbufferHandle,
}

// =============================================================================
// COMPLETE GL STATE
// =============================================================================

/// Complete OpenGL state machine
#[derive(Debug, Clone)]
pub struct GlState {
    /// Dirty flags indicating changed state
    pub dirty: DirtyFlags,
    /// Current viewport
    pub viewport: Viewport,
    /// Current scissor rect
    pub scissor: ScissorRect,
    /// Blend state
    pub blend: BlendState,
    /// Depth state
    pub depth: DepthState,
    /// Stencil state
    pub stencil: StencilState,
    /// Rasterization state
    pub raster: RasterState,
    /// Clear state
    pub clear: ClearState,
    /// Buffer bindings
    pub buffers: BufferBindings,
    /// Texture bindings
    pub textures: TextureBindings,
    /// Vertex array state
    pub vao: VaoState,
    /// Program state
    pub program: ProgramState,
    /// Framebuffer state
    pub framebuffer: FramebufferState,
    /// Current error code
    pub error: GLenum,
}

impl Default for GlState {
    fn default() -> Self {
        Self {
            dirty: DirtyFlags::all(), // All dirty on init
            viewport: Viewport::default(),
            scissor: ScissorRect::default(),
            blend: BlendState::default(),
            depth: DepthState::default(),
            stencil: StencilState::default(),
            raster: RasterState::default(),
            clear: ClearState::default(),
            buffers: BufferBindings::default(),
            textures: TextureBindings::default(),
            vao: VaoState::default(),
            program: ProgramState::default(),
            framebuffer: FramebufferState::default(),
            error: GL_NO_ERROR,
        }
    }
}

impl GlState {
    /// Create a new GL state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a state category as dirty
    #[inline]
    pub fn mark_dirty(&mut self, flags: DirtyFlags) {
        self.dirty |= flags;
    }

    /// Check if a state category is dirty
    #[inline]
    pub fn is_dirty(&self, flags: DirtyFlags) -> bool {
        self.dirty.contains(flags)
    }

    /// Clear dirty flags
    #[inline]
    pub fn clear_dirty(&mut self, flags: DirtyFlags) {
        self.dirty.remove(flags);
    }

    /// Clear all dirty flags
    #[inline]
    pub fn clear_all_dirty(&mut self) {
        self.dirty = DirtyFlags::empty();
    }

    /// Set error if no error is currently set
    #[inline]
    pub fn set_error(&mut self, error: GLenum) {
        if self.error == GL_NO_ERROR {
            self.error = error;
        }
    }

    /// Get and clear the current error
    #[inline]
    pub fn get_error(&mut self) -> GLenum {
        let error = self.error;
        self.error = GL_NO_ERROR;
        error
    }

    // =========================================================================
    // VIEWPORT OPERATIONS
    // =========================================================================

    /// Set viewport
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if self.viewport.x != x
            || self.viewport.y != y
            || self.viewport.width != width
            || self.viewport.height != height
        {
            self.viewport.x = x;
            self.viewport.y = y;
            self.viewport.width = width;
            self.viewport.height = height;
            self.mark_dirty(DirtyFlags::VIEWPORT);
        }
    }

    /// Set depth range
    pub fn set_depth_range(&mut self, near: f64, far: f64) {
        if self.viewport.near != near as f32 || self.viewport.far != far as f32 {
            self.viewport.near = near as f32;
            self.viewport.far = far as f32;
            self.mark_dirty(DirtyFlags::VIEWPORT);
        }
    }

    /// Set scissor rect
    pub fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let rect = ScissorRect {
            x,
            y,
            width,
            height,
        };
        if self.scissor != rect {
            self.scissor = rect;
            self.mark_dirty(DirtyFlags::SCISSOR);
        }
    }

    // =========================================================================
    // BLEND OPERATIONS
    // =========================================================================

    /// Enable or disable blending
    pub fn set_blend_enabled(&mut self, enabled: bool) {
        if self.blend.enabled != enabled {
            self.blend.enabled = enabled;
            self.mark_dirty(DirtyFlags::BLEND);
        }
    }

    /// Set blend function
    pub fn set_blend_func(&mut self, src: GLenum, dst: GLenum) {
        self.set_blend_func_separate(src, dst, src, dst);
    }

    /// Set blend function with separate alpha
    pub fn set_blend_func_separate(
        &mut self,
        src_rgb: GLenum,
        dst_rgb: GLenum,
        src_alpha: GLenum,
        dst_alpha: GLenum,
    ) {
        let factor = BlendFactor {
            src_rgb,
            dst_rgb,
            src_alpha,
            dst_alpha,
        };
        if self.blend.factor != factor {
            self.blend.factor = factor;
            self.mark_dirty(DirtyFlags::BLEND);
        }
    }

    /// Set blend equation
    pub fn set_blend_equation(&mut self, mode: GLenum) {
        self.set_blend_equation_separate(mode, mode);
    }

    /// Set blend equation with separate alpha
    pub fn set_blend_equation_separate(&mut self, rgb: GLenum, alpha: GLenum) {
        let eq = BlendEquation { rgb, alpha };
        if self.blend.equation != eq {
            self.blend.equation = eq;
            self.mark_dirty(DirtyFlags::BLEND);
        }
    }

    /// Set blend color
    pub fn set_blend_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        let color = [r, g, b, a];
        if self.blend.color != color {
            self.blend.color = color;
            self.mark_dirty(DirtyFlags::BLEND);
        }
    }

    /// Set color write mask
    pub fn set_color_mask(&mut self, r: bool, g: bool, b: bool, a: bool) {
        let mask = [r, g, b, a];
        if self.blend.write_mask != mask {
            self.blend.write_mask = mask;
            self.mark_dirty(DirtyFlags::BLEND);
        }
    }

    // =========================================================================
    // DEPTH OPERATIONS
    // =========================================================================

    /// Enable or disable depth testing
    pub fn set_depth_test_enabled(&mut self, enabled: bool) {
        if self.depth.test_enabled != enabled {
            self.depth.test_enabled = enabled;
            self.mark_dirty(DirtyFlags::DEPTH);
        }
    }

    /// Enable or disable depth writing
    pub fn set_depth_mask(&mut self, enabled: bool) {
        if self.depth.write_enabled != enabled {
            self.depth.write_enabled = enabled;
            self.mark_dirty(DirtyFlags::DEPTH);
        }
    }

    /// Set depth function
    pub fn set_depth_func(&mut self, func: GLenum) {
        if self.depth.func != func {
            self.depth.func = func;
            self.mark_dirty(DirtyFlags::DEPTH);
        }
    }

    // =========================================================================
    // STENCIL OPERATIONS
    // =========================================================================

    /// Enable or disable stencil testing
    pub fn set_stencil_test_enabled(&mut self, enabled: bool) {
        if self.stencil.enabled != enabled {
            self.stencil.enabled = enabled;
            self.mark_dirty(DirtyFlags::STENCIL);
        }
    }

    /// Set stencil function for both faces
    pub fn set_stencil_func(&mut self, func: GLenum, ref_value: i32, mask: u32) {
        self.set_stencil_func_separate(GL_FRONT_AND_BACK, func, ref_value, mask);
    }

    /// Set stencil function for specific face
    pub fn set_stencil_func_separate(
        &mut self,
        face: GLenum,
        func: GLenum,
        ref_value: i32,
        mask: u32,
    ) {
        let stencil_func = StencilFunc {
            func,
            ref_value,
            mask,
        };
        let changed = match face {
            GL_FRONT => {
                if self.stencil.front.func != stencil_func {
                    self.stencil.front.func = stencil_func;
                    true
                } else {
                    false
                }
            }
            GL_BACK => {
                if self.stencil.back.func != stencil_func {
                    self.stencil.back.func = stencil_func;
                    true
                } else {
                    false
                }
            }
            GL_FRONT_AND_BACK => {
                let front_changed = self.stencil.front.func != stencil_func;
                let back_changed = self.stencil.back.func != stencil_func;
                if front_changed {
                    self.stencil.front.func = stencil_func;
                }
                if back_changed {
                    self.stencil.back.func = stencil_func;
                }
                front_changed || back_changed
            }
            _ => false,
        };
        if changed {
            self.mark_dirty(DirtyFlags::STENCIL);
        }
    }

    /// Set stencil operations for both faces
    pub fn set_stencil_op(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
        self.set_stencil_op_separate(GL_FRONT_AND_BACK, sfail, dpfail, dppass);
    }

    /// Set stencil operations for specific face
    pub fn set_stencil_op_separate(
        &mut self,
        face: GLenum,
        sfail: GLenum,
        dpfail: GLenum,
        dppass: GLenum,
    ) {
        let op = StencilOp {
            sfail,
            dpfail,
            dppass,
        };
        let changed = match face {
            GL_FRONT => {
                if self.stencil.front.op != op {
                    self.stencil.front.op = op;
                    true
                } else {
                    false
                }
            }
            GL_BACK => {
                if self.stencil.back.op != op {
                    self.stencil.back.op = op;
                    true
                } else {
                    false
                }
            }
            GL_FRONT_AND_BACK => {
                let front_changed = self.stencil.front.op != op;
                let back_changed = self.stencil.back.op != op;
                if front_changed {
                    self.stencil.front.op = op;
                }
                if back_changed {
                    self.stencil.back.op = op;
                }
                front_changed || back_changed
            }
            _ => false,
        };
        if changed {
            self.mark_dirty(DirtyFlags::STENCIL);
        }
    }

    /// Set stencil write mask
    pub fn set_stencil_mask(&mut self, mask: u32) {
        self.set_stencil_mask_separate(GL_FRONT_AND_BACK, mask);
    }

    /// Set stencil write mask for specific face
    pub fn set_stencil_mask_separate(&mut self, face: GLenum, mask: u32) {
        let changed = match face {
            GL_FRONT => {
                if self.stencil.front.write_mask != mask {
                    self.stencil.front.write_mask = mask;
                    true
                } else {
                    false
                }
            }
            GL_BACK => {
                if self.stencil.back.write_mask != mask {
                    self.stencil.back.write_mask = mask;
                    true
                } else {
                    false
                }
            }
            GL_FRONT_AND_BACK => {
                let front_changed = self.stencil.front.write_mask != mask;
                let back_changed = self.stencil.back.write_mask != mask;
                if front_changed {
                    self.stencil.front.write_mask = mask;
                }
                if back_changed {
                    self.stencil.back.write_mask = mask;
                }
                front_changed || back_changed
            }
            _ => false,
        };
        if changed {
            self.mark_dirty(DirtyFlags::STENCIL);
        }
    }

    // =========================================================================
    // RASTERIZATION OPERATIONS
    // =========================================================================

    /// Enable or disable face culling
    pub fn set_cull_face_enabled(&mut self, enabled: bool) {
        if self.raster.cull_enabled != enabled {
            self.raster.cull_enabled = enabled;
            self.mark_dirty(DirtyFlags::RASTER);
        }
    }

    /// Set which face to cull
    pub fn set_cull_face(&mut self, face: GLenum) {
        if self.raster.cull_face != face {
            self.raster.cull_face = face;
            self.mark_dirty(DirtyFlags::RASTER);
        }
    }

    /// Set front face winding order
    pub fn set_front_face(&mut self, mode: GLenum) {
        if self.raster.front_face != mode {
            self.raster.front_face = mode;
            self.mark_dirty(DirtyFlags::RASTER);
        }
    }

    /// Set polygon mode
    pub fn set_polygon_mode(&mut self, face: GLenum, mode: GLenum) {
        // OpenGL specifies per-face but Vulkan uses single mode
        // We use the mode for all faces
        if self.raster.polygon_mode != mode {
            self.raster.polygon_mode = mode;
            self.mark_dirty(DirtyFlags::POLYGON_MODE);
        }
    }

    /// Set line width
    pub fn set_line_width(&mut self, width: f32) {
        if self.raster.line_width != width {
            self.raster.line_width = width;
            self.mark_dirty(DirtyFlags::LINE_WIDTH);
        }
    }

    /// Set point size
    pub fn set_point_size(&mut self, size: f32) {
        if self.raster.point_size != size {
            self.raster.point_size = size;
            self.mark_dirty(DirtyFlags::POINT_SIZE);
        }
    }

    /// Set polygon offset
    pub fn set_polygon_offset(&mut self, factor: f32, units: f32) {
        if self.raster.polygon_offset_factor != factor || self.raster.polygon_offset_units != units
        {
            self.raster.polygon_offset_factor = factor;
            self.raster.polygon_offset_units = units;
            self.mark_dirty(DirtyFlags::RASTER);
        }
    }

    // =========================================================================
    // CLEAR OPERATIONS
    // =========================================================================

    /// Set clear color
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        let color = [r, g, b, a];
        if self.clear.color != color {
            self.clear.color = color;
            self.mark_dirty(DirtyFlags::CLEAR_COLOR);
        }
    }

    /// Set clear depth
    pub fn set_clear_depth(&mut self, depth: f64) {
        if self.clear.depth != depth {
            self.clear.depth = depth;
            self.mark_dirty(DirtyFlags::CLEAR_DEPTH);
        }
    }

    /// Set clear stencil
    pub fn set_clear_stencil(&mut self, stencil: i32) {
        if self.clear.stencil != stencil {
            self.clear.stencil = stencil;
            self.mark_dirty(DirtyFlags::CLEAR_STENCIL);
        }
    }

    // =========================================================================
    // BUFFER BINDING OPERATIONS
    // =========================================================================

    /// Bind buffer to target
    pub fn bind_buffer(&mut self, target: GLenum, buffer: BufferHandle) {
        let changed = match target {
            GL_ARRAY_BUFFER => {
                if self.buffers.array_buffer != buffer {
                    self.buffers.array_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_ELEMENT_ARRAY_BUFFER => {
                if self.buffers.element_array_buffer != buffer {
                    self.buffers.element_array_buffer = buffer;
                    // Also update VAO element buffer
                    self.vao.element_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_COPY_READ_BUFFER => {
                if self.buffers.copy_read_buffer != buffer {
                    self.buffers.copy_read_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_COPY_WRITE_BUFFER => {
                if self.buffers.copy_write_buffer != buffer {
                    self.buffers.copy_write_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_PIXEL_PACK_BUFFER => {
                if self.buffers.pixel_pack_buffer != buffer {
                    self.buffers.pixel_pack_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_PIXEL_UNPACK_BUFFER => {
                if self.buffers.pixel_unpack_buffer != buffer {
                    self.buffers.pixel_unpack_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_DRAW_INDIRECT_BUFFER => {
                if self.buffers.draw_indirect_buffer != buffer {
                    self.buffers.draw_indirect_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            GL_DISPATCH_INDIRECT_BUFFER => {
                if self.buffers.dispatch_indirect_buffer != buffer {
                    self.buffers.dispatch_indirect_buffer = buffer;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if changed {
            self.mark_dirty(DirtyFlags::BUFFERS);
        }
    }

    /// Bind buffer to indexed target
    pub fn bind_buffer_base(&mut self, target: GLenum, index: u32, buffer: BufferHandle) {
        self.bind_buffer_range(target, index, buffer, 0, 0);
    }

    /// Bind buffer range to indexed target
    pub fn bind_buffer_range(
        &mut self,
        target: GLenum,
        index: u32,
        buffer: BufferHandle,
        offset: usize,
        size: usize,
    ) {
        let binding = BufferBinding {
            buffer,
            offset,
            size,
        };
        let changed = match target {
            GL_UNIFORM_BUFFER => {
                if (index as usize) < MAX_UNIFORM_BUFFER_BINDINGS {
                    if self.buffers.uniform_buffers[index as usize] != binding {
                        self.buffers.uniform_buffers[index as usize] = binding;
                        true
                    } else {
                        false
                    }
                } else {
                    self.set_error(GL_INVALID_VALUE);
                    false
                }
            }
            GL_SHADER_STORAGE_BUFFER => {
                if (index as usize) < MAX_SSBO_BINDINGS {
                    if self.buffers.shader_storage_buffers[index as usize] != binding {
                        self.buffers.shader_storage_buffers[index as usize] = binding;
                        true
                    } else {
                        false
                    }
                } else {
                    self.set_error(GL_INVALID_VALUE);
                    false
                }
            }
            _ => false,
        };
        if changed {
            self.mark_dirty(DirtyFlags::BUFFERS);
        }
    }

    // =========================================================================
    // TEXTURE BINDING OPERATIONS
    // =========================================================================

    /// Set active texture unit
    pub fn set_active_texture(&mut self, unit: u32) {
        if unit < MAX_TEXTURE_UNITS as u32 {
            self.textures.active_unit = unit;
        } else {
            self.set_error(GL_INVALID_ENUM);
        }
    }

    /// Bind texture to target on active unit
    pub fn bind_texture(&mut self, target: GLenum, texture: TextureHandle) {
        let unit = self.textures.active_unit as usize;
        if unit >= MAX_TEXTURE_UNITS {
            return;
        }

        let binding = &mut self.textures.units[unit];
        let changed = match target {
            GL_TEXTURE_1D => {
                if binding.texture_1d != texture {
                    binding.texture_1d = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_2D => {
                if binding.texture_2d != texture {
                    binding.texture_2d = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_3D => {
                if binding.texture_3d != texture {
                    binding.texture_3d = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_CUBE_MAP => {
                if binding.texture_cube_map != texture {
                    binding.texture_cube_map = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_1D_ARRAY => {
                if binding.texture_1d_array != texture {
                    binding.texture_1d_array = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_2D_ARRAY => {
                if binding.texture_2d_array != texture {
                    binding.texture_2d_array = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_2D_MULTISAMPLE => {
                if binding.texture_2d_multisample != texture {
                    binding.texture_2d_multisample = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_2D_MULTISAMPLE_ARRAY => {
                if binding.texture_2d_multisample_array != texture {
                    binding.texture_2d_multisample_array = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_RECTANGLE => {
                if binding.texture_rectangle != texture {
                    binding.texture_rectangle = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_BUFFER => {
                if binding.texture_buffer != texture {
                    binding.texture_buffer = texture;
                    true
                } else {
                    false
                }
            }
            GL_TEXTURE_CUBE_MAP_ARRAY => {
                if binding.texture_cube_map_array != texture {
                    binding.texture_cube_map_array = texture;
                    true
                } else {
                    false
                }
            }
            _ => {
                self.set_error(GL_INVALID_ENUM);
                false
            }
        };
        if changed {
            self.mark_dirty(DirtyFlags::TEXTURES);
        }
    }

    // =========================================================================
    // PROGRAM OPERATIONS
    // =========================================================================

    /// Use shader program
    pub fn use_program(&mut self, program: ProgramHandle) {
        if self.program.current_program != program {
            self.program.current_program = program;
            self.mark_dirty(DirtyFlags::PROGRAM);
        }
    }

    // =========================================================================
    // FRAMEBUFFER OPERATIONS
    // =========================================================================

    /// Bind framebuffer
    pub fn bind_framebuffer(&mut self, target: GLenum, framebuffer: FramebufferHandle) {
        let changed = match target {
            GL_FRAMEBUFFER | GL_DRAW_FRAMEBUFFER => {
                if self.framebuffer.draw_framebuffer != framebuffer {
                    self.framebuffer.draw_framebuffer = framebuffer;
                    if target == GL_FRAMEBUFFER {
                        self.framebuffer.read_framebuffer = framebuffer;
                    }
                    true
                } else {
                    false
                }
            }
            GL_READ_FRAMEBUFFER => {
                if self.framebuffer.read_framebuffer != framebuffer {
                    self.framebuffer.read_framebuffer = framebuffer;
                    true
                } else {
                    false
                }
            }
            _ => {
                self.set_error(GL_INVALID_ENUM);
                false
            }
        };
        if changed {
            self.mark_dirty(DirtyFlags::FRAMEBUFFER);
        }
    }

    /// Bind renderbuffer
    pub fn bind_renderbuffer(&mut self, target: GLenum, renderbuffer: RenderbufferHandle) {
        if target == GL_RENDERBUFFER {
            if self.framebuffer.renderbuffer != renderbuffer {
                self.framebuffer.renderbuffer = renderbuffer;
                self.mark_dirty(DirtyFlags::FRAMEBUFFER);
            }
        } else {
            self.set_error(GL_INVALID_ENUM);
        }
    }

    // =========================================================================
    // VAO OPERATIONS
    // =========================================================================

    /// Bind vertex array object
    pub fn bind_vertex_array(&mut self, vao: VaoHandle) {
        if self.vao.bound_vao != vao {
            self.vao.bound_vao = vao;
            self.mark_dirty(DirtyFlags::VERTEX_INPUT);
        }
    }

    /// Enable vertex attribute
    pub fn enable_vertex_attrib_array(&mut self, index: u32) {
        if (index as usize) < MAX_VERTEX_ATTRIBS {
            if !self.vao.attribs[index as usize].enabled {
                self.vao.attribs[index as usize].enabled = true;
                self.mark_dirty(DirtyFlags::VERTEX_INPUT);
            }
        } else {
            self.set_error(GL_INVALID_VALUE);
        }
    }

    /// Disable vertex attribute
    pub fn disable_vertex_attrib_array(&mut self, index: u32) {
        if (index as usize) < MAX_VERTEX_ATTRIBS {
            if self.vao.attribs[index as usize].enabled {
                self.vao.attribs[index as usize].enabled = false;
                self.mark_dirty(DirtyFlags::VERTEX_INPUT);
            }
        } else {
            self.set_error(GL_INVALID_VALUE);
        }
    }

    /// Set vertex attribute pointer
    pub fn vertex_attrib_pointer(
        &mut self,
        index: u32,
        size: i32,
        type_: GLenum,
        normalized: bool,
        stride: i32,
        offset: usize,
    ) {
        if (index as usize) < MAX_VERTEX_ATTRIBS {
            let attrib = &mut self.vao.attribs[index as usize];
            attrib.size = size;
            attrib.type_ = type_;
            attrib.normalized = normalized;
            attrib.stride = stride;
            attrib.offset = offset;
            // Record current array buffer binding
            self.vao.attrib_buffers[index as usize] = self.buffers.array_buffer;
            self.mark_dirty(DirtyFlags::VERTEX_INPUT);
        } else {
            self.set_error(GL_INVALID_VALUE);
        }
    }

    /// Set vertex attribute divisor for instancing
    pub fn vertex_attrib_divisor(&mut self, index: u32, divisor: u32) {
        if (index as usize) < MAX_VERTEX_ATTRIBS {
            if self.vao.attribs[index as usize].divisor != divisor {
                self.vao.attribs[index as usize].divisor = divisor;
                self.mark_dirty(DirtyFlags::VERTEX_INPUT);
            }
        } else {
            self.set_error(GL_INVALID_VALUE);
        }
    }

    // =========================================================================
    // CAPABILITY ENABLE/DISABLE
    // =========================================================================

    /// Enable a capability
    pub fn enable(&mut self, cap: GLenum) {
        match cap {
            GL_BLEND => self.set_blend_enabled(true),
            GL_CULL_FACE => self.set_cull_face_enabled(true),
            GL_DEPTH_TEST => self.set_depth_test_enabled(true),
            GL_STENCIL_TEST => self.set_stencil_test_enabled(true),
            GL_SCISSOR_TEST => {
                if !self.raster.scissor_enabled {
                    self.raster.scissor_enabled = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_FILL => {
                if !self.raster.polygon_offset_fill {
                    self.raster.polygon_offset_fill = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_LINE => {
                if !self.raster.polygon_offset_line {
                    self.raster.polygon_offset_line = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_POINT => {
                if !self.raster.polygon_offset_point {
                    self.raster.polygon_offset_point = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_LINE_SMOOTH => {
                if !self.raster.line_smooth {
                    self.raster.line_smooth = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_MULTISAMPLE => {
                if !self.raster.multisample {
                    self.raster.multisample = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_RASTERIZER_DISCARD => {
                if !self.raster.discard {
                    self.raster.discard = true;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            _ => self.set_error(GL_INVALID_ENUM),
        }
    }

    /// Disable a capability
    pub fn disable(&mut self, cap: GLenum) {
        match cap {
            GL_BLEND => self.set_blend_enabled(false),
            GL_CULL_FACE => self.set_cull_face_enabled(false),
            GL_DEPTH_TEST => self.set_depth_test_enabled(false),
            GL_STENCIL_TEST => self.set_stencil_test_enabled(false),
            GL_SCISSOR_TEST => {
                if self.raster.scissor_enabled {
                    self.raster.scissor_enabled = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_FILL => {
                if self.raster.polygon_offset_fill {
                    self.raster.polygon_offset_fill = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_LINE => {
                if self.raster.polygon_offset_line {
                    self.raster.polygon_offset_line = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_POLYGON_OFFSET_POINT => {
                if self.raster.polygon_offset_point {
                    self.raster.polygon_offset_point = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_LINE_SMOOTH => {
                if self.raster.line_smooth {
                    self.raster.line_smooth = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_MULTISAMPLE => {
                if self.raster.multisample {
                    self.raster.multisample = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            GL_RASTERIZER_DISCARD => {
                if self.raster.discard {
                    self.raster.discard = false;
                    self.mark_dirty(DirtyFlags::RASTER);
                }
            }
            _ => self.set_error(GL_INVALID_ENUM),
        }
    }

    /// Check if a capability is enabled
    pub fn is_enabled(&self, cap: GLenum) -> bool {
        match cap {
            GL_BLEND => self.blend.enabled,
            GL_CULL_FACE => self.raster.cull_enabled,
            GL_DEPTH_TEST => self.depth.test_enabled,
            GL_STENCIL_TEST => self.stencil.enabled,
            GL_SCISSOR_TEST => self.raster.scissor_enabled,
            GL_POLYGON_OFFSET_FILL => self.raster.polygon_offset_fill,
            GL_POLYGON_OFFSET_LINE => self.raster.polygon_offset_line,
            GL_POLYGON_OFFSET_POINT => self.raster.polygon_offset_point,
            GL_LINE_SMOOTH => self.raster.line_smooth,
            GL_MULTISAMPLE => self.raster.multisample,
            GL_RASTERIZER_DISCARD => self.raster.discard,
            _ => false,
        }
    }
}
