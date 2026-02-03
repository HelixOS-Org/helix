//! Pipeline Builder for Lumina
//!
//! This module provides comprehensive fluent builder patterns for constructing
//! graphics, compute, and ray tracing pipelines with full validation.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Graphics Pipeline Builder
// ============================================================================

/// Graphics pipeline builder with fluent API
#[derive(Clone, Debug)]
pub struct GraphicsPipelineBuilder {
    /// Pipeline create flags
    pub flags: PipelineCreateFlags,
    /// Shader stages
    pub stages: Vec<ShaderStageInfo>,
    /// Vertex input state
    pub vertex_input: VertexInputState,
    /// Input assembly state
    pub input_assembly: InputAssemblyState,
    /// Tessellation state
    pub tessellation: Option<TessellationState>,
    /// Viewport state
    pub viewport: ViewportState,
    /// Rasterization state
    pub rasterization: RasterizationState,
    /// Multisample state
    pub multisample: MultisampleState,
    /// Depth stencil state
    pub depth_stencil: Option<DepthStencilState>,
    /// Color blend state
    pub color_blend: ColorBlendState,
    /// Dynamic states
    pub dynamic_states: Vec<DynamicStateType>,
    /// Pipeline layout
    pub layout: u64,
    /// Render pass
    pub render_pass: u64,
    /// Subpass
    pub subpass: u32,
    /// Base pipeline (for derivatives)
    pub base_pipeline: Option<u64>,
    /// Base pipeline index
    pub base_pipeline_index: i32,
    /// Debug name
    pub debug_name: Option<String>,
}

