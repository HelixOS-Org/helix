//! # OpenGL API Dispatch
//!
//! Public OpenGL API functions that dispatch to the context.
//! These are the entry points that match the standard OpenGL API.

use crate::context::{GlContext, SharedGlContext};
use crate::enums::*;
use crate::state::DirtyFlags;
use crate::types::*;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;

// =============================================================================
// THREAD-LOCAL CONTEXT
// =============================================================================

// In a real implementation, this would be a thread-local
// For now, we use a static option that requires explicit initialization
static mut CURRENT_CONTEXT: Option<SharedGlContext> = None;

/// Make a context current
/// 
/// # Safety
/// This function modifies global state and should be called
/// from a single thread or with proper synchronization.
pub unsafe fn make_current(context: Option<SharedGlContext>) {
    CURRENT_CONTEXT = context;
}

/// Get the current context
/// 
/// # Safety
/// This function reads global state.
pub unsafe fn get_current() -> Option<SharedGlContext> {
    CURRENT_CONTEXT.clone()
}

/// Execute a closure with the current context
fn with_context<F, R>(f: F) -> R
where
    F: FnOnce(&mut GlContext) -> R,
    R: Default,
{
    unsafe {
        if let Some(ref ctx) = CURRENT_CONTEXT {
            let mut guard = ctx.lock();
            f(&mut guard)
        } else {
            R::default()
        }
    }
}

// =============================================================================
// ERROR HANDLING
// =============================================================================

/// Get the current error code
#[no_mangle]
pub extern "C" fn glGetError() -> GLenum {
    with_context(|ctx| ctx.state.get_error())
}

// =============================================================================
// STATE QUERIES
// =============================================================================

/// Get string value
#[no_mangle]
pub extern "C" fn glGetString(name: GLenum) -> *const u8 {
    with_context(|ctx| ctx.get_string(name).as_ptr())
}

/// Check if capability is enabled
#[no_mangle]
pub extern "C" fn glIsEnabled(cap: GLenum) -> GLboolean {
    with_context(|ctx| if ctx.state.is_enabled(cap) { GL_TRUE } else { GL_FALSE })
}

// =============================================================================
// ENABLE/DISABLE
// =============================================================================

/// Enable a capability
#[no_mangle]
pub extern "C" fn glEnable(cap: GLenum) {
    with_context(|ctx| ctx.state.enable(cap));
}

/// Disable a capability
#[no_mangle]
pub extern "C" fn glDisable(cap: GLenum) {
    with_context(|ctx| ctx.state.disable(cap));
}

// =============================================================================
// VIEWPORT AND SCISSOR
// =============================================================================

/// Set viewport
#[no_mangle]
pub extern "C" fn glViewport(x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
    with_context(|ctx| {
        ctx.state.set_viewport(x as f32, y as f32, width as f32, height as f32);
    });
}

/// Set depth range
#[no_mangle]
pub extern "C" fn glDepthRange(near: GLdouble, far: GLdouble) {
    with_context(|ctx| ctx.state.set_depth_range(near, far));
}

/// Set depth range (float version)
#[no_mangle]
pub extern "C" fn glDepthRangef(near: GLfloat, far: GLfloat) {
    with_context(|ctx| ctx.state.set_depth_range(near as f64, far as f64));
}

/// Set scissor rect
#[no_mangle]
pub extern "C" fn glScissor(x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
    with_context(|ctx| {
        ctx.state.set_scissor(x, y, width as u32, height as u32);
    });
}

// =============================================================================
// CLEAR
// =============================================================================

/// Set clear color
#[no_mangle]
pub extern "C" fn glClearColor(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
    with_context(|ctx| ctx.state.set_clear_color(red, green, blue, alpha));
}

/// Set clear depth
#[no_mangle]
pub extern "C" fn glClearDepth(depth: GLdouble) {
    with_context(|ctx| ctx.state.set_clear_depth(depth));
}

/// Set clear depth (float version)
#[no_mangle]
pub extern "C" fn glClearDepthf(depth: GLfloat) {
    with_context(|ctx| ctx.state.set_clear_depth(depth as f64));
}

/// Set clear stencil
#[no_mangle]
pub extern "C" fn glClearStencil(s: GLint) {
    with_context(|ctx| ctx.state.set_clear_stencil(s));
}

