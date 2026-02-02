//! # OpenGL Enums
//!
//! OpenGL enumeration constants.

use crate::types::GLenum;

// =============================================================================
// ERROR CODES
// =============================================================================

/// No error
pub const GL_NO_ERROR: GLenum = 0;
/// Invalid enum parameter
pub const GL_INVALID_ENUM: GLenum = 0x0500;
/// Invalid value parameter
pub const GL_INVALID_VALUE: GLenum = 0x0501;
/// Invalid operation
pub const GL_INVALID_OPERATION: GLenum = 0x0502;
/// Stack overflow
pub const GL_STACK_OVERFLOW: GLenum = 0x0503;
/// Stack underflow
pub const GL_STACK_UNDERFLOW: GLenum = 0x0504;
/// Out of memory
pub const GL_OUT_OF_MEMORY: GLenum = 0x0505;
/// Invalid framebuffer operation
pub const GL_INVALID_FRAMEBUFFER_OPERATION: GLenum = 0x0506;

// =============================================================================
// BUFFER TARGETS
// =============================================================================

/// Array buffer (vertex data)
pub const GL_ARRAY_BUFFER: GLenum = 0x8892;
/// Element array buffer (index data)
pub const GL_ELEMENT_ARRAY_BUFFER: GLenum = 0x8893;
/// Uniform buffer
pub const GL_UNIFORM_BUFFER: GLenum = 0x8A11;
/// Shader storage buffer
pub const GL_SHADER_STORAGE_BUFFER: GLenum = 0x90D2;
/// Copy read buffer
pub const GL_COPY_READ_BUFFER: GLenum = 0x8F36;
/// Copy write buffer
pub const GL_COPY_WRITE_BUFFER: GLenum = 0x8F37;
/// Pixel pack buffer
pub const GL_PIXEL_PACK_BUFFER: GLenum = 0x88EB;
/// Pixel unpack buffer
pub const GL_PIXEL_UNPACK_BUFFER: GLenum = 0x88EC;
/// Transform feedback buffer
pub const GL_TRANSFORM_FEEDBACK_BUFFER: GLenum = 0x8C8E;
/// Draw indirect buffer
pub const GL_DRAW_INDIRECT_BUFFER: GLenum = 0x8F3F;
/// Dispatch indirect buffer
pub const GL_DISPATCH_INDIRECT_BUFFER: GLenum = 0x90EE;
/// Atomic counter buffer
pub const GL_ATOMIC_COUNTER_BUFFER: GLenum = 0x92C0;

// =============================================================================
// BUFFER USAGE
// =============================================================================

/// Static draw usage
pub const GL_STATIC_DRAW: GLenum = 0x88E4;
/// Static read usage
pub const GL_STATIC_READ: GLenum = 0x88E5;
/// Static copy usage
pub const GL_STATIC_COPY: GLenum = 0x88E6;
/// Dynamic draw usage
pub const GL_DYNAMIC_DRAW: GLenum = 0x88E8;
/// Dynamic read usage
pub const GL_DYNAMIC_READ: GLenum = 0x88E9;
/// Dynamic copy usage
pub const GL_DYNAMIC_COPY: GLenum = 0x88EA;
/// Stream draw usage
pub const GL_STREAM_DRAW: GLenum = 0x88E0;
/// Stream read usage
pub const GL_STREAM_READ: GLenum = 0x88E1;
/// Stream copy usage
pub const GL_STREAM_COPY: GLenum = 0x88E2;

// =============================================================================
// PRIMITIVE TYPES
// =============================================================================

/// Points
pub const GL_POINTS: GLenum = 0x0000;
/// Lines
pub const GL_LINES: GLenum = 0x0001;
/// Line loop
pub const GL_LINE_LOOP: GLenum = 0x0002;
/// Line strip
pub const GL_LINE_STRIP: GLenum = 0x0003;
/// Triangles
pub const GL_TRIANGLES: GLenum = 0x0004;
/// Triangle strip
pub const GL_TRIANGLE_STRIP: GLenum = 0x0005;
/// Triangle fan
pub const GL_TRIANGLE_FAN: GLenum = 0x0006;
/// Lines with adjacency
pub const GL_LINES_ADJACENCY: GLenum = 0x000A;
/// Line strip with adjacency
pub const GL_LINE_STRIP_ADJACENCY: GLenum = 0x000B;
/// Triangles with adjacency
pub const GL_TRIANGLES_ADJACENCY: GLenum = 0x000C;
/// Triangle strip with adjacency
pub const GL_TRIANGLE_STRIP_ADJACENCY: GLenum = 0x000D;
/// Patches (tessellation)
pub const GL_PATCHES: GLenum = 0x000E;

