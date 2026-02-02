//! # OpenGL Context
//!
//! The GL context holds all OpenGL state and manages translation to Vulkan.
//! This is the main entry point for the Helix-GL implementation.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

use crate::enums::*;
use crate::state::GlState;
use crate::types::*;

// =============================================================================
// CONTEXT CONFIGURATION
// =============================================================================

/// OpenGL context creation parameters
#[derive(Debug, Clone)]
pub struct GlContextConfig {
    /// Major version (minimum 3)
    pub major_version: u32,
    /// Minor version
    pub minor_version: u32,
    /// Use core profile (always true, compatibility not supported)
    pub core_profile: bool,
    /// Enable debug output
    pub debug: bool,
    /// Number of samples for default framebuffer
    pub samples: u32,
    /// Double buffering
    pub double_buffer: bool,
    /// sRGB framebuffer
    pub srgb: bool,
}

impl Default for GlContextConfig {
    fn default() -> Self {
        Self {
            major_version: 3,
            minor_version: 3,
            core_profile: true,
            debug: false,
            samples: 1,
            double_buffer: true,
            srgb: false,
        }
    }
}

// =============================================================================
// RESOURCE HANDLES
// =============================================================================

/// Handle generator for OpenGL objects
pub struct HandleGenerator {
    next_id: AtomicU32,
}

impl HandleGenerator {
    /// Create a new handle generator
    pub const fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
        }
    }

    /// Generate a new unique handle
    pub fn generate(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Generate multiple handles
    pub fn generate_n(&self, count: u32) -> Vec<u32> {
        let mut handles = Vec::with_capacity(count as usize);
        for _ in 0..count {
            handles.push(self.generate());
        }
        handles
    }
}

impl Default for HandleGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// BUFFER OBJECT
// =============================================================================

/// Buffer usage hint for Vulkan memory type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsageHint {
    /// Static data, GPU read-only
    StaticDraw,
    /// Static data, GPU readback
    StaticRead,
    /// Static data, GPU copy
    StaticCopy,
    /// Dynamic data, GPU read-only
    DynamicDraw,
    /// Dynamic data, GPU readback
    DynamicRead,
    /// Dynamic data, GPU copy
    DynamicCopy,
    /// Streaming data, GPU read-only
    StreamDraw,
    /// Streaming data, GPU readback
    StreamRead,
    /// Streaming data, GPU copy
    StreamCopy,
}

impl From<GLenum> for BufferUsageHint {
    fn from(usage: GLenum) -> Self {
        match usage {
            GL_STATIC_DRAW => BufferUsageHint::StaticDraw,
            GL_STATIC_READ => BufferUsageHint::StaticRead,
            GL_STATIC_COPY => BufferUsageHint::StaticCopy,
            GL_DYNAMIC_DRAW => BufferUsageHint::DynamicDraw,
            GL_DYNAMIC_READ => BufferUsageHint::DynamicRead,
            GL_DYNAMIC_COPY => BufferUsageHint::DynamicCopy,
            GL_STREAM_DRAW => BufferUsageHint::StreamDraw,
            GL_STREAM_READ => BufferUsageHint::StreamRead,
            GL_STREAM_COPY => BufferUsageHint::StreamCopy,
            _ => BufferUsageHint::StaticDraw,
        }
    }
}

/// Internal buffer object representation
#[derive(Debug)]
pub struct BufferObject {
    /// Buffer name (handle)
    pub name: u32,
    /// Buffer size in bytes
    pub size: usize,
    /// Usage hint
    pub usage: BufferUsageHint,
    /// Is this buffer immutable (created with glBufferStorage)
    pub immutable: bool,
    /// Vulkan buffer handle (opaque, provided by magma-vulkan)
    pub vk_buffer: u64,
    /// Mapped pointer if currently mapped
    pub mapped_ptr: Option<*mut u8>,
    /// Mapped offset
    pub mapped_offset: usize,
    /// Mapped length
    pub mapped_length: usize,
}