/// Clear buffers
#[no_mangle]
pub extern "C" fn glClear(mask: GLbitfield) {
    with_context(|ctx| {
        // TODO: Translate to Vulkan clear attachment commands
        // This requires knowing the current framebuffer and render pass
        let _ = mask;
    });
}

// =============================================================================
// BLEND
// =============================================================================

/// Set blend function
#[no_mangle]
pub extern "C" fn glBlendFunc(sfactor: GLenum, dfactor: GLenum) {
    with_context(|ctx| ctx.state.set_blend_func(sfactor, dfactor));
}

/// Set blend function with separate RGB/alpha
#[no_mangle]
pub extern "C" fn glBlendFuncSeparate(
    srcRGB: GLenum,
    dstRGB: GLenum,
    srcAlpha: GLenum,
    dstAlpha: GLenum,
) {
    with_context(|ctx| ctx.state.set_blend_func_separate(srcRGB, dstRGB, srcAlpha, dstAlpha));
}

/// Set blend equation
#[no_mangle]
pub extern "C" fn glBlendEquation(mode: GLenum) {
    with_context(|ctx| ctx.state.set_blend_equation(mode));
}

/// Set blend equation with separate RGB/alpha
#[no_mangle]
pub extern "C" fn glBlendEquationSeparate(modeRGB: GLenum, modeAlpha: GLenum) {
    with_context(|ctx| ctx.state.set_blend_equation_separate(modeRGB, modeAlpha));
}

/// Set blend color
#[no_mangle]
pub extern "C" fn glBlendColor(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
    with_context(|ctx| ctx.state.set_blend_color(red, green, blue, alpha));
}

// =============================================================================
// DEPTH
// =============================================================================

/// Set depth function
#[no_mangle]
pub extern "C" fn glDepthFunc(func: GLenum) {
    with_context(|ctx| ctx.state.set_depth_func(func));
}

/// Set depth mask
#[no_mangle]
pub extern "C" fn glDepthMask(flag: GLboolean) {
    with_context(|ctx| ctx.state.set_depth_mask(flag != GL_FALSE));
}

// =============================================================================
// STENCIL
// =============================================================================

/// Set stencil function
#[no_mangle]
pub extern "C" fn glStencilFunc(func: GLenum, ref_: GLint, mask: GLuint) {
    with_context(|ctx| ctx.state.set_stencil_func(func, ref_, mask));
}

/// Set stencil function for specific face
#[no_mangle]
pub extern "C" fn glStencilFuncSeparate(face: GLenum, func: GLenum, ref_: GLint, mask: GLuint) {
    with_context(|ctx| ctx.state.set_stencil_func_separate(face, func, ref_, mask));
}

/// Set stencil operations
#[no_mangle]
pub extern "C" fn glStencilOp(sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
    with_context(|ctx| ctx.state.set_stencil_op(sfail, dpfail, dppass));
}

/// Set stencil operations for specific face
#[no_mangle]
pub extern "C" fn glStencilOpSeparate(
    face: GLenum,
    sfail: GLenum,
    dpfail: GLenum,
    dppass: GLenum,
) {
    with_context(|ctx| ctx.state.set_stencil_op_separate(face, sfail, dpfail, dppass));
}

/// Set stencil mask
#[no_mangle]
pub extern "C" fn glStencilMask(mask: GLuint) {
    with_context(|ctx| ctx.state.set_stencil_mask(mask));
}

/// Set stencil mask for specific face
#[no_mangle]
pub extern "C" fn glStencilMaskSeparate(face: GLenum, mask: GLuint) {
    with_context(|ctx| ctx.state.set_stencil_mask_separate(face, mask));
}

// =============================================================================
// FACE CULLING
// =============================================================================

/// Set cull face mode
#[no_mangle]
pub extern "C" fn glCullFace(mode: GLenum) {
    with_context(|ctx| ctx.state.set_cull_face(mode));
}

/// Set front face winding
#[no_mangle]
pub extern "C" fn glFrontFace(mode: GLenum) {
    with_context(|ctx| ctx.state.set_front_face(mode));
}

// =============================================================================
// POLYGON
// =============================================================================

/// Set polygon mode (wireframe, etc.)
#[no_mangle]
pub extern "C" fn glPolygonMode(face: GLenum, mode: GLenum) {
    with_context(|ctx| ctx.state.set_polygon_mode(face, mode));
}

