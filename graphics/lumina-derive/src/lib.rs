//! # LUMINA Derive - Procedural Macros for Shader Development
//!
//! This crate provides procedural macros for the LUMINA shader compilation system,
//! enabling Rust-native shader development with compile-time validation.
//!
//! ## Core Macros
//!
//! - `#[lumina::shader]` - Mark a module as a shader program
//! - `#[vertex]` - Mark a function as a vertex shader entry point
//! - `#[fragment]` - Mark a function as a fragment shader entry point
//! - `#[compute]` - Mark a function as a compute shader entry point
//! - `#[geometry]` - Mark a function as a geometry shader entry point
//! - `#[tessellation_control]` - Mark a function as a tessellation control shader
//! - `#[tessellation_evaluation]` - Mark a function as a tessellation evaluation shader
//! - `#[mesh]` - Mark a function as a mesh shader entry point
//! - `#[task]` - Mark a function as a task shader entry point
//! - `#[ray_generation]` - Mark a function as a ray generation shader
//! - `#[closest_hit]` - Mark a function as a closest hit shader
//! - `#[any_hit]` - Mark a function as an any hit shader
//! - `#[miss]` - Mark a function as a miss shader
//! - `#[intersection]` - Mark a function as an intersection shader
//! - `#[callable]` - Mark a function as a callable shader
//!
//! ## Resource Binding Macros
//!
//! - `#[uniform(binding = N, set = M)]` - Uniform buffer binding
//! - `#[storage(binding = N, set = M)]` - Storage buffer binding
//! - `#[texture(binding = N, set = M)]` - Texture binding
//! - `#[sampler(binding = N, set = M)]` - Sampler binding
//! - `#[push_constant]` - Push constant block
//!
//! ## Input/Output Macros
//!
//! - `#[location(N)]` - Specify shader input/output location
//! - `#[builtin(Position)]` - Access built-in variables
//!
//! ## Example
//!
//! ```ignore
//! use lumina_derive::*;
//!
//! #[lumina::shader]
//! mod my_shader {
//!     #[uniform(binding = 0, set = 0)]
//!     struct Uniforms {
//!         mvp: Mat4,
//!         time: f32,
//!     }
//!
//!     #[vertex]
//!     fn vertex_main(
//!         #[location(0)] position: Vec3,
//!         #[location(1)] uv: Vec2,
//!     ) -> VertexOutput {
//!         VertexOutput {
//!             position: uniforms.mvp * vec4(position, 1.0),
//!             uv,
//!         }
//!     }
//!
//!     #[fragment]
//!     fn fragment_main(input: VertexOutput) -> Vec4 {
//!         texture(albedo, input.uv)
//!     }
//! }
//! ```

extern crate proc_macro;

mod analyze;
mod codegen;
mod error;
mod ir_gen;
mod parse;
mod shader;
mod spirv_gen;
mod types;
mod validate;

use proc_macro::TokenStream;