impl GraphicsPipelineBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            flags: PipelineCreateFlags::NONE,
            stages: Vec::new(),
            vertex_input: VertexInputState::default(),
            input_assembly: InputAssemblyState::default(),
            tessellation: None,
            viewport: ViewportState::default(),
            rasterization: RasterizationState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
            color_blend: ColorBlendState::default(),
            dynamic_states: Vec::new(),
            layout: 0,
            render_pass: 0,
            subpass: 0,
            base_pipeline: None,
            base_pipeline_index: -1,
            debug_name: None,
        }
    }

    /// Set flags
    #[inline]
    pub fn flags(mut self, flags: PipelineCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Add vertex shader
    #[inline]
    pub fn vertex_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::vertex(module, entry));
        self
    }

    /// Add fragment shader
    #[inline]
    pub fn fragment_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::fragment(module, entry));
        self
    }

    /// Add geometry shader
    #[inline]
    pub fn geometry_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::geometry(module, entry));
        self
    }

    /// Add tessellation control shader
    #[inline]
    pub fn tess_control_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::tess_control(module, entry));
        self
    }

    /// Add tessellation evaluation shader
    #[inline]
    pub fn tess_eval_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::tess_eval(module, entry));
        self
    }

    /// Add task shader (mesh shading)
    #[inline]
    pub fn task_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::task(module, entry));
        self
    }

    /// Add mesh shader
    #[inline]
    pub fn mesh_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::mesh(module, entry));
        self
    }

    /// Set vertex input state
    #[inline]
    pub fn vertex_input(mut self, state: VertexInputState) -> Self {
        self.vertex_input = state;
        self
    }

    /// Add vertex binding
    #[inline]
    pub fn add_vertex_binding(mut self, binding: VertexBindingDescription) -> Self {
        self.vertex_input.bindings.push(binding);
        self
    }

    /// Add vertex attribute
    #[inline]
    pub fn add_vertex_attribute(mut self, attribute: VertexAttributeDescription) -> Self {
        self.vertex_input.attributes.push(attribute);
        self
    }

    /// Set input assembly state
    #[inline]
    pub fn input_assembly(mut self, state: InputAssemblyState) -> Self {
        self.input_assembly = state;
        self
    }

    /// Set primitive topology
    #[inline]
    pub fn topology(mut self, topology: PrimitiveTopology) -> Self {
        self.input_assembly.topology = topology;
        self
    }

    /// Enable primitive restart
    #[inline]
    pub fn primitive_restart(mut self, enable: bool) -> Self {
        self.input_assembly.primitive_restart_enable = enable;
        self
    }

    /// Set tessellation state
    #[inline]
    pub fn tessellation(mut self, patch_control_points: u32) -> Self {
        self.tessellation = Some(TessellationState {
            patch_control_points,
        });
        self
    }

    /// Set viewport state
    #[inline]
    pub fn viewport_state(mut self, state: ViewportState) -> Self {
        self.viewport = state;
        self
    }

    /// Add viewport
    #[inline]
    pub fn add_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport.viewports.push(viewport);
        self
    }

    /// Add scissor
    #[inline]
    pub fn add_scissor(mut self, scissor: Scissor) -> Self {
        self.viewport.scissors.push(scissor);
        self
    }

    /// Set rasterization state
    #[inline]
    pub fn rasterization(mut self, state: RasterizationState) -> Self {
        self.rasterization = state;
        self
    }

    /// Set polygon mode
    #[inline]
    pub fn polygon_mode(mut self, mode: PolygonMode) -> Self {
        self.rasterization.polygon_mode = mode;
        self
    }

    /// Set cull mode
    #[inline]
    pub fn cull_mode(mut self, mode: CullModeFlags) -> Self {
        self.rasterization.cull_mode = mode;
        self
    }

    /// Set front face
    #[inline]
    pub fn front_face(mut self, face: FrontFace) -> Self {
        self.rasterization.front_face = face;
        self
    }

    /// Enable depth clamp
    #[inline]
    pub fn depth_clamp(mut self, enable: bool) -> Self {
        self.rasterization.depth_clamp_enable = enable;
        self
    }

    /// Enable rasterizer discard
    #[inline]
    pub fn rasterizer_discard(mut self, enable: bool) -> Self {
        self.rasterization.rasterizer_discard_enable = enable;
        self
    }

    /// Set depth bias
    #[inline]
    pub fn depth_bias(mut self, constant: f32, clamp: f32, slope: f32) -> Self {
        self.rasterization.depth_bias_enable = true;
        self.rasterization.depth_bias_constant_factor = constant;
        self.rasterization.depth_bias_clamp = clamp;
        self.rasterization.depth_bias_slope_factor = slope;
        self
    }

    /// Set line width
    #[inline]
    pub fn line_width(mut self, width: f32) -> Self {
        self.rasterization.line_width = width;
        self
    }

    /// Set multisample state
    #[inline]
    pub fn multisample(mut self, state: MultisampleState) -> Self {
        self.multisample = state;
        self
    }

    /// Set sample count
    #[inline]
    pub fn samples(mut self, count: SampleCount) -> Self {
        self.multisample.rasterization_samples = count;
        self
    }

    /// Enable sample shading
    #[inline]
    pub fn sample_shading(mut self, min_fraction: f32) -> Self {
        self.multisample.sample_shading_enable = true;
        self.multisample.min_sample_shading = min_fraction;
        self
    }

    /// Enable alpha to coverage
    #[inline]
    pub fn alpha_to_coverage(mut self, enable: bool) -> Self {
        self.multisample.alpha_to_coverage_enable = enable;
        self
    }

    /// Set depth stencil state
    #[inline]
    pub fn depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }

    /// Enable depth test
    #[inline]
    pub fn depth_test(mut self, enable: bool) -> Self {
        let state = self.depth_stencil.get_or_insert_with(DepthStencilState::default);
        state.depth_test_enable = enable;
        self
    }

    /// Enable depth write
    #[inline]
    pub fn depth_write(mut self, enable: bool) -> Self {
        let state = self.depth_stencil.get_or_insert_with(DepthStencilState::default);
        state.depth_write_enable = enable;
        self
    }

    /// Set depth compare op
    #[inline]
    pub fn depth_compare(mut self, op: CompareOp) -> Self {
        let state = self.depth_stencil.get_or_insert_with(DepthStencilState::default);
        state.depth_compare_op = op;
        self
    }

    /// Enable stencil test
    #[inline]
    pub fn stencil_test(mut self, enable: bool) -> Self {
        let state = self.depth_stencil.get_or_insert_with(DepthStencilState::default);
        state.stencil_test_enable = enable;
        self
    }

    /// Set color blend state
    #[inline]
    pub fn color_blend(mut self, state: ColorBlendState) -> Self {
        self.color_blend = state;
        self
    }

    /// Add color blend attachment
    #[inline]
    pub fn add_blend_attachment(mut self, attachment: ColorBlendAttachment) -> Self {
        self.color_blend.attachments.push(attachment);
        self
    }

    /// Set blend constants
    #[inline]
    pub fn blend_constants(mut self, constants: [f32; 4]) -> Self {
        self.color_blend.blend_constants = constants;
        self
    }

    /// Add dynamic state
    #[inline]
    pub fn dynamic_state(mut self, state: DynamicStateType) -> Self {
        self.dynamic_states.push(state);
        self
    }

    /// Add multiple dynamic states
    #[inline]
    pub fn dynamic_states(mut self, states: &[DynamicStateType]) -> Self {
        self.dynamic_states.extend_from_slice(states);
        self
    }

    /// Set pipeline layout
    #[inline]
    pub fn layout(mut self, layout: u64) -> Self {
        self.layout = layout;
        self
    }

    /// Set render pass
    #[inline]
    pub fn render_pass(mut self, render_pass: u64, subpass: u32) -> Self {
        self.render_pass = render_pass;
        self.subpass = subpass;
        self
    }

    /// Set base pipeline for derivatives
    #[inline]
    pub fn base_pipeline(mut self, pipeline: u64) -> Self {
        self.base_pipeline = Some(pipeline);
        self.flags = self.flags.union(PipelineCreateFlags::DERIVATIVE);
        self
    }

    /// Set base pipeline index
    #[inline]
    pub fn base_pipeline_index(mut self, index: i32) -> Self {
        self.base_pipeline_index = index;
        self.flags = self.flags.union(PipelineCreateFlags::DERIVATIVE);
        self
    }

    /// Allow derivatives
    #[inline]
    pub fn allow_derivatives(mut self) -> Self {
        self.flags = self.flags.union(PipelineCreateFlags::ALLOW_DERIVATIVES);
        self
    }

    /// Set debug name
    #[inline]
    pub fn name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Build the pipeline create info
    pub fn build(self) -> Result<GraphicsPipelineCreateInfo, PipelineBuildError> {
        // Validate stages
        if self.stages.is_empty() {
            return Err(PipelineBuildError::NoShaderStages);
        }

        // Validate layout
        if self.layout == 0 {
            return Err(PipelineBuildError::NoLayout);
        }

        // Validate render pass (unless dynamic rendering)
        if self.render_pass == 0 && !self.flags.contains(PipelineCreateFlags::RENDERING_FRAGMENT_SHADING_RATE_ATTACHMENT_KHR) {
            // Dynamic rendering might not need a render pass
        }

        Ok(GraphicsPipelineCreateInfo {
            flags: self.flags,
            stages: self.stages,
            vertex_input: self.vertex_input,
            input_assembly: self.input_assembly,
            tessellation: self.tessellation,
            viewport: self.viewport,
            rasterization: self.rasterization,
            multisample: self.multisample,
            depth_stencil: self.depth_stencil,
            color_blend: self.color_blend,
            dynamic_states: self.dynamic_states,
            layout: self.layout,
            render_pass: self.render_pass,
            subpass: self.subpass,
            base_pipeline: self.base_pipeline,
            base_pipeline_index: self.base_pipeline_index,
        })
    }
}