/// Set polygon offset
#[no_mangle]
pub extern "C" fn glPolygonOffset(factor: GLfloat, units: GLfloat) {
    with_context(|ctx| ctx.state.set_polygon_offset(factor, units));
}

/// Set line width
#[no_mangle]
pub extern "C" fn glLineWidth(width: GLfloat) {
    with_context(|ctx| ctx.state.set_line_width(width));
}

/// Set point size
#[no_mangle]
pub extern "C" fn glPointSize(size: GLfloat) {
    with_context(|ctx| ctx.state.set_point_size(size));
}

// =============================================================================
// COLOR MASK
// =============================================================================

/// Set color write mask
#[no_mangle]
pub extern "C" fn glColorMask(red: GLboolean, green: GLboolean, blue: GLboolean, alpha: GLboolean) {
    with_context(|ctx| {
        ctx.state.set_color_mask(
            red != GL_FALSE,
            green != GL_FALSE,
            blue != GL_FALSE,
            alpha != GL_FALSE,
        );
    });
}

// =============================================================================
// BUFFER OBJECTS
// =============================================================================

/// Generate buffer names
#[no_mangle]
pub extern "C" fn glGenBuffers(n: GLsizei, buffers: *mut GLuint) {
    if n < 0 || buffers.is_null() {
        with_context(|ctx| ctx.state.set_error(GL_INVALID_VALUE));
        return;
    }
    with_context(|ctx| {
        let handles = ctx.gen_buffers(n as u32);
        unsafe {
            for (i, &handle) in handles.iter().enumerate() {
                *buffers.add(i) = handle;
            }
        }
    });
}

/// Delete buffers
#[no_mangle]
pub extern "C" fn glDeleteBuffers(n: GLsizei, buffers: *const GLuint) {
    if n < 0 || buffers.is_null() {
        return;
    }
    with_context(|ctx| {
        let slice = unsafe { core::slice::from_raw_parts(buffers, n as usize) };
        ctx.delete_buffers(slice);
    });
}

/// Check if name is a buffer
#[no_mangle]
pub extern "C" fn glIsBuffer(buffer: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_buffer(buffer) { GL_TRUE } else { GL_FALSE })
}

/// Bind buffer to target
#[no_mangle]
pub extern "C" fn glBindBuffer(target: GLenum, buffer: GLuint) {
    with_context(|ctx| {
        use crate::types::BufferHandle;
        let handle = if buffer == 0 {
            BufferHandle::default()
        } else {
            BufferHandle::new(buffer)
        };
        ctx.state.bind_buffer(target, handle);
    });
}

/// Allocate buffer storage
#[no_mangle]
pub extern "C" fn glBufferData(target: GLenum, size: GLsizeiptr, data: *const c_void, usage: GLenum) {
    with_context(|ctx| {
        // Get bound buffer for target
        let buffer_handle = match target {
            GL_ARRAY_BUFFER => ctx.state.buffers.array_buffer,
            GL_ELEMENT_ARRAY_BUFFER => ctx.state.buffers.element_array_buffer,
            _ => {
                ctx.state.set_error(GL_INVALID_ENUM);
                return;
            }
        };

        if !buffer_handle.is_valid() {
            ctx.state.set_error(GL_INVALID_OPERATION);
            return;
        }

        if let Some(buffer) = ctx.buffers.get_mut(&buffer_handle.id()) {
            buffer.size = size as usize;
            buffer.usage = usage.into();
            // TODO: Allocate Vulkan buffer via magma-mem
            // TODO: Copy data if provided
        }
    });
}

/// Update buffer sub-data
#[no_mangle]
pub extern "C" fn glBufferSubData(
    target: GLenum,
    offset: GLintptr,
    size: GLsizeiptr,
    data: *const c_void,
) {
    with_context(|ctx| {
        let buffer_handle = match target {
            GL_ARRAY_BUFFER => ctx.state.buffers.array_buffer,
            GL_ELEMENT_ARRAY_BUFFER => ctx.state.buffers.element_array_buffer,
            _ => {
                ctx.state.set_error(GL_INVALID_ENUM);
                return;
            }
        };

        if !buffer_handle.is_valid() {
            ctx.state.set_error(GL_INVALID_OPERATION);
            return;
        }

        if let Some(_buffer) = ctx.buffers.get(&buffer_handle.id()) {
            // TODO: Copy data to Vulkan buffer
            let _ = (offset, size, data);
        }
    });
}