impl BufferObject {
    /// Create a new buffer object
    pub fn new(name: u32) -> Self {
        Self {
            name,
            size: 0,
            usage: BufferUsageHint::StaticDraw,
            immutable: false,
            vk_buffer: 0,
            mapped_ptr: None,
            mapped_offset: 0,
            mapped_length: 0,
        }
    }
}

// =============================================================================
// SHADER OBJECT
// =============================================================================

/// Shader type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderType {
    /// Vertex shader
    Vertex,
    /// Fragment shader
    Fragment,
    /// Geometry shader
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEvaluation,
    /// Compute shader
    Compute,
}

impl TryFrom<GLenum> for ShaderType {
    type Error = ();

    fn try_from(type_: GLenum) -> Result<Self, Self::Error> {
        match type_ {
            GL_VERTEX_SHADER => Ok(ShaderType::Vertex),
            GL_FRAGMENT_SHADER => Ok(ShaderType::Fragment),
            GL_GEOMETRY_SHADER => Ok(ShaderType::Geometry),
            GL_TESS_CONTROL_SHADER => Ok(ShaderType::TessControl),
            GL_TESS_EVALUATION_SHADER => Ok(ShaderType::TessEvaluation),
            GL_COMPUTE_SHADER => Ok(ShaderType::Compute),
            _ => Err(()),
        }
    }
}

/// Internal shader object representation
#[derive(Debug)]
pub struct ShaderObject {
    /// Shader name (handle)
    pub name: u32,
    /// Shader type
    pub shader_type: ShaderType,
    /// Source code
    pub source: String,
    /// Compiled SPIR-V (via naga)
    pub spirv: Option<Vec<u32>>,
    /// Compilation log
    pub info_log: String,
    /// Compilation status
    pub compiled: bool,
    /// Marked for deletion
    pub delete_pending: bool,
    /// Reference count (programs using this shader)
    pub ref_count: u32,
}

impl ShaderObject {
    /// Create a new shader object
    pub fn new(name: u32, shader_type: ShaderType) -> Self {
        Self {
            name,
            shader_type,
            source: String::new(),
            spirv: None,
            info_log: String::new(),
            compiled: false,
            delete_pending: false,
            ref_count: 0,
        }
    }
}

// =============================================================================
// PROGRAM OBJECT
// =============================================================================

/// Uniform variable information
#[derive(Debug, Clone)]
pub struct UniformInfo {
    /// Uniform name
    pub name: String,
    /// Uniform location
    pub location: i32,
    /// Uniform type
    pub type_: GLenum,
    /// Array size (1 for non-arrays)
    pub size: i32,
    /// Offset in uniform block (if applicable)
    pub offset: i32,
    /// Block index (-1 if not in block)
    pub block_index: i32,
}

/// Attribute variable information
#[derive(Debug, Clone)]
pub struct AttribInfo {
    /// Attribute name
    pub name: String,
    /// Attribute location
    pub location: i32,
    /// Attribute type
    pub type_: GLenum,
    /// Array size
    pub size: i32,
}

/// Internal program object representation
#[derive(Debug)]
pub struct ProgramObject {
    /// Program name (handle)
    pub name: u32,
    /// Attached vertex shader
    pub vertex_shader: Option<u32>,
    /// Attached fragment shader
    pub fragment_shader: Option<u32>,
    /// Attached geometry shader
    pub geometry_shader: Option<u32>,
    /// Attached tessellation control shader
    pub tess_control_shader: Option<u32>,
    /// Attached tessellation evaluation shader
    pub tess_evaluation_shader: Option<u32>,
    /// Attached compute shader
    pub compute_shader: Option<u32>,
    /// Uniforms
    pub uniforms: Vec<UniformInfo>,
    /// Attributes
    pub attributes: Vec<AttribInfo>,
    /// Link status
    pub linked: bool,
    /// Link log
    pub info_log: String,
    /// Validated status
    pub validated: bool,
    /// Marked for deletion
    pub delete_pending: bool,
    /// Vulkan pipeline layout handle
    pub vk_pipeline_layout: u64,
    /// Vulkan descriptor set layouts
    pub vk_descriptor_set_layouts: Vec<u64>,
}