impl Default for GraphicsPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Compute Pipeline Builder
// ============================================================================

/// Compute pipeline builder
#[derive(Clone, Debug)]
pub struct ComputePipelineBuilder {
    /// Flags
    pub flags: PipelineCreateFlags,
    /// Compute shader stage
    pub stage: Option<ShaderStageInfo>,
    /// Pipeline layout
    pub layout: u64,
    /// Base pipeline
    pub base_pipeline: Option<u64>,
    /// Base pipeline index
    pub base_pipeline_index: i32,
    /// Debug name
    pub debug_name: Option<String>,
}

impl ComputePipelineBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            flags: PipelineCreateFlags::NONE,
            stage: None,
            layout: 0,
            base_pipeline: None,
            base_pipeline_index: -1,
            debug_name: None,
        }
    }

    /// Set flags
    #[inline]
    pub fn flags(mut self, flags: PipelineCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set compute shader
    #[inline]
    pub fn shader(mut self, module: u64, entry: &str) -> Self {
        self.stage = Some(ShaderStageInfo::compute(module, entry));
        self
    }

    /// Set pipeline layout
    #[inline]
    pub fn layout(mut self, layout: u64) -> Self {
        self.layout = layout;
        self
    }

    /// Set base pipeline
    #[inline]
    pub fn base_pipeline(mut self, pipeline: u64) -> Self {
        self.base_pipeline = Some(pipeline);
        self.flags = self.flags.union(PipelineCreateFlags::DERIVATIVE);
        self
    }

    /// Allow derivatives
    #[inline]
    pub fn allow_derivatives(mut self) -> Self {
        self.flags = self.flags.union(PipelineCreateFlags::ALLOW_DERIVATIVES);
        self
    }

    /// Set debug name
    #[inline]
    pub fn name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Build
    pub fn build(self) -> Result<ComputePipelineCreateInfo, PipelineBuildError> {
        let stage = self.stage.ok_or(PipelineBuildError::NoShaderStages)?;

        if self.layout == 0 {
            return Err(PipelineBuildError::NoLayout);
        }

        Ok(ComputePipelineCreateInfo {
            flags: self.flags,
            stage,
            layout: self.layout,
            base_pipeline: self.base_pipeline,
            base_pipeline_index: self.base_pipeline_index,
        })
    }
}

impl Default for ComputePipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ray Tracing Pipeline Builder
// ============================================================================

/// Ray tracing pipeline builder
#[derive(Clone, Debug)]
pub struct RayTracingPipelineBuilder {
    /// Flags
    pub flags: PipelineCreateFlags,
    /// Shader stages
    pub stages: Vec<ShaderStageInfo>,
    /// Shader groups
    pub groups: Vec<RayTracingShaderGroup>,
    /// Max recursion depth
    pub max_recursion_depth: u32,
    /// Pipeline layout
    pub layout: u64,
    /// Base pipeline
    pub base_pipeline: Option<u64>,
    /// Debug name
    pub debug_name: Option<String>,
}