/// Main shader module attribute macro.
///
/// This macro processes an entire module as a shader program, generating:
/// - SPIR-V bytecode for each shader stage
/// - Reflection data for resource bindings
/// - Type-safe Rust bindings for CPU-side usage
///
/// # Attributes
///
/// - `target = "vulkan1.2"` - Target Vulkan version (default: vulkan1.2)
/// - `optimize = "performance"` - Optimization level: none, size, performance, aggressive
/// - `debug = true` - Include debug information in SPIR-V
/// - `validate = true` - Enable validation (default: true)
///
/// # Example
///
/// ```ignore
/// #[lumina::shader(target = "vulkan1.2", optimize = "performance")]
/// mod pbr_shader {
///     // Shader code here
/// }
/// ```
#[proc_macro_attribute]
pub fn shader(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::shader_impl(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Mark a function as a vertex shader entry point.
///
/// # Parameters
///
/// Parameters with `#[location(N)]` are vertex inputs.
/// The return type defines vertex outputs passed to fragment shader.
///
/// # Built-in Outputs
///
/// Use `#[builtin(Position)]` for `gl_Position` equivalent.
///
/// # Example
///
/// ```ignore
/// #[vertex]
/// fn main(
///     #[location(0)] position: Vec3,
///     #[location(1)] normal: Vec3,
/// ) -> VertexOutput {
///     // Transform vertices
/// }
/// ```
#[proc_macro_attribute]
pub fn vertex(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Vertex,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a fragment shader entry point.
///
/// # Parameters
///
/// Input parameters come from vertex shader interpolated outputs.
/// Return type is the fragment color output(s).
///
/// # Example
///
/// ```ignore
/// #[fragment]
/// fn main(input: VertexOutput) -> Vec4 {
///     vec4(input.color, 1.0)
/// }
/// ```
#[proc_macro_attribute]
pub fn fragment(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Fragment,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a compute shader entry point.
///
/// # Attributes
///
/// - `local_size = (x, y, z)` - Workgroup size (required)
///
/// # Example
///
/// ```ignore
/// #[compute(local_size = (64, 1, 1))]
/// fn main() {
///     let id = global_invocation_id();
///     // Compute work
/// }
/// ```
#[proc_macro_attribute]
pub fn compute(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Compute,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a geometry shader entry point.
///
/// # Attributes
///
/// - `input = triangles` - Input primitive type
/// - `output = triangle_strip` - Output primitive type
/// - `max_vertices = N` - Maximum output vertices
///
/// # Example
///
/// ```ignore
/// #[geometry(input = triangles, output = triangle_strip, max_vertices = 3)]
/// fn main(vertices: [VertexInput; 3]) {
///     // Process geometry
/// }
/// ```
#[proc_macro_attribute]
pub fn geometry(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Geometry,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a tessellation control shader entry point.
///
/// # Attributes
///
/// - `output_vertices = N` - Number of output control points
///
/// # Example
///
/// ```ignore
/// #[tessellation_control(output_vertices = 3)]
/// fn main() {
///     // Set tessellation levels
/// }
/// ```
#[proc_macro_attribute]
pub fn tessellation_control(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::TessellationControl,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a tessellation evaluation shader entry point.
///
/// # Attributes
///
/// - `mode = triangles` - Tessellation mode: triangles, quads, isolines
/// - `spacing = equal` - Spacing: equal, fractional_even, fractional_odd
/// - `winding = ccw` - Winding order: cw, ccw
///
/// # Example
///
/// ```ignore
/// #[tessellation_evaluation(mode = triangles, spacing = equal, winding = ccw)]
/// fn main() -> Vec4 {
///     // Evaluate tessellated position
/// }
/// ```
#[proc_macro_attribute]
pub fn tessellation_evaluation(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::TessellationEvaluation,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a mesh shader entry point.
///
/// # Attributes
///
/// - `local_size = (x, y, z)` - Workgroup size
/// - `max_vertices = N` - Maximum output vertices
/// - `max_primitives = N` - Maximum output primitives
/// - `output = triangles` - Output primitive type
///
/// # Example
///
/// ```ignore
/// #[mesh(local_size = (32, 1, 1), max_vertices = 64, max_primitives = 126, output = triangles)]
/// fn main() {
///     // Generate mesh
/// }
/// ```
#[proc_macro_attribute]
pub fn mesh(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Mesh,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a task shader entry point.
///
/// # Attributes
///
/// - `local_size = (x, y, z)` - Workgroup size
///
/// # Example
///
/// ```ignore
/// #[task(local_size = (32, 1, 1))]
/// fn main() {
///     // Dispatch mesh shaders
///     emit_mesh_tasks(mesh_count, 1, 1);
/// }
/// ```
#[proc_macro_attribute]
pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Task,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a ray generation shader entry point.
///
/// # Example
///
/// ```ignore
/// #[ray_generation]
/// fn main() {
///     let ray = Ray::new(origin, direction);
///     trace_ray(acceleration_structure, ray, payload);
/// }
/// ```
#[proc_macro_attribute]
pub fn ray_generation(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::RayGeneration,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a closest hit shader entry point.
///
/// # Example
///
/// ```ignore
/// #[closest_hit]
/// fn main(hit: HitAttributes, payload: &mut RayPayload) {
///     payload.color = compute_lighting(hit);
/// }
/// ```
#[proc_macro_attribute]
pub fn closest_hit(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::ClosestHit,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as an any hit shader entry point.
///
/// # Example
///
/// ```ignore
/// #[any_hit]
/// fn main(hit: HitAttributes, payload: &mut RayPayload) {
///     if is_transparent(hit) {
///         ignore_intersection();
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn any_hit(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::AnyHit,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a miss shader entry point.
///
/// # Example
///
/// ```ignore
/// #[miss]
/// fn main(payload: &mut RayPayload) {
///     payload.color = sample_environment(ray_direction());
/// }
/// ```
#[proc_macro_attribute]
pub fn miss(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Miss,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as an intersection shader entry point.
///
/// # Example
///
/// ```ignore
/// #[intersection]
/// fn main() {
///     if let Some(t) = intersect_sphere(ray, sphere) {
///         report_intersection(t, HIT_KIND_SPHERE);
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn intersection(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Intersection,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a function as a callable shader entry point.
///
/// # Example
///
/// ```ignore
/// #[callable]
/// fn shade_material(material_id: u32, data: &mut ShadeData) {
///     // Complex material shading
/// }
/// ```
#[proc_macro_attribute]
pub fn callable(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::entry_point_impl(
        shader::ShaderStage::Callable,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a struct as a uniform buffer.
///
/// # Attributes
///
/// - `binding = N` - Binding number (required)
/// - `set = M` - Descriptor set (default: 0)
/// - `layout = std140` - Memory layout: std140, std430, scalar
///
/// # Example
///
/// ```ignore
/// #[uniform(binding = 0, set = 0, layout = std140)]
/// struct CameraData {
///     view: Mat4,
///     projection: Mat4,
///     position: Vec3,
/// }
/// ```
#[proc_macro_attribute]
pub fn uniform(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::resource_impl(
        shader::ResourceKind::UniformBuffer,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a struct as a storage buffer.
///
/// # Attributes
///
/// - `binding = N` - Binding number (required)
/// - `set = M` - Descriptor set (default: 0)
/// - `access = read_write` - Access mode: read, write, read_write
/// - `layout = std430` - Memory layout: std430, scalar
///
/// # Example
///
/// ```ignore
/// #[storage(binding = 0, set = 1, access = read_write)]
/// struct ParticleBuffer {
///     count: u32,
///     particles: [Particle],
/// }
/// ```
#[proc_macro_attribute]
pub fn storage(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::resource_impl(
        shader::ResourceKind::StorageBuffer,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a field as a texture binding.
///
/// # Attributes
///
/// - `binding = N` - Binding number (required)
/// - `set = M` - Descriptor set (default: 0)
/// - `dim = 2d` - Texture dimension: 1d, 2d, 3d, cube, 2d_array, cube_array
/// - `format = rgba8` - Image format (for storage images)
///
/// # Example
///
/// ```ignore
/// #[texture(binding = 0, set = 0, dim = 2d)]
/// albedo: Texture2D<Vec4>,
///
/// #[texture(binding = 1, set = 0, dim = cube)]
/// environment: TextureCube<Vec4>,
/// ```
#[proc_macro_attribute]
pub fn texture(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::resource_impl(
        shader::ResourceKind::SampledImage,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a field as a sampler binding.
///
/// # Attributes
///
/// - `binding = N` - Binding number (required)
/// - `set = M` - Descriptor set (default: 0)
///
/// # Example
///
/// ```ignore
/// #[sampler(binding = 0, set = 0)]
/// linear_sampler: Sampler,
/// ```
#[proc_macro_attribute]
pub fn sampler(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::resource_impl(
        shader::ResourceKind::Sampler,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a struct as a push constant block.
///
/// Push constants are small, frequently updated data sent directly in command buffers.
///
/// # Constraints
///
/// - Maximum size is typically 128-256 bytes (GPU-dependent)
/// - Only one push constant block per shader stage
///
/// # Example
///
/// ```ignore
/// #[push_constant]
/// struct PushData {
///     model_matrix: Mat4,
///     material_id: u32,
/// }
/// ```
#[proc_macro_attribute]
pub fn push_constant(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::resource_impl(
        shader::ResourceKind::PushConstant,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Specify the location of a shader input or output.
///
/// # Example
///
/// ```ignore
/// fn vertex_main(
///     #[location(0)] position: Vec3,
///     #[location(1)] normal: Vec3,
///     #[location(2)] uv: Vec2,
/// ) -> VertexOutput { ... }
/// ```
#[proc_macro_attribute]
pub fn location(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::location_impl(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Access a built-in shader variable.
///
/// # Built-in Variables
///
/// ## Vertex Shader
/// - `VertexIndex` - `gl_VertexIndex`
/// - `InstanceIndex` - `gl_InstanceIndex`
/// - `Position` (output) - `gl_Position`
///
/// ## Fragment Shader
/// - `FragCoord` - `gl_FragCoord`
/// - `FrontFacing` - `gl_FrontFacing`
/// - `PointCoord` - `gl_PointCoord`
/// - `FragDepth` (output) - `gl_FragDepth`
///
/// ## Compute Shader
/// - `GlobalInvocationId` - `gl_GlobalInvocationID`
/// - `LocalInvocationId` - `gl_LocalInvocationID`
/// - `WorkGroupId` - `gl_WorkGroupID`
/// - `NumWorkGroups` - `gl_NumWorkGroups`
/// - `LocalInvocationIndex` - `gl_LocalInvocationIndex`
///
/// # Example
///
/// ```ignore
/// fn vertex_main(
///     #[builtin(VertexIndex)] vertex_id: u32,
/// ) -> #[builtin(Position)] Vec4 { ... }
/// ```
#[proc_macro_attribute]
pub fn builtin(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::builtin_impl(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Mark a struct as shader input interface.
///
/// # Example
///
/// ```ignore
/// #[input]
/// struct VertexInput {
///     #[location(0)] position: Vec3,
///     #[location(1)] normal: Vec3,
/// }
/// ```
#[proc_macro_attribute]
pub fn input(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::interface_impl(
        shader::InterfaceKind::Input,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a struct as shader output interface.
///
/// # Example
///
/// ```ignore
/// #[output]
/// struct VertexOutput {
///     #[location(0)] world_pos: Vec3,
///     #[location(1)] uv: Vec2,
///     #[builtin(Position)] position: Vec4,
/// }
/// ```
#[proc_macro_attribute]
pub fn output(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::interface_impl(
        shader::InterfaceKind::Output,
        attr.into(),
        item.into(),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

/// Mark a variable as shared workgroup memory.
///
/// Only valid in compute, mesh, and task shaders.
///
/// # Example
///
/// ```ignore
/// #[shared]
/// static TILE_DATA: [Vec4; 256];
/// ```
#[proc_macro_attribute]
pub fn shared(attr: TokenStream, item: TokenStream) -> TokenStream {
    shader::shared_impl(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Inline SPIR-V assembly.
///
/// For advanced use cases requiring direct SPIR-V control.
///
/// # Example
///
/// ```ignore
/// let result = spirv_asm! {
///     "%1 = OpLoad %float %input",
///     "%2 = OpFMul %float %1 %scale",
///     "OpStore %output %2"
/// };
/// ```
#[proc_macro]
pub fn spirv_asm(input: TokenStream) -> TokenStream {
    shader::spirv_asm_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Include pre-compiled SPIR-V binary.
///
/// # Example
///
/// ```ignore
/// const SHADER: &[u32] = include_spirv!("shaders/compiled/vertex.spv");
/// ```
#[proc_macro]
pub fn include_spirv(input: TokenStream) -> TokenStream {
    shader::include_spirv_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for shader-compatible structs.
///
/// Generates GPU memory layout and type information.
///
/// # Example
///
/// ```ignore
/// #[derive(ShaderType)]
/// #[repr(C)]
/// struct Material {
///     albedo: Vec4,
///     metallic: f32,
///     roughness: f32,
/// }
/// ```
#[proc_macro_derive(ShaderType, attributes(shader))]
pub fn derive_shader_type(input: TokenStream) -> TokenStream {
    types::derive_shader_type_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for vertex buffer layout.
///
/// Generates vertex attribute descriptions and stride.
///
/// # Example
///
/// ```ignore
/// #[derive(VertexInput)]
/// #[repr(C)]
/// struct Vertex {
///     #[location(0)]
///     position: Vec3,
///     #[location(1)]
///     normal: Vec3,
///     #[location(2)]
///     uv: Vec2,
/// }
/// ```
#[proc_macro_derive(VertexInput, attributes(location, format))]
pub fn derive_vertex_input(input: TokenStream) -> TokenStream {
    types::derive_vertex_input_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for push constant layout.
///
/// # Example
///
/// ```ignore
/// #[derive(PushConstant)]
/// #[repr(C)]
/// struct PushData {
///     transform: Mat4,
///     flags: u32,
/// }
/// ```
#[proc_macro_derive(PushConstant)]
pub fn derive_push_constant(input: TokenStream) -> TokenStream {
    types::derive_push_constant_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