impl ProgramObject {
    /// Create a new program object
    pub fn new(name: u32) -> Self {
        Self {
            name,
            vertex_shader: None,
            fragment_shader: None,
            geometry_shader: None,
            tess_control_shader: None,
            tess_evaluation_shader: None,
            compute_shader: None,
            uniforms: Vec::new(),
            attributes: Vec::new(),
            linked: false,
            info_log: String::new(),
            validated: false,
            delete_pending: false,
            vk_pipeline_layout: 0,
            vk_descriptor_set_layouts: Vec::new(),
        }
    }

    /// Get uniform location by name
    pub fn get_uniform_location(&self, name: &str) -> i32 {
        for uniform in &self.uniforms {
            if uniform.name == name {
                return uniform.location;
            }
        }
        -1
    }

    /// Get attribute location by name
    pub fn get_attrib_location(&self, name: &str) -> i32 {
        for attrib in &self.attributes {
            if attrib.name == name {
                return attrib.location;
            }
        }
        -1
    }
}

// =============================================================================
// TEXTURE OBJECT
// =============================================================================

/// Internal texture object representation
#[derive(Debug)]
pub struct TextureObject {
    /// Texture name (handle)
    pub name: u32,
    /// Texture target (GL_TEXTURE_2D, etc.)
    pub target: GLenum,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth (for 3D textures)
    pub depth: u32,
    /// Internal format
    pub internal_format: GLenum,
    /// Number of mip levels
    pub mip_levels: u32,
    /// Number of array layers
    pub array_layers: u32,
    /// Sample count (for multisample textures)
    pub samples: u32,
    /// Minification filter
    pub min_filter: GLenum,
    /// Magnification filter
    pub mag_filter: GLenum,
    /// S wrap mode
    pub wrap_s: GLenum,
    /// T wrap mode
    pub wrap_t: GLenum,
    /// R wrap mode
    pub wrap_r: GLenum,
    /// Border color
    pub border_color: [f32; 4],
    /// Min LOD
    pub min_lod: f32,
    /// Max LOD
    pub max_lod: f32,
    /// Base mip level
    pub base_level: u32,
    /// Max mip level
    pub max_level: u32,
    /// Compare mode
    pub compare_mode: GLenum,
    /// Compare function
    pub compare_func: GLenum,
    /// Is immutable (allocated with glTexStorage)
    pub immutable: bool,
    /// Vulkan image handle
    pub vk_image: u64,
    /// Vulkan image view handle
    pub vk_image_view: u64,
    /// Vulkan sampler handle
    pub vk_sampler: u64,
}

impl TextureObject {
    /// Create a new texture object
    pub fn new(name: u32) -> Self {
        Self {
            name,
            target: 0,
            width: 0,
            height: 0,
            depth: 0,
            internal_format: 0,
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            min_filter: GL_NEAREST_MIPMAP_LINEAR,
            mag_filter: GL_LINEAR,
            wrap_s: GL_REPEAT,
            wrap_t: GL_REPEAT,
            wrap_r: GL_REPEAT,
            border_color: [0.0, 0.0, 0.0, 0.0],
            min_lod: -1000.0,
            max_lod: 1000.0,
            base_level: 0,
            max_level: 1000,
            compare_mode: 0,
            compare_func: GL_LEQUAL,
            immutable: false,
            vk_image: 0,
            vk_image_view: 0,
            vk_sampler: 0,
        }
    }
}

// =============================================================================
// FRAMEBUFFER OBJECT
// =============================================================================