impl RayTracingPipelineBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            flags: PipelineCreateFlags::NONE,
            stages: Vec::new(),
            groups: Vec::new(),
            max_recursion_depth: 1,
            layout: 0,
            base_pipeline: None,
            debug_name: None,
        }
    }

    /// Set flags
    #[inline]
    pub fn flags(mut self, flags: PipelineCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Add ray generation shader
    #[inline]
    pub fn raygen_shader(mut self, module: u64, entry: &str) -> Self {
        let stage_index = self.stages.len() as u32;
        self.stages.push(ShaderStageInfo::raygen(module, entry));
        self.groups.push(RayTracingShaderGroup::general(stage_index));
        self
    }

    /// Add miss shader
    #[inline]
    pub fn miss_shader(mut self, module: u64, entry: &str) -> Self {
        let stage_index = self.stages.len() as u32;
        self.stages.push(ShaderStageInfo::miss(module, entry));
        self.groups.push(RayTracingShaderGroup::general(stage_index));
        self
    }

    /// Add closest hit shader
    #[inline]
    pub fn closest_hit_shader(mut self, module: u64, entry: &str) -> Self {
        let stage_index = self.stages.len() as u32;
        self.stages.push(ShaderStageInfo::closest_hit(module, entry));
        self.groups.push(RayTracingShaderGroup::triangles_hit(stage_index));
        self
    }

    /// Add any hit shader
    #[inline]
    pub fn any_hit_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::any_hit(module, entry));
        self
    }

    /// Add intersection shader
    #[inline]
    pub fn intersection_shader(mut self, module: u64, entry: &str) -> Self {
        self.stages.push(ShaderStageInfo::intersection(module, entry));
        self
    }

    /// Add callable shader
    #[inline]
    pub fn callable_shader(mut self, module: u64, entry: &str) -> Self {
        let stage_index = self.stages.len() as u32;
        self.stages.push(ShaderStageInfo::callable(module, entry));
        self.groups.push(RayTracingShaderGroup::general(stage_index));
        self
    }

    /// Add shader group
    #[inline]
    pub fn add_group(mut self, group: RayTracingShaderGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Set max recursion depth
    #[inline]
    pub fn max_recursion(mut self, depth: u32) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Set layout
    #[inline]
    pub fn layout(mut self, layout: u64) -> Self {
        self.layout = layout;
        self
    }

    /// Set debug name
    #[inline]
    pub fn name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Build
    pub fn build(self) -> Result<RayTracingPipelineCreateInfo, PipelineBuildError> {
        if self.stages.is_empty() {
            return Err(PipelineBuildError::NoShaderStages);
        }

        if self.layout == 0 {
            return Err(PipelineBuildError::NoLayout);
        }

        if self.groups.is_empty() {
            return Err(PipelineBuildError::NoShaderGroups);
        }

        Ok(RayTracingPipelineCreateInfo {
            flags: self.flags,
            stages: self.stages,
            groups: self.groups,
            max_recursion_depth: self.max_recursion_depth,
            layout: self.layout,
            base_pipeline: self.base_pipeline,
        })
    }
}

impl Default for RayTracingPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Pipeline Create Info Structures
// ============================================================================

/// Graphics pipeline create info
#[derive(Clone, Debug)]
pub struct GraphicsPipelineCreateInfo {
    /// Flags
    pub flags: PipelineCreateFlags,
    /// Shader stages
    pub stages: Vec<ShaderStageInfo>,
    /// Vertex input state
    pub vertex_input: VertexInputState,
    /// Input assembly state
    pub input_assembly: InputAssemblyState,
    /// Tessellation state
    pub tessellation: Option<TessellationState>,
    /// Viewport state
    pub viewport: ViewportState,
    /// Rasterization state
    pub rasterization: RasterizationState,
    /// Multisample state
    pub multisample: MultisampleState,
    /// Depth stencil state
    pub depth_stencil: Option<DepthStencilState>,
    /// Color blend state
    pub color_blend: ColorBlendState,
    /// Dynamic states
    pub dynamic_states: Vec<DynamicStateType>,
    /// Layout
    pub layout: u64,
    /// Render pass
    pub render_pass: u64,
    /// Subpass
    pub subpass: u32,
    /// Base pipeline
    pub base_pipeline: Option<u64>,
    /// Base pipeline index
    pub base_pipeline_index: i32,
}

/// Compute pipeline create info
#[derive(Clone, Debug)]
pub struct ComputePipelineCreateInfo {
    /// Flags
    pub flags: PipelineCreateFlags,
    /// Stage
    pub stage: ShaderStageInfo,
    /// Layout
    pub layout: u64,
    /// Base pipeline
    pub base_pipeline: Option<u64>,
    /// Base pipeline index
    pub base_pipeline_index: i32,
}

/// Ray tracing pipeline create info
#[derive(Clone, Debug)]
pub struct RayTracingPipelineCreateInfo {
    /// Flags
    pub flags: PipelineCreateFlags,
    /// Stages
    pub stages: Vec<ShaderStageInfo>,
    /// Groups
    pub groups: Vec<RayTracingShaderGroup>,
    /// Max recursion depth
    pub max_recursion_depth: u32,
    /// Layout
    pub layout: u64,
    /// Base pipeline
    pub base_pipeline: Option<u64>,
}

// ============================================================================
// Pipeline Create Flags
// ============================================================================

/// Pipeline create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineCreateFlags(pub u32);