// =============================================================================
// DATA TYPES
// =============================================================================

/// Byte type
pub const GL_BYTE: GLenum = 0x1400;
/// Unsigned byte type
pub const GL_UNSIGNED_BYTE: GLenum = 0x1401;
/// Short type
pub const GL_SHORT: GLenum = 0x1402;
/// Unsigned short type
pub const GL_UNSIGNED_SHORT: GLenum = 0x1403;
/// Int type
pub const GL_INT: GLenum = 0x1404;
/// Unsigned int type
pub const GL_UNSIGNED_INT: GLenum = 0x1405;
/// Float type
pub const GL_FLOAT: GLenum = 0x1406;
/// Double type
pub const GL_DOUBLE: GLenum = 0x140A;
/// Half float type
pub const GL_HALF_FLOAT: GLenum = 0x140B;
/// Fixed type
pub const GL_FIXED: GLenum = 0x140C;

// =============================================================================
// SHADER TYPES
// =============================================================================

/// Vertex shader
pub const GL_VERTEX_SHADER: GLenum = 0x8B31;
/// Fragment shader
pub const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
/// Geometry shader
pub const GL_GEOMETRY_SHADER: GLenum = 0x8DD9;
/// Tessellation control shader
pub const GL_TESS_CONTROL_SHADER: GLenum = 0x8E88;
/// Tessellation evaluation shader
pub const GL_TESS_EVALUATION_SHADER: GLenum = 0x8E87;
/// Compute shader
pub const GL_COMPUTE_SHADER: GLenum = 0x91B9;

// =============================================================================
// SHADER PARAMETERS
// =============================================================================

/// Shader type query
pub const GL_SHADER_TYPE: GLenum = 0x8B4F;
/// Delete status query
pub const GL_DELETE_STATUS: GLenum = 0x8B80;
/// Compile status query
pub const GL_COMPILE_STATUS: GLenum = 0x8B81;
/// Link status query
pub const GL_LINK_STATUS: GLenum = 0x8B82;
/// Validate status query
pub const GL_VALIDATE_STATUS: GLenum = 0x8B83;
/// Info log length query
pub const GL_INFO_LOG_LENGTH: GLenum = 0x8B84;
/// Attached shaders count
pub const GL_ATTACHED_SHADERS: GLenum = 0x8B85;
/// Active uniforms count
pub const GL_ACTIVE_UNIFORMS: GLenum = 0x8B86;
/// Active uniform max length
pub const GL_ACTIVE_UNIFORM_MAX_LENGTH: GLenum = 0x8B87;
/// Shader source length
pub const GL_SHADER_SOURCE_LENGTH: GLenum = 0x8B88;
/// Active attributes count
pub const GL_ACTIVE_ATTRIBUTES: GLenum = 0x8B89;
/// Active attribute max length
pub const GL_ACTIVE_ATTRIBUTE_MAX_LENGTH: GLenum = 0x8B8A;

// =============================================================================
// TEXTURE TARGETS
// =============================================================================

/// 1D texture
pub const GL_TEXTURE_1D: GLenum = 0x0DE0;
/// 2D texture
pub const GL_TEXTURE_2D: GLenum = 0x0DE1;
/// 3D texture
pub const GL_TEXTURE_3D: GLenum = 0x806F;
/// Cube map texture
pub const GL_TEXTURE_CUBE_MAP: GLenum = 0x8513;
/// 1D texture array
pub const GL_TEXTURE_1D_ARRAY: GLenum = 0x8C18;
/// 2D texture array
pub const GL_TEXTURE_2D_ARRAY: GLenum = 0x8C1A;
/// 2D multisample texture
pub const GL_TEXTURE_2D_MULTISAMPLE: GLenum = 0x9100;
/// 2D multisample array texture
pub const GL_TEXTURE_2D_MULTISAMPLE_ARRAY: GLenum = 0x9102;
/// Rectangle texture
pub const GL_TEXTURE_RECTANGLE: GLenum = 0x84F5;
/// Buffer texture
pub const GL_TEXTURE_BUFFER: GLenum = 0x8C2A;
/// Cube map array texture
pub const GL_TEXTURE_CUBE_MAP_ARRAY: GLenum = 0x9009;