/// Bind buffer to indexed target
#[no_mangle]
pub extern "C" fn glBindBufferBase(target: GLenum, index: GLuint, buffer: GLuint) {
    with_context(|ctx| {
        use crate::types::BufferHandle;
        let handle = if buffer == 0 {
            BufferHandle::default()
        } else {
            BufferHandle::new(buffer)
        };
        ctx.state.bind_buffer_base(target, index, handle);
    });
}

/// Bind buffer range to indexed target
#[no_mangle]
pub extern "C" fn glBindBufferRange(
    target: GLenum,
    index: GLuint,
    buffer: GLuint,
    offset: GLintptr,
    size: GLsizeiptr,
) {
    with_context(|ctx| {
        use crate::types::BufferHandle;
        let handle = if buffer == 0 {
            BufferHandle::default()
        } else {
            BufferHandle::new(buffer)
        };
        ctx.state.bind_buffer_range(target, index, handle, offset as usize, size as usize);
    });
}

// =============================================================================
// VERTEX ARRAY OBJECTS
// =============================================================================

/// Generate VAO names
#[no_mangle]
pub extern "C" fn glGenVertexArrays(n: GLsizei, arrays: *mut GLuint) {
    if n < 0 || arrays.is_null() {
        with_context(|ctx| ctx.state.set_error(GL_INVALID_VALUE));
        return;
    }
    with_context(|ctx| {
        let handles = ctx.gen_vertex_arrays(n as u32);
        unsafe {
            for (i, &handle) in handles.iter().enumerate() {
                *arrays.add(i) = handle;
            }
        }
    });
}

/// Delete VAOs
#[no_mangle]
pub extern "C" fn glDeleteVertexArrays(n: GLsizei, arrays: *const GLuint) {
    if n < 0 || arrays.is_null() {
        return;
    }
    with_context(|ctx| {
        let slice = unsafe { core::slice::from_raw_parts(arrays, n as usize) };
        ctx.delete_vertex_arrays(slice);
    });
}

/// Check if name is a VAO
#[no_mangle]
pub extern "C" fn glIsVertexArray(array: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_vertex_array(array) { GL_TRUE } else { GL_FALSE })
}

/// Bind VAO
#[no_mangle]
pub extern "C" fn glBindVertexArray(array: GLuint) {
    with_context(|ctx| {
        use crate::types::VaoHandle;
        let handle = VaoHandle::new(array);
        ctx.state.bind_vertex_array(handle);
    });
}

/// Enable vertex attribute array
#[no_mangle]
pub extern "C" fn glEnableVertexAttribArray(index: GLuint) {
    with_context(|ctx| ctx.state.enable_vertex_attrib_array(index));
}

/// Disable vertex attribute array
#[no_mangle]
pub extern "C" fn glDisableVertexAttribArray(index: GLuint) {
    with_context(|ctx| ctx.state.disable_vertex_attrib_array(index));
}

/// Set vertex attribute pointer
#[no_mangle]
pub extern "C" fn glVertexAttribPointer(
    index: GLuint,
    size: GLint,
    type_: GLenum,
    normalized: GLboolean,
    stride: GLsizei,
    pointer: *const c_void,
) {
    with_context(|ctx| {
        ctx.state.vertex_attrib_pointer(
            index,
            size,
            type_,
            normalized != GL_FALSE,
            stride,
            pointer as usize,
        );
    });
}

/// Set vertex attribute divisor
#[no_mangle]
pub extern "C" fn glVertexAttribDivisor(index: GLuint, divisor: GLuint) {
    with_context(|ctx| ctx.state.vertex_attrib_divisor(index, divisor));
}

// =============================================================================
// SHADERS
// =============================================================================

/// Create a shader
#[no_mangle]
pub extern "C" fn glCreateShader(type_: GLenum) -> GLuint {
    with_context(|ctx| ctx.create_shader(type_))
}

/// Delete a shader
#[no_mangle]
pub extern "C" fn glDeleteShader(shader: GLuint) {
    with_context(|ctx| ctx.delete_shader(shader));
}

/// Check if name is a shader
#[no_mangle]
pub extern "C" fn glIsShader(shader: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_shader(shader) { GL_TRUE } else { GL_FALSE })
}