/// Framebuffer attachment
#[derive(Debug, Clone, Copy, Default)]
pub struct FramebufferAttachment {
    /// Attachment type (texture or renderbuffer)
    pub is_texture: bool,
    /// Attached object name
    pub name: u32,
    /// Mip level (for textures)
    pub level: u32,
    /// Layer (for array textures)
    pub layer: u32,
    /// Cube face (for cube maps)
    pub face: GLenum,
}

/// Internal framebuffer object representation
#[derive(Debug)]
pub struct FramebufferObject {
    /// Framebuffer name (handle)
    pub name: u32,
    /// Color attachments (0-7)
    pub color_attachments: [FramebufferAttachment; 8],
    /// Depth attachment
    pub depth_attachment: FramebufferAttachment,
    /// Stencil attachment
    pub stencil_attachment: FramebufferAttachment,
    /// Draw buffers configuration
    pub draw_buffers: [GLenum; 8],
    /// Read buffer
    pub read_buffer: GLenum,
    /// Vulkan framebuffer handle
    pub vk_framebuffer: u64,
    /// Vulkan render pass handle
    pub vk_render_pass: u64,
    /// Framebuffer width
    pub width: u32,
    /// Framebuffer height
    pub height: u32,
    /// Framebuffer layers
    pub layers: u32,
}

impl FramebufferObject {
    /// Create a new framebuffer object
    pub fn new(name: u32) -> Self {
        Self {
            name,
            color_attachments: [FramebufferAttachment::default(); 8],
            depth_attachment: FramebufferAttachment::default(),
            stencil_attachment: FramebufferAttachment::default(),
            draw_buffers: [GL_COLOR_ATTACHMENT0, 0, 0, 0, 0, 0, 0, 0],
            read_buffer: GL_COLOR_ATTACHMENT0,
            vk_framebuffer: 0,
            vk_render_pass: 0,
            width: 0,
            height: 0,
            layers: 0,
        }
    }
}

// =============================================================================
// RENDERBUFFER OBJECT
// =============================================================================

/// Internal renderbuffer object representation
#[derive(Debug)]
pub struct RenderbufferObject {
    /// Renderbuffer name (handle)
    pub name: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Internal format
    pub internal_format: GLenum,
    /// Sample count
    pub samples: u32,
    /// Vulkan image handle
    pub vk_image: u64,
    /// Vulkan image view handle
    pub vk_image_view: u64,
}

impl RenderbufferObject {
    /// Create a new renderbuffer object
    pub fn new(name: u32) -> Self {
        Self {
            name,
            width: 0,
            height: 0,
            internal_format: 0,
            samples: 0,
            vk_image: 0,
            vk_image_view: 0,
        }
    }
}

// =============================================================================
// VERTEX ARRAY OBJECT
// =============================================================================

/// Internal VAO representation
#[derive(Debug)]
pub struct VertexArrayObject {
    /// VAO name (handle)
    pub name: u32,
    /// Element buffer binding
    pub element_buffer: u32,
    /// Vertex buffer bindings (per attribute)
    pub vertex_buffers: [u32; 16],
    /// Attribute states
    pub attribs: [crate::state::VertexAttribState; 16],
}

impl VertexArrayObject {
    /// Create a new VAO
    pub fn new(name: u32) -> Self {
        Self {
            name,
            element_buffer: 0,
            vertex_buffers: [0; 16],
            attribs: [crate::state::VertexAttribState::default(); 16],
        }
    }
}

// =============================================================================
// GL CONTEXT
// =============================================================================