// =============================================================================
// TEXTURE PARAMETERS
// =============================================================================

/// Minification filter
pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
/// Magnification filter
pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
/// S coordinate wrap mode
pub const GL_TEXTURE_WRAP_S: GLenum = 0x2802;
/// T coordinate wrap mode
pub const GL_TEXTURE_WRAP_T: GLenum = 0x2803;
/// R coordinate wrap mode
pub const GL_TEXTURE_WRAP_R: GLenum = 0x8072;
/// Border color
pub const GL_TEXTURE_BORDER_COLOR: GLenum = 0x1004;
/// Min LOD
pub const GL_TEXTURE_MIN_LOD: GLenum = 0x813A;
/// Max LOD
pub const GL_TEXTURE_MAX_LOD: GLenum = 0x813B;
/// Base level
pub const GL_TEXTURE_BASE_LEVEL: GLenum = 0x813C;
/// Max level
pub const GL_TEXTURE_MAX_LEVEL: GLenum = 0x813D;
/// Compare mode
pub const GL_TEXTURE_COMPARE_MODE: GLenum = 0x884C;
/// Compare func
pub const GL_TEXTURE_COMPARE_FUNC: GLenum = 0x884D;
/// Max anisotropy
pub const GL_TEXTURE_MAX_ANISOTROPY: GLenum = 0x84FE;

// =============================================================================
// TEXTURE FILTER VALUES
// =============================================================================

/// Nearest filter
pub const GL_NEAREST: GLenum = 0x2600;
/// Linear filter
pub const GL_LINEAR: GLenum = 0x2601;
/// Nearest mipmap nearest
pub const GL_NEAREST_MIPMAP_NEAREST: GLenum = 0x2700;
/// Linear mipmap nearest
pub const GL_LINEAR_MIPMAP_NEAREST: GLenum = 0x2701;
/// Nearest mipmap linear
pub const GL_NEAREST_MIPMAP_LINEAR: GLenum = 0x2702;
/// Linear mipmap linear
pub const GL_LINEAR_MIPMAP_LINEAR: GLenum = 0x2703;

// =============================================================================
// TEXTURE WRAP VALUES
// =============================================================================

/// Repeat wrap mode
pub const GL_REPEAT: GLenum = 0x2901;
/// Clamp to edge wrap mode
pub const GL_CLAMP_TO_EDGE: GLenum = 0x812F;
/// Clamp to border wrap mode
pub const GL_CLAMP_TO_BORDER: GLenum = 0x812D;
/// Mirrored repeat wrap mode
pub const GL_MIRRORED_REPEAT: GLenum = 0x8370;
/// Mirror clamp to edge
pub const GL_MIRROR_CLAMP_TO_EDGE: GLenum = 0x8743;

// =============================================================================
// PIXEL FORMATS
// =============================================================================

/// Red format
pub const GL_RED: GLenum = 0x1903;
/// RG format
pub const GL_RG: GLenum = 0x8227;
/// RGB format
pub const GL_RGB: GLenum = 0x1907;
/// RGBA format
pub const GL_RGBA: GLenum = 0x1908;
/// BGR format
pub const GL_BGR: GLenum = 0x80E0;
/// BGRA format
pub const GL_BGRA: GLenum = 0x80E1;
/// Depth component format
pub const GL_DEPTH_COMPONENT: GLenum = 0x1902;
/// Depth stencil format
pub const GL_DEPTH_STENCIL: GLenum = 0x84F9;
/// Stencil index format
pub const GL_STENCIL_INDEX: GLenum = 0x1901;

// =============================================================================
// INTERNAL FORMATS
// =============================================================================