impl PipelineCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable optimization
    pub const DISABLE_OPTIMIZATION: Self = Self(1 << 0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 1);
    /// Derivative
    pub const DERIVATIVE: Self = Self(1 << 2);
    /// View index from device index
    pub const VIEW_INDEX_FROM_DEVICE_INDEX: Self = Self(1 << 3);
    /// Dispatch base
    pub const DISPATCH_BASE: Self = Self(1 << 4);
    /// Fail on pipeline compile required
    pub const FAIL_ON_PIPELINE_COMPILE_REQUIRED: Self = Self(1 << 8);
    /// Early return on failure
    pub const EARLY_RETURN_ON_FAILURE: Self = Self(1 << 9);
    /// Rendering fragment shading rate attachment KHR
    pub const RENDERING_FRAGMENT_SHADING_RATE_ATTACHMENT_KHR: Self = Self(1 << 21);
    /// Rendering fragment density map attachment EXT
    pub const RENDERING_FRAGMENT_DENSITY_MAP_ATTACHMENT_EXT: Self = Self(1 << 22);
    /// Ray tracing no null any hit shaders KHR
    pub const RAY_TRACING_NO_NULL_ANY_HIT_SHADERS_KHR: Self = Self(1 << 14);
    /// Ray tracing no null closest hit shaders KHR
    pub const RAY_TRACING_NO_NULL_CLOSEST_HIT_SHADERS_KHR: Self = Self(1 << 15);
    /// Ray tracing no null miss shaders KHR
    pub const RAY_TRACING_NO_NULL_MISS_SHADERS_KHR: Self = Self(1 << 16);
    /// Ray tracing no null intersection shaders KHR
    pub const RAY_TRACING_NO_NULL_INTERSECTION_SHADERS_KHR: Self = Self(1 << 17);
    /// Ray tracing skip triangles KHR
    pub const RAY_TRACING_SKIP_TRIANGLES_KHR: Self = Self(1 << 12);
    /// Ray tracing skip AABBs KHR
    pub const RAY_TRACING_SKIP_AABBS_KHR: Self = Self(1 << 13);
    /// Library KHR
    pub const LIBRARY_KHR: Self = Self(1 << 11);
    /// Retain link time optimization info EXT
    pub const RETAIN_LINK_TIME_OPTIMIZATION_INFO_EXT: Self = Self(1 << 23);
    /// Link time optimization EXT
    pub const LINK_TIME_OPTIMIZATION_EXT: Self = Self(1 << 10);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Shader Stage Info
// ============================================================================

/// Shader stage info
#[derive(Clone, Debug)]
pub struct ShaderStageInfo {
    /// Stage
    pub stage: ShaderStage,
    /// Module handle
    pub module: u64,
    /// Entry point name
    pub entry_point: String,
    /// Specialization constants
    pub specialization: Vec<SpecializationConstant>,
}

impl ShaderStageInfo {
    /// Creates new stage info
    fn new(stage: ShaderStage, module: u64, entry: &str) -> Self {
        Self {
            stage,
            module,
            entry_point: String::from(entry),
            specialization: Vec::new(),
        }
    }

    /// Vertex shader
    #[inline]
    pub fn vertex(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Vertex, module, entry)
    }

    /// Fragment shader
    #[inline]
    pub fn fragment(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Fragment, module, entry)
    }

    /// Compute shader
    #[inline]
    pub fn compute(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Compute, module, entry)
    }

    /// Geometry shader
    #[inline]
    pub fn geometry(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Geometry, module, entry)
    }

    /// Tessellation control shader
    #[inline]
    pub fn tess_control(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::TessellationControl, module, entry)
    }

    /// Tessellation evaluation shader
    #[inline]
    pub fn tess_eval(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::TessellationEvaluation, module, entry)
    }

    /// Task shader
    #[inline]
    pub fn task(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Task, module, entry)
    }

    /// Mesh shader
    #[inline]
    pub fn mesh(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Mesh, module, entry)
    }

    /// Ray generation shader
    #[inline]
    pub fn raygen(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::RayGen, module, entry)
    }

    /// Miss shader
    #[inline]
    pub fn miss(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Miss, module, entry)
    }

    /// Closest hit shader
    #[inline]
    pub fn closest_hit(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::ClosestHit, module, entry)
    }

    /// Any hit shader
    #[inline]
    pub fn any_hit(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::AnyHit, module, entry)
    }

    /// Intersection shader
    #[inline]
    pub fn intersection(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Intersection, module, entry)
    }

    /// Callable shader
    #[inline]
    pub fn callable(module: u64, entry: &str) -> Self {
        Self::new(ShaderStage::Callable, module, entry)
    }

    /// Add specialization constant
    #[inline]
    pub fn specialize(mut self, constant: SpecializationConstant) -> Self {
        self.specialization.push(constant);
        self
    }
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex
    Vertex = 0x00000001,
    /// Tessellation control
    TessellationControl = 0x00000002,
    /// Tessellation evaluation
    TessellationEvaluation = 0x00000004,
    /// Geometry
    Geometry = 0x00000008,
    /// Fragment
    Fragment = 0x00000010,
    /// Compute
    Compute = 0x00000020,
    /// Task (mesh shading)
    Task = 0x00000040,
    /// Mesh
    Mesh = 0x00000080,
    /// Ray generation
    RayGen = 0x00000100,
    /// Any hit
    AnyHit = 0x00000200,
    /// Closest hit
    ClosestHit = 0x00000400,
    /// Miss
    Miss = 0x00000800,
    /// Intersection
    Intersection = 0x00001000,
    /// Callable
    Callable = 0x00002000,
}

/// Specialization constant
#[derive(Clone, Debug)]
pub struct SpecializationConstant {
    /// Constant ID
    pub constant_id: u32,
    /// Value
    pub value: SpecializationValue,
}