/// The main OpenGL context
pub struct GlContext {
    /// Configuration
    pub config: GlContextConfig,
    /// Current GL state
    pub state: GlState,
    /// Handle generator
    handles: HandleGenerator,
    /// Buffer objects
    pub buffers: BTreeMap<u32, BufferObject>,
    /// Shader objects
    pub shaders: BTreeMap<u32, ShaderObject>,
    /// Program objects
    pub programs: BTreeMap<u32, ProgramObject>,
    /// Texture objects
    pub textures: BTreeMap<u32, TextureObject>,
    /// Framebuffer objects
    pub framebuffers: BTreeMap<u32, FramebufferObject>,
    /// Renderbuffer objects
    pub renderbuffers: BTreeMap<u32, RenderbufferObject>,
    /// Vertex array objects
    pub vaos: BTreeMap<u32, VertexArrayObject>,
    // TODO: Add Vulkan resources from magma-vulkan
    // pub vk_instance: VkInstance,
    // pub vk_device: VkDevice,
    // pub vk_swapchain: VkSwapchain,
    // pub vk_command_pool: VkCommandPool,
    // pub vk_descriptor_pool: VkDescriptorPool,
}

impl GlContext {
    /// Create a new GL context
    pub fn new(config: GlContextConfig) -> Self {
        // Create default VAO (VAO 0 is always valid in core profile)
        let mut vaos = BTreeMap::new();
        vaos.insert(0, VertexArrayObject::new(0));

        // Create default framebuffer (FBO 0 is the window framebuffer)
        let mut framebuffers = BTreeMap::new();
        framebuffers.insert(0, FramebufferObject::new(0));

        Self {
            config,
            state: GlState::new(),
            handles: HandleGenerator::new(),
            buffers: BTreeMap::new(),
            shaders: BTreeMap::new(),
            programs: BTreeMap::new(),
            textures: BTreeMap::new(),
            framebuffers,
            renderbuffers: BTreeMap::new(),
            vaos,
        }
    }

    // =========================================================================
    // BUFFER OPERATIONS
    // =========================================================================

    /// Generate buffer names
    pub fn gen_buffers(&mut self, count: u32) -> Vec<u32> {
        let handles = self.handles.generate_n(count);
        for &handle in &handles {
            self.buffers.insert(handle, BufferObject::new(handle));
        }
        handles
    }

    /// Delete buffers
    pub fn delete_buffers(&mut self, buffers: &[u32]) {
        for &buffer in buffers {
            if buffer != 0 {
                self.buffers.remove(&buffer);
            }
        }
    }

    /// Check if name is a buffer
    pub fn is_buffer(&self, buffer: u32) -> bool {
        buffer != 0 && self.buffers.contains_key(&buffer)
    }

    /// Get buffer object
    pub fn get_buffer(&self, buffer: u32) -> Option<&BufferObject> {
        self.buffers.get(&buffer)
    }

    /// Get mutable buffer object
    pub fn get_buffer_mut(&mut self, buffer: u32) -> Option<&mut BufferObject> {
        self.buffers.get_mut(&buffer)
    }

    // =========================================================================
    // SHADER OPERATIONS
    // =========================================================================

    /// Create a shader
    pub fn create_shader(&mut self, shader_type: GLenum) -> u32 {
        if let Ok(st) = ShaderType::try_from(shader_type) {
            let handle = self.handles.generate();
            self.shaders.insert(handle, ShaderObject::new(handle, st));
            handle
        } else {
            self.state.set_error(GL_INVALID_ENUM);
            0
        }
    }

    /// Delete a shader
    pub fn delete_shader(&mut self, shader: u32) {
        if shader != 0 {
            if let Some(shader_obj) = self.shaders.get_mut(&shader) {
                if shader_obj.ref_count > 0 {
                    // Shader is attached to program, mark for deletion
                    shader_obj.delete_pending = true;
                } else {
                    self.shaders.remove(&shader);
                }
            }
        }
    }

    /// Check if name is a shader
    pub fn is_shader(&self, shader: u32) -> bool {
        shader != 0 && self.shaders.contains_key(&shader)
    }

    /// Get shader object
    pub fn get_shader(&self, shader: u32) -> Option<&ShaderObject> {
        self.shaders.get(&shader)
    }