/// Set shader source
#[no_mangle]
pub extern "C" fn glShaderSource(
    shader: GLuint,
    count: GLsizei,
    string: *const *const i8,
    length: *const GLint,
) {
    with_context(|ctx| {
        if let Some(shader_obj) = ctx.shaders.get_mut(&shader) {
            shader_obj.source.clear();
            
            for i in 0..count as usize {
                let src_ptr = unsafe { *string.add(i) };
                let len = if length.is_null() {
                    // Null-terminated
                    let mut l = 0;
                    unsafe {
                        while *src_ptr.add(l) != 0 {
                            l += 1;
                        }
                    }
                    l
                } else {
                    let l = unsafe { *length.add(i) };
                    if l < 0 {
                        // Null-terminated
                        let mut len = 0;
                        unsafe {
                            while *src_ptr.add(len) != 0 {
                                len += 1;
                            }
                        }
                        len
                    } else {
                        l as usize
                    }
                };
                
                let slice = unsafe { core::slice::from_raw_parts(src_ptr as *const u8, len) };
                if let Ok(s) = core::str::from_utf8(slice) {
                    shader_obj.source.push_str(s);
                }
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Compile a shader
#[no_mangle]
pub extern "C" fn glCompileShader(shader: GLuint) {
    with_context(|ctx| {
        if let Some(shader_obj) = ctx.shaders.get_mut(&shader) {
            // TODO: Use naga to compile GLSL to SPIR-V
            // For now, mark as compiled for testing
            shader_obj.compiled = true;
            shader_obj.info_log.clear();
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Get shader parameter
#[no_mangle]
pub extern "C" fn glGetShaderiv(shader: GLuint, pname: GLenum, params: *mut GLint) {
    with_context(|ctx| {
        if let Some(shader_obj) = ctx.shaders.get(&shader) {
            let value = match pname {
                GL_SHADER_TYPE => match shader_obj.shader_type {
                    crate::context::ShaderType::Vertex => GL_VERTEX_SHADER as i32,
                    crate::context::ShaderType::Fragment => GL_FRAGMENT_SHADER as i32,
                    crate::context::ShaderType::Geometry => GL_GEOMETRY_SHADER as i32,
                    crate::context::ShaderType::TessControl => GL_TESS_CONTROL_SHADER as i32,
                    crate::context::ShaderType::TessEvaluation => GL_TESS_EVALUATION_SHADER as i32,
                    crate::context::ShaderType::Compute => GL_COMPUTE_SHADER as i32,
                },
                GL_DELETE_STATUS => if shader_obj.delete_pending { GL_TRUE as i32 } else { GL_FALSE as i32 },
                GL_COMPILE_STATUS => if shader_obj.compiled { GL_TRUE as i32 } else { GL_FALSE as i32 },
                GL_INFO_LOG_LENGTH => shader_obj.info_log.len() as i32 + 1,
                GL_SHADER_SOURCE_LENGTH => shader_obj.source.len() as i32 + 1,
                _ => {
                    ctx.state.set_error(GL_INVALID_ENUM);
                    return;
                }
            };
            unsafe { *params = value; }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Get shader info log
#[no_mangle]
pub extern "C" fn glGetShaderInfoLog(
    shader: GLuint,
    maxLength: GLsizei,
    length: *mut GLsizei,
    infoLog: *mut i8,
) {
    with_context(|ctx| {
        if let Some(shader_obj) = ctx.shaders.get(&shader) {
            let log_bytes = shader_obj.info_log.as_bytes();
            let copy_len = core::cmp::min(log_bytes.len(), (maxLength - 1) as usize);
            
            unsafe {
                ptr::copy_nonoverlapping(log_bytes.as_ptr(), infoLog as *mut u8, copy_len);
                *infoLog.add(copy_len) = 0;
                if !length.is_null() {
                    *length = copy_len as GLsizei;
                }
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

// =============================================================================
// PROGRAMS
// =============================================================================

/// Create a program
#[no_mangle]
pub extern "C" fn glCreateProgram() -> GLuint {
    with_context(|ctx| ctx.create_program())
}

/// Delete a program
#[no_mangle]
pub extern "C" fn glDeleteProgram(program: GLuint) {
    with_context(|ctx| ctx.delete_program(program));
}

/// Check if name is a program
#[no_mangle]
pub extern "C" fn glIsProgram(program: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_program(program) { GL_TRUE } else { GL_FALSE })
}

/// Attach shader to program
#[no_mangle]
pub extern "C" fn glAttachShader(program: GLuint, shader: GLuint) {
    with_context(|ctx| {
        let shader_type = if let Some(s) = ctx.shaders.get(&shader) {
            s.shader_type
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
            return;
        };
        
        if let Some(program_obj) = ctx.programs.get_mut(&program) {
            match shader_type {
                crate::context::ShaderType::Vertex => program_obj.vertex_shader = Some(shader),
                crate::context::ShaderType::Fragment => program_obj.fragment_shader = Some(shader),
                crate::context::ShaderType::Geometry => program_obj.geometry_shader = Some(shader),
                crate::context::ShaderType::TessControl => program_obj.tess_control_shader = Some(shader),
                crate::context::ShaderType::TessEvaluation => program_obj.tess_evaluation_shader = Some(shader),
                crate::context::ShaderType::Compute => program_obj.compute_shader = Some(shader),
            }
            
            // Increment shader ref count
            if let Some(s) = ctx.shaders.get_mut(&shader) {
                s.ref_count += 1;
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Detach shader from program
#[no_mangle]
pub extern "C" fn glDetachShader(program: GLuint, shader: GLuint) {
    with_context(|ctx| {
        if let Some(program_obj) = ctx.programs.get_mut(&program) {
            let detached = if program_obj.vertex_shader == Some(shader) {
                program_obj.vertex_shader = None;
                true
            } else if program_obj.fragment_shader == Some(shader) {
                program_obj.fragment_shader = None;
                true
            } else if program_obj.geometry_shader == Some(shader) {
                program_obj.geometry_shader = None;
                true
            } else if program_obj.tess_control_shader == Some(shader) {
                program_obj.tess_control_shader = None;
                true
            } else if program_obj.tess_evaluation_shader == Some(shader) {
                program_obj.tess_evaluation_shader = None;
                true
            } else if program_obj.compute_shader == Some(shader) {
                program_obj.compute_shader = None;
                true
            } else {
                false
            };
            
            if detached {
                // Decrement shader ref count
                if let Some(s) = ctx.shaders.get_mut(&shader) {
                    s.ref_count = s.ref_count.saturating_sub(1);
                    if s.ref_count == 0 && s.delete_pending {
                        ctx.shaders.remove(&shader);
                    }
                }
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Link a program
#[no_mangle]
pub extern "C" fn glLinkProgram(program: GLuint) {
    with_context(|ctx| {
        if let Some(program_obj) = ctx.programs.get_mut(&program) {
            // TODO: Create Vulkan pipeline layout and descriptor set layouts
            program_obj.linked = true;
            program_obj.info_log.clear();
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Use a program
#[no_mangle]
pub extern "C" fn glUseProgram(program: GLuint) {
    with_context(|ctx| {
        use crate::types::ProgramHandle;
        let handle = if program == 0 {
            ProgramHandle::default()
        } else {
            ProgramHandle::new(program)
        };
        ctx.state.use_program(handle);
    });
}

/// Get program parameter
#[no_mangle]
pub extern "C" fn glGetProgramiv(program: GLuint, pname: GLenum, params: *mut GLint) {
    with_context(|ctx| {
        if let Some(program_obj) = ctx.programs.get(&program) {
            let value = match pname {
                GL_DELETE_STATUS => if program_obj.delete_pending { GL_TRUE as i32 } else { GL_FALSE as i32 },
                GL_LINK_STATUS => if program_obj.linked { GL_TRUE as i32 } else { GL_FALSE as i32 },
                GL_VALIDATE_STATUS => if program_obj.validated { GL_TRUE as i32 } else { GL_FALSE as i32 },
                GL_INFO_LOG_LENGTH => program_obj.info_log.len() as i32 + 1,
                GL_ATTACHED_SHADERS => {
                    let mut count = 0;
                    if program_obj.vertex_shader.is_some() { count += 1; }
                    if program_obj.fragment_shader.is_some() { count += 1; }
                    if program_obj.geometry_shader.is_some() { count += 1; }
                    if program_obj.tess_control_shader.is_some() { count += 1; }
                    if program_obj.tess_evaluation_shader.is_some() { count += 1; }
                    if program_obj.compute_shader.is_some() { count += 1; }
                    count
                },
                GL_ACTIVE_UNIFORMS => program_obj.uniforms.len() as i32,
                GL_ACTIVE_ATTRIBUTES => program_obj.attributes.len() as i32,
                _ => {
                    ctx.state.set_error(GL_INVALID_ENUM);
                    return;
                }
            };
            unsafe { *params = value; }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
        }
    });
}

/// Get uniform location
#[no_mangle]
pub extern "C" fn glGetUniformLocation(program: GLuint, name: *const i8) -> GLint {
    with_context(|ctx| {
        if let Some(program_obj) = ctx.programs.get(&program) {
            // Convert C string to Rust string
            let mut len = 0;
            unsafe {
                while *name.add(len) != 0 {
                    len += 1;
                }
            }
            let slice = unsafe { core::slice::from_raw_parts(name as *const u8, len) };
            if let Ok(name_str) = core::str::from_utf8(slice) {
                program_obj.get_uniform_location(name_str)
            } else {
                -1
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
            -1
        }
    })
}

/// Get attribute location
#[no_mangle]
pub extern "C" fn glGetAttribLocation(program: GLuint, name: *const i8) -> GLint {
    with_context(|ctx| {
        if let Some(program_obj) = ctx.programs.get(&program) {
            let mut len = 0;
            unsafe {
                while *name.add(len) != 0 {
                    len += 1;
                }
            }
            let slice = unsafe { core::slice::from_raw_parts(name as *const u8, len) };
            if let Ok(name_str) = core::str::from_utf8(slice) {
                program_obj.get_attrib_location(name_str)
            } else {
                -1
            }
        } else {
            ctx.state.set_error(GL_INVALID_VALUE);
            -1
        }
    })
}

// =============================================================================
// TEXTURES
// =============================================================================

/// Generate texture names
#[no_mangle]
pub extern "C" fn glGenTextures(n: GLsizei, textures: *mut GLuint) {
    if n < 0 || textures.is_null() {
        with_context(|ctx| ctx.state.set_error(GL_INVALID_VALUE));
        return;
    }
    with_context(|ctx| {
        let handles = ctx.gen_textures(n as u32);
        unsafe {
            for (i, &handle) in handles.iter().enumerate() {
                *textures.add(i) = handle;
            }
        }
    });
}

/// Delete textures
#[no_mangle]
pub extern "C" fn glDeleteTextures(n: GLsizei, textures: *const GLuint) {
    if n < 0 || textures.is_null() {
        return;
    }
    with_context(|ctx| {
        let slice = unsafe { core::slice::from_raw_parts(textures, n as usize) };
        ctx.delete_textures(slice);
    });
}

/// Check if name is a texture
#[no_mangle]
pub extern "C" fn glIsTexture(texture: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_texture(texture) { GL_TRUE } else { GL_FALSE })
}

/// Bind texture
#[no_mangle]
pub extern "C" fn glBindTexture(target: GLenum, texture: GLuint) {
    with_context(|ctx| {
        use crate::types::TextureHandle;
        let handle = if texture == 0 {
            TextureHandle::default()
        } else {
            TextureHandle::new(texture)
        };
        ctx.state.bind_texture(target, handle);
    });
}

/// Set active texture unit
#[no_mangle]
pub extern "C" fn glActiveTexture(texture: GLenum) {
    with_context(|ctx| {
        let unit = texture.saturating_sub(0x84C0); // GL_TEXTURE0
        ctx.state.set_active_texture(unit);
    });
}

// =============================================================================
// FRAMEBUFFERS
// =============================================================================

/// Generate framebuffer names
#[no_mangle]
pub extern "C" fn glGenFramebuffers(n: GLsizei, framebuffers: *mut GLuint) {
    if n < 0 || framebuffers.is_null() {
        with_context(|ctx| ctx.state.set_error(GL_INVALID_VALUE));
        return;
    }
    with_context(|ctx| {
        let handles = ctx.gen_framebuffers(n as u32);
        unsafe {
            for (i, &handle) in handles.iter().enumerate() {
                *framebuffers.add(i) = handle;
            }
        }
    });
}

/// Delete framebuffers
#[no_mangle]
pub extern "C" fn glDeleteFramebuffers(n: GLsizei, framebuffers: *const GLuint) {
    if n < 0 || framebuffers.is_null() {
        return;
    }
    with_context(|ctx| {
        let slice = unsafe { core::slice::from_raw_parts(framebuffers, n as usize) };
        ctx.delete_framebuffers(slice);
    });
}

/// Check if name is a framebuffer
#[no_mangle]
pub extern "C" fn glIsFramebuffer(framebuffer: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_framebuffer(framebuffer) { GL_TRUE } else { GL_FALSE })
}

/// Bind framebuffer
#[no_mangle]
pub extern "C" fn glBindFramebuffer(target: GLenum, framebuffer: GLuint) {
    with_context(|ctx| {
        use crate::types::FramebufferHandle;
        let handle = FramebufferHandle::new(framebuffer);
        ctx.state.bind_framebuffer(target, handle);
    });
}

// =============================================================================
// RENDERBUFFERS
// =============================================================================

/// Generate renderbuffer names
#[no_mangle]
pub extern "C" fn glGenRenderbuffers(n: GLsizei, renderbuffers: *mut GLuint) {
    if n < 0 || renderbuffers.is_null() {
        with_context(|ctx| ctx.state.set_error(GL_INVALID_VALUE));
        return;
    }
    with_context(|ctx| {
        let handles = ctx.gen_renderbuffers(n as u32);
        unsafe {
            for (i, &handle) in handles.iter().enumerate() {
                *renderbuffers.add(i) = handle;
            }
        }
    });
}

/// Delete renderbuffers
#[no_mangle]
pub extern "C" fn glDeleteRenderbuffers(n: GLsizei, renderbuffers: *const GLuint) {
    if n < 0 || renderbuffers.is_null() {
        return;
    }
    with_context(|ctx| {
        let slice = unsafe { core::slice::from_raw_parts(renderbuffers, n as usize) };
        ctx.delete_renderbuffers(slice);
    });
}

/// Check if name is a renderbuffer
#[no_mangle]
pub extern "C" fn glIsRenderbuffer(renderbuffer: GLuint) -> GLboolean {
    with_context(|ctx| if ctx.is_renderbuffer(renderbuffer) { GL_TRUE } else { GL_FALSE })
}

/// Bind renderbuffer
#[no_mangle]
pub extern "C" fn glBindRenderbuffer(target: GLenum, renderbuffer: GLuint) {
    with_context(|ctx| {
        use crate::types::RenderbufferHandle;
        let handle = if renderbuffer == 0 {
            RenderbufferHandle::default()
        } else {
            RenderbufferHandle::new(renderbuffer)
        };
        ctx.state.bind_renderbuffer(target, handle);
    });
}

// =============================================================================
// DRAW CALLS
// =============================================================================

/// Draw arrays
#[no_mangle]
pub extern "C" fn glDrawArrays(mode: GLenum, first: GLint, count: GLsizei) {
    with_context(|ctx| {
        // TODO: Translate to vkCmdDraw
        // 1. Flush dirty state to Vulkan
        // 2. Bind pipeline based on current state
        // 3. Bind vertex buffers from VAO
        // 4. Issue draw command
        let _ = (mode, first, count);
    });
}

/// Draw elements
#[no_mangle]
pub extern "C" fn glDrawElements(mode: GLenum, count: GLsizei, type_: GLenum, indices: *const c_void) {
    with_context(|ctx| {
        // TODO: Translate to vkCmdDrawIndexed
        let _ = (mode, count, type_, indices);
    });
}

/// Draw arrays instanced
#[no_mangle]
pub extern "C" fn glDrawArraysInstanced(
    mode: GLenum,
    first: GLint,
    count: GLsizei,
    instancecount: GLsizei,
) {
    with_context(|ctx| {
        // TODO: Translate to vkCmdDraw with instanceCount
        let _ = (mode, first, count, instancecount);
    });
}

/// Draw elements instanced
#[no_mangle]
pub extern "C" fn glDrawElementsInstanced(
    mode: GLenum,
    count: GLsizei,
    type_: GLenum,
    indices: *const c_void,
    instancecount: GLsizei,
) {
    with_context(|ctx| {
        // TODO: Translate to vkCmdDrawIndexed with instanceCount
        let _ = (mode, count, type_, indices, instancecount);
    });
}

// =============================================================================
// SYNC
// =============================================================================

/// Flush GL commands
#[no_mangle]
pub extern "C" fn glFlush() {
    with_context(|ctx| ctx.flush());
}

/// Finish all GL commands
#[no_mangle]
pub extern "C" fn glFinish() {
    with_context(|ctx| ctx.finish());
}