/// Specialization value
#[derive(Clone, Debug)]
pub enum SpecializationValue {
    /// Bool
    Bool(bool),
    /// I32
    I32(i32),
    /// U32
    U32(u32),
    /// F32
    F32(f32),
    /// I64
    I64(i64),
    /// U64
    U64(u64),
    /// F64
    F64(f64),
}

// ============================================================================
// Ray Tracing Shader Group
// ============================================================================

/// Ray tracing shader group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RayTracingShaderGroup {
    /// Type
    pub group_type: RayTracingShaderGroupType,
    /// General shader index
    pub general_shader: u32,
    /// Closest hit shader index
    pub closest_hit_shader: u32,
    /// Any hit shader index
    pub any_hit_shader: u32,
    /// Intersection shader index
    pub intersection_shader: u32,
}

impl RayTracingShaderGroup {
    /// Unused shader
    pub const UNUSED: u32 = !0;

    /// General group (raygen, miss, callable)
    #[inline]
    pub const fn general(shader_index: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::General,
            general_shader: shader_index,
            closest_hit_shader: Self::UNUSED,
            any_hit_shader: Self::UNUSED,
            intersection_shader: Self::UNUSED,
        }
    }

    /// Triangles hit group
    #[inline]
    pub const fn triangles_hit(closest_hit: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::TrianglesHitGroup,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: Self::UNUSED,
            intersection_shader: Self::UNUSED,
        }
    }

    /// Procedural hit group
    #[inline]
    pub const fn procedural_hit(intersection: u32, closest_hit: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::ProceduralHitGroup,
            general_shader: Self::UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: Self::UNUSED,
            intersection_shader: intersection,
        }
    }

    /// With any hit shader
    #[inline]
    pub const fn with_any_hit(mut self, any_hit: u32) -> Self {
        self.any_hit_shader = any_hit;
        self
    }
}

/// Ray tracing shader group type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RayTracingShaderGroupType {
    /// General (raygen, miss, callable)
    General = 0,
    /// Triangles hit group
    TrianglesHitGroup = 1,
    /// Procedural hit group
    ProceduralHitGroup = 2,
}

// ============================================================================
// Pipeline States
// ============================================================================

/// Vertex input state
#[derive(Clone, Debug, Default)]
pub struct VertexInputState {
    /// Bindings
    pub bindings: Vec<VertexBindingDescription>,
    /// Attributes
    pub attributes: Vec<VertexAttributeDescription>,
}

/// Vertex binding description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexBindingDescription {
    /// Binding
    pub binding: u32,
    /// Stride
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexInputRate {
    /// Per vertex
    #[default]
    Vertex = 0,
    /// Per instance
    Instance = 1,
}

/// Vertex attribute description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexAttributeDescription {
    /// Location
    pub location: u32,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset
    pub offset: u32,
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// R32 float
    #[default]
    R32Sfloat = 100,
    /// RG32 float
    R32G32Sfloat = 103,
    /// RGB32 float
    R32G32B32Sfloat = 106,
    /// RGBA32 float
    R32G32B32A32Sfloat = 109,
    /// R32 int
    R32Sint = 99,
    /// RG32 int
    R32G32Sint = 102,
    /// RGB32 int
    R32G32B32Sint = 105,
    /// RGBA32 int
    R32G32B32A32Sint = 108,
    /// R32 uint
    R32Uint = 98,
    /// RG32 uint
    R32G32Uint = 101,
    /// RGB32 uint
    R32G32B32Uint = 104,
    /// RGBA32 uint
    R32G32B32A32Uint = 107,
}

/// Input assembly state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InputAssemblyState {
    /// Topology
    pub topology: PrimitiveTopology,
    /// Primitive restart enable
    pub primitive_restart_enable: bool,
}

impl Default for InputAssemblyState {
    fn default() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        }
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrimitiveTopology {
    /// Point list
    PointList = 0,
    /// Line list
    LineList = 1,
    /// Line strip
    LineStrip = 2,
    /// Triangle list
    #[default]
    TriangleList = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan = 5,
    /// Line list with adjacency
    LineListWithAdjacency = 6,
    /// Line strip with adjacency
    LineStripWithAdjacency = 7,
    /// Triangle list with adjacency
    TriangleListWithAdjacency = 8,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency = 9,
    /// Patch list
    PatchList = 10,
}

/// Tessellation state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct TessellationState {
    /// Patch control points
    pub patch_control_points: u32,
}

/// Viewport state
#[derive(Clone, Debug, Default)]
pub struct ViewportState {
    /// Viewports
    pub viewports: Vec<Viewport>,
    /// Scissors
    pub scissors: Vec<Scissor>,
}

/// Viewport
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Viewport {
    /// X
    pub x: f32,
    /// Y
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Min depth
    pub min_depth: f32,
    /// Max depth
    pub max_depth: f32,
}