    /// Get mutable shader object
    pub fn get_shader_mut(&mut self, shader: u32) -> Option<&mut ShaderObject> {
        self.shaders.get_mut(&shader)
    }

    // =========================================================================
    // PROGRAM OPERATIONS
    // =========================================================================

    /// Create a program
    pub fn create_program(&mut self) -> u32 {
        let handle = self.handles.generate();
        self.programs.insert(handle, ProgramObject::new(handle));
        handle
    }

    /// Delete a program
    pub fn delete_program(&mut self, program: u32) {
        if program != 0 {
            if let Some(program_obj) = self.programs.remove(&program) {
                // Decrease shader ref counts and potentially delete them
                for shader_handle in [
                    program_obj.vertex_shader,
                    program_obj.fragment_shader,
                    program_obj.geometry_shader,
                    program_obj.tess_control_shader,
                    program_obj.tess_evaluation_shader,
                    program_obj.compute_shader,
                ]
                .iter()
                .flatten()
                {
                    if let Some(shader) = self.shaders.get_mut(shader_handle) {
                        shader.ref_count = shader.ref_count.saturating_sub(1);
                        if shader.ref_count == 0 && shader.delete_pending {
                            self.shaders.remove(shader_handle);
                        }
                    }
                }
            }
        }
    }

    /// Check if name is a program
    pub fn is_program(&self, program: u32) -> bool {
        program != 0 && self.programs.contains_key(&program)
    }

    /// Get program object
    pub fn get_program(&self, program: u32) -> Option<&ProgramObject> {
        self.programs.get(&program)
    }

    /// Get mutable program object
    pub fn get_program_mut(&mut self, program: u32) -> Option<&mut ProgramObject> {
        self.programs.get_mut(&program)
    }

    // =========================================================================
    // TEXTURE OPERATIONS
    // =========================================================================

    /// Generate texture names
    pub fn gen_textures(&mut self, count: u32) -> Vec<u32> {
        let handles = self.handles.generate_n(count);
        for &handle in &handles {
            self.textures.insert(handle, TextureObject::new(handle));
        }
        handles
    }

    /// Delete textures
    pub fn delete_textures(&mut self, textures: &[u32]) {
        for &texture in textures {
            if texture != 0 {
                self.textures.remove(&texture);
            }
        }
    }

    /// Check if name is a texture
    pub fn is_texture(&self, texture: u32) -> bool {
        texture != 0 && self.textures.contains_key(&texture)
    }

    /// Get texture object
    pub fn get_texture(&self, texture: u32) -> Option<&TextureObject> {
        self.textures.get(&texture)
    }

    /// Get mutable texture object
    pub fn get_texture_mut(&mut self, texture: u32) -> Option<&mut TextureObject> {
        self.textures.get_mut(&texture)
    }

    // =========================================================================
    // FRAMEBUFFER OPERATIONS
    // =========================================================================

    /// Generate framebuffer names
    pub fn gen_framebuffers(&mut self, count: u32) -> Vec<u32> {
        let handles = self.handles.generate_n(count);
        for &handle in &handles {
            self.framebuffers
                .insert(handle, FramebufferObject::new(handle));
        }
        handles
    }

    /// Delete framebuffers
    pub fn delete_framebuffers(&mut self, framebuffers: &[u32]) {
        for &framebuffer in framebuffers {
            if framebuffer != 0 {
                self.framebuffers.remove(&framebuffer);
            }
        }
    }

    /// Check if name is a framebuffer
    pub fn is_framebuffer(&self, framebuffer: u32) -> bool {
        framebuffer == 0 || self.framebuffers.contains_key(&framebuffer)
    }

    /// Get framebuffer object
    pub fn get_framebuffer(&self, framebuffer: u32) -> Option<&FramebufferObject> {
        self.framebuffers.get(&framebuffer)
    }

