//! # OpenGL Types
//!
//! Core OpenGL type definitions matching the GL specification.

use core::ffi::c_void;

// =============================================================================
// BASIC TYPES
// =============================================================================

/// OpenGL boolean type
pub type GLboolean = u8;
/// OpenGL byte type (signed)
pub type GLbyte = i8;
/// OpenGL unsigned byte type
pub type GLubyte = u8;
/// OpenGL short type
pub type GLshort = i16;
/// OpenGL unsigned short type
pub type GLushort = u16;
/// OpenGL int type
pub type GLint = i32;
/// OpenGL unsigned int type
pub type GLuint = u32;
/// OpenGL int64 type
pub type GLint64 = i64;
/// OpenGL unsigned int64 type
pub type GLuint64 = u64;
/// OpenGL fixed point type
pub type GLfixed = i32;
/// OpenGL size type
pub type GLsizei = i32;
/// OpenGL enum type
pub type GLenum = u32;
/// OpenGL intptr type
pub type GLintptr = isize;
/// OpenGL sizeiptr type
pub type GLsizeiptr = isize;
/// OpenGL sync object
pub type GLsync = *mut c_void;
/// OpenGL bitfield type
pub type GLbitfield = u32;
/// OpenGL half float type
pub type GLhalf = u16;
/// OpenGL float type
pub type GLfloat = f32;
/// OpenGL clamp float type
pub type GLclampf = f32;
/// OpenGL double type
pub type GLdouble = f64;
/// OpenGL clamp double type
pub type GLclampd = f64;
/// OpenGL char type
pub type GLchar = i8;

// =============================================================================
// BOOLEAN CONSTANTS
// =============================================================================

/// GL false value
pub const GL_FALSE: GLboolean = 0;
/// GL true value
pub const GL_TRUE: GLboolean = 1;

// =============================================================================
// HANDLE TYPES
// =============================================================================

/// Opaque handle for GL objects
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct GlHandle(pub GLuint);

impl GlHandle {
    /// Invalid/null handle
    pub const NONE: Self = Self(0);

    /// Create new handle
    pub const fn new(id: GLuint) -> Self {
        Self(id)
    }

    /// Check if handle is valid (non-zero)
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Get raw value
    pub const fn raw(&self) -> GLuint {
        self.0
    }
}

impl From<GLuint> for GlHandle {
    fn from(id: GLuint) -> Self {
        Self(id)
    }
}

impl From<GlHandle> for GLuint {
    fn from(handle: GlHandle) -> Self {
        handle.0
    }
}

// =============================================================================
// TYPED HANDLES
// =============================================================================

/// Buffer object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BufferHandle(pub GlHandle);

/// Texture object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TextureHandle(pub GlHandle);

/// Shader object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderHandle(pub GlHandle);

/// Program object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ProgramHandle(pub GlHandle);

/// Vertex array object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct VaoHandle(pub GlHandle);

/// Framebuffer object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FramebufferHandle(pub GlHandle);

/// Renderbuffer object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct RenderbufferHandle(pub GlHandle);

/// Sampler object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SamplerHandle(pub GlHandle);

/// Query object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueryHandle(pub GlHandle);

// =============================================================================
// UNIFORM LOCATION
// =============================================================================

/// Uniform location (can be -1 for invalid)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct UniformLocation(pub GLint);

impl UniformLocation {
    /// Invalid uniform location
    pub const INVALID: Self = Self(-1);

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.0 >= 0
    }
}

impl Default for UniformLocation {
    fn default() -> Self {
        Self::INVALID
    }
}

// =============================================================================
// ATTRIBUTE LOCATION
// =============================================================================

/// Attribute location
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AttribLocation(pub GLint);

impl AttribLocation {
    /// Invalid attribute location
    pub const INVALID: Self = Self(-1);

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.0 >= 0
    }
}

impl Default for AttribLocation {
    fn default() -> Self {
        Self::INVALID
    }
}

// =============================================================================
// DEBUG CALLBACK
// =============================================================================

/// Debug callback function type
pub type GLDebugProc = Option<
    unsafe extern "C" fn(
        source: GLenum,
        type_: GLenum,
        id: GLuint,
        severity: GLenum,
        length: GLsizei,
        message: *const GLchar,
        user_param: *mut c_void,
    ),
>;