/// Scissor
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Scissor {
    /// X
    pub x: i32,
    /// Y
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// Rasterization state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationState {
    /// Depth clamp enable
    pub depth_clamp_enable: bool,
    /// Rasterizer discard enable
    pub rasterizer_discard_enable: bool,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullModeFlags,
    /// Front face
    pub front_face: FrontFace,
    /// Depth bias enable
    pub depth_bias_enable: bool,
    /// Depth bias constant factor
    pub depth_bias_constant_factor: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope factor
    pub depth_bias_slope_factor: f32,
    /// Line width
    pub line_width: f32,
}

impl Default for RasterizationState {
    fn default() -> Self {
        Self {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::BACK,
            front_face: FrontFace::CounterClockwise,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }
}

/// Polygon mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PolygonMode {
    /// Fill
    #[default]
    Fill = 0,
    /// Line
    Line = 1,
    /// Point
    Point = 2,
}

/// Cull mode flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CullModeFlags(pub u32);

impl CullModeFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Front
    pub const FRONT: Self = Self(1);
    /// Back
    pub const BACK: Self = Self(2);
    /// Front and back
    pub const FRONT_AND_BACK: Self = Self(3);
}

/// Front face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FrontFace {
    /// Counter clockwise
    #[default]
    CounterClockwise = 0,
    /// Clockwise
    Clockwise = 1,
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Rasterization samples
    pub rasterization_samples: SampleCount,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Min sample shading
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: u32,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            rasterization_samples: SampleCount::Count1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: !0,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    Count1 = 1,
    /// 2 samples
    Count2 = 2,
    /// 4 samples
    Count4 = 4,
    /// 8 samples
    Count8 = 8,
    /// 16 samples
    Count16 = 16,
    /// 32 samples
    Count32 = 32,
    /// 64 samples
    Count64 = 64,
}

/// Depth stencil state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilState {
    /// Depth test enable
    pub depth_test_enable: bool,
    /// Depth write enable
    pub depth_write_enable: bool,
    /// Depth compare op
    pub depth_compare_op: CompareOp,
    /// Depth bounds test enable
    pub depth_bounds_test_enable: bool,
    /// Stencil test enable
    pub stencil_test_enable: bool,
    /// Front stencil op state
    pub front: StencilOpState,
    /// Back stencil op state
    pub back: StencilOpState,
    /// Min depth bounds
    pub min_depth_bounds: f32,
    /// Max depth bounds
    pub max_depth_bounds: f32,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test_enable: false,
            stencil_test_enable: false,
            front: StencilOpState::default(),
            back: StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }
}

/// Compare op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompareOp {
    /// Never
    Never = 0,
    /// Less
    #[default]
    Less = 1,
    /// Equal
    Equal = 2,
    /// Less or equal
    LessOrEqual = 3,
    /// Greater
    Greater = 4,
    /// Not equal
    NotEqual = 5,
    /// Greater or equal
    GreaterOrEqual = 6,
    /// Always
    Always = 7,
}

/// Stencil op state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct StencilOpState {
    /// Fail op
    pub fail_op: StencilOp,
    /// Pass op
    pub pass_op: StencilOp,
    /// Depth fail op
    pub depth_fail_op: StencilOp,
    /// Compare op
    pub compare_op: CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference
    pub reference: u32,
}

/// Stencil op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StencilOp {
    /// Keep
    #[default]
    Keep = 0,
    /// Zero
    Zero = 1,
    /// Replace
    Replace = 2,
    /// Increment and clamp
    IncrementAndClamp = 3,
    /// Decrement and clamp
    DecrementAndClamp = 4,
    /// Invert
    Invert = 5,
    /// Increment and wrap
    IncrementAndWrap = 6,
    /// Decrement and wrap
    DecrementAndWrap = 7,
}

/// Color blend state
#[derive(Clone, Debug, Default)]
pub struct ColorBlendState {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Logic op
    pub logic_op: LogicOp,
    /// Attachments
    pub attachments: Vec<ColorBlendAttachment>,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

/// Logic op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LogicOp {
    /// Clear
    Clear = 0,
    /// And
    And = 1,
    /// And reverse
    AndReverse = 2,
    /// Copy
    #[default]
    Copy = 3,
    /// And inverted
    AndInverted = 4,
    /// No op
    NoOp = 5,
    /// Xor
    Xor = 6,
    /// Or
    Or = 7,
    /// Nor
    Nor = 8,
    /// Equivalent
    Equivalent = 9,
    /// Invert
    Invert = 10,
    /// Or reverse
    OrReverse = 11,
    /// Copy inverted
    CopyInverted = 12,
    /// Or inverted
    OrInverted = 13,
    /// Nand
    Nand = 14,
    /// Set
    Set = 15,
}

/// Color blend attachment
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorBlendAttachment {
    /// Blend enable
    pub blend_enable: bool,
    /// Src color blend factor
    pub src_color_blend_factor: BlendFactor,
    /// Dst color blend factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend op
    pub color_blend_op: BlendOp,
    /// Src alpha blend factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Dst alpha blend factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend op
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorComponentFlags,
}

