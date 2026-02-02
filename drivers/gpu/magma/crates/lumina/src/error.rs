//! Error handling for Lumina
//!
//! This module provides the error types used throughout Lumina.
//! Errors are designed to be informative and actionable.

use alloc::string::String;
use core::fmt;

/// The result type used throughout Lumina
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur in Lumina operations
#[derive(Debug)]
pub enum Error {
    /// No compatible GPU device was found
    NoDevice,

    /// Failed to create the graphics context
    ContextCreation(String),

    /// Failed to create a swapchain
    SwapchainCreation(String),

    /// Failed to acquire a swapchain image
    SwapchainAcquire,

    /// Swapchain is out of date and needs recreation
    SwapchainOutOfDate,

    /// Failed to create a buffer
    BufferCreation(String),

    /// Failed to create a texture
    TextureCreation(String),

    /// Failed to create a pipeline
    PipelineCreation(String),

    /// Shader compilation failed
    ShaderCompilation {
        /// The stage that failed
        stage: ShaderStage,
        /// Error message from the compiler
        message: String,
    },

    /// Shader linking failed
    ShaderLinking(String),

    /// Invalid resource handle
    InvalidHandle,

    /// Resource is still in use and cannot be destroyed
    ResourceInUse,

    /// Out of GPU memory
    OutOfMemory,

    /// Feature not supported by the device
    NotSupported(String),

    /// Validation error (debug builds only)
    Validation(String),

    /// The render graph has a cycle
    GraphCycle,

    /// The render graph has unresolved dependencies
    GraphUnresolved(String),

    /// Invalid framebuffer configuration
    InvalidFramebuffer(String),

    /// Texture format mismatch
    FormatMismatch {
        /// Expected format
        expected: String,
        /// Actual format
        found: String,
    },

    /// Attempt to use resource from a different context
    ContextMismatch,

    /// Generic internal error
    Internal(String),
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex,
    /// Fragment shader
    Fragment,
    /// Compute shader
    Compute,
    /// Geometry shader
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEval,
    /// Mesh shader
    Mesh,
    /// Task shader
    Task,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => write!(f, "No compatible GPU device found"),
            Self::ContextCreation(msg) => write!(f, "Failed to create graphics context: {}", msg),
            Self::SwapchainCreation(msg) => write!(f, "Failed to create swapchain: {}", msg),
            Self::SwapchainAcquire => write!(f, "Failed to acquire swapchain image"),
            Self::SwapchainOutOfDate => write!(f, "Swapchain is out of date"),
            Self::BufferCreation(msg) => write!(f, "Failed to create buffer: {}", msg),
            Self::TextureCreation(msg) => write!(f, "Failed to create texture: {}", msg),
            Self::PipelineCreation(msg) => write!(f, "Failed to create pipeline: {}", msg),
            Self::ShaderCompilation { stage, message } => {
                write!(f, "Shader compilation failed ({:?}): {}", stage, message)
            }
            Self::ShaderLinking(msg) => write!(f, "Shader linking failed: {}", msg),
            Self::InvalidHandle => write!(f, "Invalid resource handle"),
            Self::ResourceInUse => write!(f, "Resource is still in use"),
            Self::OutOfMemory => write!(f, "Out of GPU memory"),
            Self::NotSupported(feature) => write!(f, "Feature not supported: {}", feature),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
            Self::GraphCycle => write!(f, "Render graph contains a cycle"),
            Self::GraphUnresolved(dep) => {
                write!(f, "Render graph has unresolved dependency: {}", dep)
            }
            Self::InvalidFramebuffer(msg) => write!(f, "Invalid framebuffer: {}", msg),
            Self::FormatMismatch { expected, found } => {
                write!(f, "Format mismatch: expected {}, found {}", expected, found)
            }
            Self::ContextMismatch => write!(f, "Resource used with wrong context"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl fmt::Display for ShaderStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vertex => write!(f, "vertex"),
            Self::Fragment => write!(f, "fragment"),
            Self::Compute => write!(f, "compute"),
            Self::Geometry => write!(f, "geometry"),
            Self::TessControl => write!(f, "tessellation control"),
            Self::TessEval => write!(f, "tessellation evaluation"),
            Self::Mesh => write!(f, "mesh"),
            Self::Task => write!(f, "task"),
        }
    }
}

/// Result extension trait for adding context to errors
pub trait ResultExt<T> {
    /// Add context to an error
    fn context(self, context: &str) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, context: &str) -> Result<T> {
        self.map_err(|e| match e {
            Error::Internal(msg) => Error::Internal(alloc::format!("{}: {}", context, msg)),
            _ => Error::Internal(alloc::format!("{}: {:?}", context, e)),
        })
    }
}