    /// Get mutable framebuffer object
    pub fn get_framebuffer_mut(&mut self, framebuffer: u32) -> Option<&mut FramebufferObject> {
        self.framebuffers.get_mut(&framebuffer)
    }

    // =========================================================================
    // RENDERBUFFER OPERATIONS
    // =========================================================================

    /// Generate renderbuffer names
    pub fn gen_renderbuffers(&mut self, count: u32) -> Vec<u32> {
        let handles = self.handles.generate_n(count);
        for &handle in &handles {
            self.renderbuffers
                .insert(handle, RenderbufferObject::new(handle));
        }
        handles
    }

    /// Delete renderbuffers
    pub fn delete_renderbuffers(&mut self, renderbuffers: &[u32]) {
        for &renderbuffer in renderbuffers {
            if renderbuffer != 0 {
                self.renderbuffers.remove(&renderbuffer);
            }
        }
    }

    /// Check if name is a renderbuffer
    pub fn is_renderbuffer(&self, renderbuffer: u32) -> bool {
        renderbuffer != 0 && self.renderbuffers.contains_key(&renderbuffer)
    }

    /// Get renderbuffer object
    pub fn get_renderbuffer(&self, renderbuffer: u32) -> Option<&RenderbufferObject> {
        self.renderbuffers.get(&renderbuffer)
    }

    /// Get mutable renderbuffer object
    pub fn get_renderbuffer_mut(&mut self, renderbuffer: u32) -> Option<&mut RenderbufferObject> {
        self.renderbuffers.get_mut(&renderbuffer)
    }

    // =========================================================================
    // VAO OPERATIONS
    // =========================================================================

    /// Generate VAO names
    pub fn gen_vertex_arrays(&mut self, count: u32) -> Vec<u32> {
        let handles = self.handles.generate_n(count);
        for &handle in &handles {
            self.vaos.insert(handle, VertexArrayObject::new(handle));
        }
        handles
    }

    /// Delete VAOs
    pub fn delete_vertex_arrays(&mut self, vaos: &[u32]) {
        for &vao in vaos {
            if vao != 0 {
                self.vaos.remove(&vao);
            }
        }
    }

    /// Check if name is a VAO
    pub fn is_vertex_array(&self, vao: u32) -> bool {
        vao == 0 || self.vaos.contains_key(&vao)
    }

    /// Get VAO
    pub fn get_vertex_array(&self, vao: u32) -> Option<&VertexArrayObject> {
        self.vaos.get(&vao)
    }

    /// Get mutable VAO
    pub fn get_vertex_array_mut(&mut self, vao: u32) -> Option<&mut VertexArrayObject> {
        self.vaos.get_mut(&vao)
    }

    // =========================================================================
    // STRING QUERIES
    // =========================================================================

    /// Get string for GL_VENDOR, GL_RENDERER, etc.
    pub fn get_string(&self, name: GLenum) -> &'static str {
        match name {
            GL_VENDOR => "Helix OS",
            GL_RENDERER => "Helix-GL (Magma Vulkan Translation)",
            GL_VERSION => "3.3.0 Helix-GL",
            GL_SHADING_LANGUAGE_VERSION => "330",
            _ => "",
        }
    }

    // =========================================================================
    // FLUSH AND SYNC
    // =========================================================================

    /// Flush pending GL commands to Vulkan
    pub fn flush(&mut self) {
        // TODO: Submit any pending Vulkan command buffers
    }

    /// Finish all GL commands (blocking)
    pub fn finish(&mut self) {
        // TODO: Wait for Vulkan queue idle
        self.flush();
    }
}

/// Thread-safe GL context reference
pub type SharedGlContext = Arc<Mutex<GlContext>>;

/// Create a new shared GL context
pub fn create_context(config: GlContextConfig) -> SharedGlContext {
    Arc::new(Mutex::new(GlContext::new(config)))
}