impl Default for ColorBlendAttachment {
    fn default() -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendFactor {
    /// Zero
    Zero = 0,
    /// One
    #[default]
    One = 1,
    /// Src color
    SrcColor = 2,
    /// One minus src color
    OneMinusSrcColor = 3,
    /// Dst color
    DstColor = 4,
    /// One minus dst color
    OneMinusDstColor = 5,
    /// Src alpha
    SrcAlpha = 6,
    /// One minus src alpha
    OneMinusSrcAlpha = 7,
    /// Dst alpha
    DstAlpha = 8,
    /// One minus dst alpha
    OneMinusDstAlpha = 9,
    /// Constant color
    ConstantColor = 10,
    /// One minus constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha = 12,
    /// One minus constant alpha
    OneMinusConstantAlpha = 13,
    /// Src alpha saturate
    SrcAlphaSaturate = 14,
}

/// Blend op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendOp {
    /// Add
    #[default]
    Add = 0,
    /// Subtract
    Subtract = 1,
    /// Reverse subtract
    ReverseSubtract = 2,
    /// Min
    Min = 3,
    /// Max
    Max = 4,
}

/// Color component flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ColorComponentFlags(pub u32);

impl ColorComponentFlags {
    /// Red
    pub const R: Self = Self(1);
    /// Green
    pub const G: Self = Self(2);
    /// Blue
    pub const B: Self = Self(4);
    /// Alpha
    pub const A: Self = Self(8);
    /// All components
    pub const ALL: Self = Self(15);
}

/// Dynamic state type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DynamicStateType {
    /// Viewport
    Viewport = 0,
    /// Scissor
    Scissor = 1,
    /// Line width
    LineWidth = 2,
    /// Depth bias
    DepthBias = 3,
    /// Blend constants
    BlendConstants = 4,
    /// Depth bounds
    DepthBounds = 5,
    /// Stencil compare mask
    StencilCompareMask = 6,
    /// Stencil write mask
    StencilWriteMask = 7,
    /// Stencil reference
    StencilReference = 8,
    /// Cull mode
    CullMode = 1000267000,
    /// Front face
    FrontFace = 1000267001,
    /// Primitive topology
    PrimitiveTopology = 1000267002,
    /// Viewport with count
    ViewportWithCount = 1000267003,
    /// Scissor with count
    ScissorWithCount = 1000267004,
    /// Vertex input binding stride
    VertexInputBindingStride = 1000267005,
    /// Depth test enable
    DepthTestEnable = 1000267006,
    /// Depth write enable
    DepthWriteEnable = 1000267007,
    /// Depth compare op
    DepthCompareOp = 1000267008,
    /// Depth bounds test enable
    DepthBoundsTestEnable = 1000267009,
    /// Stencil test enable
    StencilTestEnable = 1000267010,
    /// Stencil op
    StencilOp = 1000267011,
    /// Rasterizer discard enable
    RasterizerDiscardEnable = 1000377001,
    /// Depth bias enable
    DepthBiasEnable = 1000377002,
    /// Primitive restart enable
    PrimitiveRestartEnable = 1000377004,
}

// ============================================================================
// Pipeline Build Error
// ============================================================================

/// Pipeline build error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineBuildError {
    /// No shader stages
    NoShaderStages,
    /// No layout
    NoLayout,
    /// No shader groups
    NoShaderGroups,
    /// Invalid stage combination
    InvalidStageCombination,
    /// Missing vertex shader
    MissingVertexShader,
    /// Missing fragment shader
    MissingFragmentShader,
}

impl core::fmt::Display for PipelineBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoShaderStages => write!(f, "No shader stages specified"),
            Self::NoLayout => write!(f, "No pipeline layout specified"),
            Self::NoShaderGroups => write!(f, "No shader groups specified"),
            Self::InvalidStageCombination => write!(f, "Invalid shader stage combination"),
            Self::MissingVertexShader => write!(f, "Missing vertex shader"),
            Self::MissingFragmentShader => write!(f, "Missing fragment shader"),
        }
    }
}

// ============================================================================
// Pipeline Batch Builder
// ============================================================================

/// Pipeline batch builder for creating multiple pipelines at once
#[derive(Clone, Debug, Default)]
pub struct PipelineBatchBuilder {
    /// Graphics pipelines
    pub graphics: Vec<GraphicsPipelineBuilder>,
    /// Compute pipelines
    pub compute: Vec<ComputePipelineBuilder>,
    /// Ray tracing pipelines
    pub ray_tracing: Vec<RayTracingPipelineBuilder>,
}

impl PipelineBatchBuilder {
    /// Creates new batch builder
    pub fn new() -> Self {
        Self {
            graphics: Vec::new(),
            compute: Vec::new(),
            ray_tracing: Vec::new(),
        }
    }

    /// Add graphics pipeline
    pub fn add_graphics(mut self, builder: GraphicsPipelineBuilder) -> Self {
        self.graphics.push(builder);
        self
    }

    /// Add compute pipeline
    pub fn add_compute(mut self, builder: ComputePipelineBuilder) -> Self {
        self.compute.push(builder);
        self
    }

    /// Add ray tracing pipeline
    pub fn add_ray_tracing(mut self, builder: RayTracingPipelineBuilder) -> Self {
        self.ray_tracing.push(builder);
        self
    }

    /// Total pipeline count
    pub fn count(&self) -> usize {
        self.graphics.len() + self.compute.len() + self.ray_tracing.len()
    }
}