/// R8 format
pub const GL_R8: GLenum = 0x8229;
/// RG8 format
pub const GL_RG8: GLenum = 0x822B;
/// RGB8 format
pub const GL_RGB8: GLenum = 0x8051;
/// RGBA8 format
pub const GL_RGBA8: GLenum = 0x8058;
/// R16F format
pub const GL_R16F: GLenum = 0x822D;
/// RG16F format
pub const GL_RG16F: GLenum = 0x822F;
/// RGB16F format
pub const GL_RGB16F: GLenum = 0x881B;
/// RGBA16F format
pub const GL_RGBA16F: GLenum = 0x881A;
/// R32F format
pub const GL_R32F: GLenum = 0x822E;
/// RG32F format
pub const GL_RG32F: GLenum = 0x8230;
/// RGB32F format
pub const GL_RGB32F: GLenum = 0x8815;
/// RGBA32F format
pub const GL_RGBA32F: GLenum = 0x8814;
/// Depth 16 format
pub const GL_DEPTH_COMPONENT16: GLenum = 0x81A5;
/// Depth 24 format
pub const GL_DEPTH_COMPONENT24: GLenum = 0x81A6;
/// Depth 32F format
pub const GL_DEPTH_COMPONENT32F: GLenum = 0x8CAC;
/// Depth 24 stencil 8 format
pub const GL_DEPTH24_STENCIL8: GLenum = 0x88F0;
/// Depth 32F stencil 8 format
pub const GL_DEPTH32F_STENCIL8: GLenum = 0x8CAD;
/// sRGB8 format
pub const GL_SRGB8: GLenum = 0x8C41;
/// sRGB8 alpha8 format
pub const GL_SRGB8_ALPHA8: GLenum = 0x8C43;

// =============================================================================
// FRAMEBUFFER
// =============================================================================

/// Framebuffer target
pub const GL_FRAMEBUFFER: GLenum = 0x8D40;
/// Read framebuffer target
pub const GL_READ_FRAMEBUFFER: GLenum = 0x8CA8;
/// Draw framebuffer target
pub const GL_DRAW_FRAMEBUFFER: GLenum = 0x8CA9;
/// Renderbuffer target
pub const GL_RENDERBUFFER: GLenum = 0x8D41;
/// Color attachment 0
pub const GL_COLOR_ATTACHMENT0: GLenum = 0x8CE0;
/// Depth attachment
pub const GL_DEPTH_ATTACHMENT: GLenum = 0x8D00;
/// Stencil attachment
pub const GL_STENCIL_ATTACHMENT: GLenum = 0x8D20;
/// Depth stencil attachment
pub const GL_DEPTH_STENCIL_ATTACHMENT: GLenum = 0x821A;

/// Framebuffer complete status
pub const GL_FRAMEBUFFER_COMPLETE: GLenum = 0x8CD5;
/// Framebuffer incomplete attachment
pub const GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT: GLenum = 0x8CD6;
/// Framebuffer incomplete missing attachment
pub const GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT: GLenum = 0x8CD7;
/// Framebuffer incomplete draw buffer
pub const GL_FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER: GLenum = 0x8CDB;
/// Framebuffer incomplete read buffer
pub const GL_FRAMEBUFFER_INCOMPLETE_READ_BUFFER: GLenum = 0x8CDC;
/// Framebuffer unsupported
pub const GL_FRAMEBUFFER_UNSUPPORTED: GLenum = 0x8CDD;
/// Framebuffer incomplete multisample
pub const GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE: GLenum = 0x8D56;
/// Framebuffer incomplete layer targets
pub const GL_FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS: GLenum = 0x8DA8;

// =============================================================================
// CLEAR BITS
// =============================================================================

/// Color buffer bit
pub const GL_COLOR_BUFFER_BIT: GLenum = 0x00004000;
/// Depth buffer bit
pub const GL_DEPTH_BUFFER_BIT: GLenum = 0x00000100;
/// Stencil buffer bit
pub const GL_STENCIL_BUFFER_BIT: GLenum = 0x00000400;

// =============================================================================
// ENABLE/DISABLE CAPS
// =============================================================================

/// Blend capability
pub const GL_BLEND: GLenum = 0x0BE2;
/// Cull face capability
pub const GL_CULL_FACE: GLenum = 0x0B44;
/// Depth test capability
pub const GL_DEPTH_TEST: GLenum = 0x0B71;
/// Dither capability
pub const GL_DITHER: GLenum = 0x0BD0;
/// Polygon offset fill capability
pub const GL_POLYGON_OFFSET_FILL: GLenum = 0x8037;
/// Polygon offset line capability
pub const GL_POLYGON_OFFSET_LINE: GLenum = 0x2A02;
/// Polygon offset point capability
pub const GL_POLYGON_OFFSET_POINT: GLenum = 0x2A01;
/// Scissor test capability
pub const GL_SCISSOR_TEST: GLenum = 0x0C11;
/// Stencil test capability
pub const GL_STENCIL_TEST: GLenum = 0x0B90;
/// Multisample capability
pub const GL_MULTISAMPLE: GLenum = 0x809D;
/// Program point size capability
pub const GL_PROGRAM_POINT_SIZE: GLenum = 0x8642;
/// Line smooth capability
pub const GL_LINE_SMOOTH: GLenum = 0x0B20;
/// Primitive restart capability
pub const GL_PRIMITIVE_RESTART: GLenum = 0x8F9D;
/// Primitive restart fixed index
pub const GL_PRIMITIVE_RESTART_FIXED_INDEX: GLenum = 0x8D69;
/// Rasterizer discard capability
pub const GL_RASTERIZER_DISCARD: GLenum = 0x8C89;

// =============================================================================
// FACE MODES
// =============================================================================

/// Front face
pub const GL_FRONT: GLenum = 0x0404;
/// Back face
pub const GL_BACK: GLenum = 0x0405;
/// Front and back faces
pub const GL_FRONT_AND_BACK: GLenum = 0x0408;
/// Clockwise winding
pub const GL_CW: GLenum = 0x0900;
/// Counter-clockwise winding
pub const GL_CCW: GLenum = 0x0901;

// =============================================================================
// BLEND FUNCTIONS
// =============================================================================

/// Zero blend factor
pub const GL_ZERO: GLenum = 0;
/// One blend factor
pub const GL_ONE: GLenum = 1;
/// Source color blend factor
pub const GL_SRC_COLOR: GLenum = 0x0300;
/// One minus source color blend factor
pub const GL_ONE_MINUS_SRC_COLOR: GLenum = 0x0301;
/// Destination color blend factor
pub const GL_DST_COLOR: GLenum = 0x0306;
/// One minus destination color blend factor
pub const GL_ONE_MINUS_DST_COLOR: GLenum = 0x0307;
/// Source alpha blend factor
pub const GL_SRC_ALPHA: GLenum = 0x0302;
/// One minus source alpha blend factor
pub const GL_ONE_MINUS_SRC_ALPHA: GLenum = 0x0303;
/// Destination alpha blend factor
pub const GL_DST_ALPHA: GLenum = 0x0304;
/// One minus destination alpha blend factor
pub const GL_ONE_MINUS_DST_ALPHA: GLenum = 0x0305;
/// Constant color blend factor
pub const GL_CONSTANT_COLOR: GLenum = 0x8001;
/// One minus constant color blend factor
pub const GL_ONE_MINUS_CONSTANT_COLOR: GLenum = 0x8002;
/// Constant alpha blend factor
pub const GL_CONSTANT_ALPHA: GLenum = 0x8003;
/// One minus constant alpha blend factor
pub const GL_ONE_MINUS_CONSTANT_ALPHA: GLenum = 0x8004;
/// Source alpha saturate blend factor
pub const GL_SRC_ALPHA_SATURATE: GLenum = 0x0308;

// =============================================================================
// BLEND EQUATIONS
// =============================================================================

/// Add blend equation
pub const GL_FUNC_ADD: GLenum = 0x8006;
/// Subtract blend equation
pub const GL_FUNC_SUBTRACT: GLenum = 0x800A;
/// Reverse subtract blend equation
pub const GL_FUNC_REVERSE_SUBTRACT: GLenum = 0x800B;
/// Min blend equation
pub const GL_MIN: GLenum = 0x8007;
/// Max blend equation
pub const GL_MAX: GLenum = 0x8008;

// =============================================================================
// COMPARISON FUNCTIONS
// =============================================================================

/// Never pass
pub const GL_NEVER: GLenum = 0x0200;
/// Less than
pub const GL_LESS: GLenum = 0x0201;
/// Equal
pub const GL_EQUAL: GLenum = 0x0202;
/// Less than or equal
pub const GL_LEQUAL: GLenum = 0x0203;
/// Greater than
pub const GL_GREATER: GLenum = 0x0204;
/// Not equal
pub const GL_NOTEQUAL: GLenum = 0x0205;
/// Greater than or equal
pub const GL_GEQUAL: GLenum = 0x0206;
/// Always pass
pub const GL_ALWAYS: GLenum = 0x0207;

// =============================================================================
// STRING QUERIES
// =============================================================================

/// Vendor string
pub const GL_VENDOR: GLenum = 0x1F00;
/// Renderer string
pub const GL_RENDERER: GLenum = 0x1F01;
/// Version string
pub const GL_VERSION: GLenum = 0x1F02;
/// Shading language version string
pub const GL_SHADING_LANGUAGE_VERSION: GLenum = 0x8B8C;
/// Extensions string
pub const GL_EXTENSIONS: GLenum = 0x1F03;
