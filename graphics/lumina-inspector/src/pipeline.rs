//! # Pipeline Inspector
//!
//! Deep inspection of GPU pipelines with state visualization.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

use crate::resource::{ShaderStage, CompareOp};

/// Pipeline inspector
pub struct PipelineInspector {
    pipelines: BTreeMap<u64, PipelineState>,
}

impl PipelineInspector {
    pub fn new() -> Self {
        Self {
            pipelines: BTreeMap::new(),
        }
    }
    
    /// Track a pipeline
    pub fn track(&mut self, id: u64, state: PipelineState) {
        self.pipelines.insert(id, state);
    }
    
    /// Get pipeline info
    pub fn get(&self, id: u64) -> Option<PipelineInfo> {
        self.pipelines.get(&id).map(|s| PipelineInfo {
            id,
            state: s.clone(),
        })
    }
    
    /// Analyze pipeline for issues
    pub fn analyze(&self, id: u64) -> Option<PipelineAnalysis> {
        let state = self.pipelines.get(&id)?;
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        
        // Check for common issues
        if let PipelineState::Graphics(ref gfx) = state {
            // Check for expensive blend modes
            for (i, blend) in gfx.blend_state.attachments.iter().enumerate() {
                if blend.blend_enable {
                    if matches!(blend.src_color_blend_factor, BlendFactor::SrcAlpha)
                        && matches!(blend.dst_color_blend_factor, BlendFactor::OneMinusSrcAlpha)
                    {
                        suggestions.push(alloc::format!(
                            "Attachment {}: Consider pre-multiplied alpha for better performance",
                            i
                        ));
                    }
                }
            }
            
            // Check depth testing
            if !gfx.depth_stencil_state.depth_test_enable && gfx.depth_stencil_state.depth_write_enable {
                issues.push(String::from("Depth write enabled without depth test - writes will be ignored"));
            }
            
            // Check for multisampling without resolve
            if gfx.multisample_state.rasterization_samples > 1 {
                suggestions.push(String::from("Consider using sample shading for better quality"));
            }
            
            // Check for expensive rasterizer modes
            if gfx.rasterization_state.polygon_mode != PolygonMode::Fill {
                suggestions.push(String::from("Non-fill polygon mode detected - may impact performance"));
            }
        }
        
        Some(PipelineAnalysis {
            pipeline_id: id,
            issues,
            suggestions,
            estimated_cost: estimate_pipeline_cost(state),
        })
    }
    
    /// Compare two pipelines
    pub fn diff(&self, id_a: u64, id_b: u64) -> Option<PipelineDiff> {
        let a = self.pipelines.get(&id_a)?;
        let b = self.pipelines.get(&id_b)?;
        
        Some(diff_pipelines(a, b))
    }
}

impl Default for PipelineInspector {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline information
#[derive(Debug, Clone)]
pub struct PipelineInfo {
    pub id: u64,
    pub state: PipelineState,
}

/// Pipeline state
#[derive(Debug, Clone)]
pub enum PipelineState {
    Graphics(GraphicsPipelineState),
    Compute(ComputePipelineState),
    RayTracing(RayTracingPipelineState),
}

/// Graphics pipeline state
#[derive(Debug, Clone)]
pub struct GraphicsPipelineState {
    pub name: Option<String>,
    pub shader_stages: Vec<ShaderStageState>,
    pub vertex_input_state: VertexInputState,
    pub input_assembly_state: InputAssemblyState,
    pub tessellation_state: Option<TessellationState>,
    pub viewport_state: ViewportState,
    pub rasterization_state: RasterizationState,
    pub multisample_state: MultisampleState,
    pub depth_stencil_state: DepthStencilState,
    pub blend_state: ColorBlendState,
    pub dynamic_state: Vec<DynamicState>,
    pub layout_id: u64,
    pub render_pass_id: Option<u64>,
    pub subpass: u32,
}

/// Shader stage state
#[derive(Debug, Clone)]
pub struct ShaderStageState {
    pub stage: ShaderStage,
    pub module_id: u64,
    pub entry_point: String,
    pub specialization: Vec<SpecializationConstant>,
}

/// Specialization constant
#[derive(Debug, Clone)]
pub struct SpecializationConstant {
    pub id: u32,
    pub data: Vec<u8>,
}

/// Vertex input state
#[derive(Debug, Clone)]
pub struct VertexInputState {
    pub bindings: Vec<VertexBinding>,
    pub attributes: Vec<VertexAttribute>,
}

/// Vertex binding
#[derive(Debug, Clone)]
pub struct VertexBinding {
    pub binding: u32,
    pub stride: u32,
    pub input_rate: VertexInputRate,
}

/// Vertex input rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

/// Vertex attribute
#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub location: u32,
    pub binding: u32,
    pub format: u32,
    pub offset: u32,
}

/// Input assembly state
#[derive(Debug, Clone)]
pub struct InputAssemblyState {
    pub topology: PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

/// Primitive topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
    LineListWithAdjacency,
    LineStripWithAdjacency,
    TriangleListWithAdjacency,
    TriangleStripWithAdjacency,
    PatchList,
}

/// Tessellation state
#[derive(Debug, Clone)]
pub struct TessellationState {
    pub patch_control_points: u32,
}

/// Viewport state
#[derive(Debug, Clone)]
pub struct ViewportState {
    pub viewports: Vec<Viewport>,
    pub scissors: Vec<Scissor>,
}

/// Viewport
#[derive(Debug, Clone)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// Scissor
#[derive(Debug, Clone)]
pub struct Scissor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Rasterization state
#[derive(Debug, Clone)]
pub struct RasterizationState {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32,
}

/// Polygon mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}

/// Cull mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
    FrontAndBack,
}

/// Front face
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

/// Multisample state
#[derive(Debug, Clone)]
pub struct MultisampleState {
    pub rasterization_samples: u32,
    pub sample_shading_enable: bool,
    pub min_sample_shading: f32,
    pub sample_mask: Option<Vec<u32>>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

/// Depth stencil state
#[derive(Debug, Clone)]
pub struct DepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub depth_bounds_test_enable: bool,
    pub stencil_test_enable: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
    pub min_depth_bounds: f32,
    pub max_depth_bounds: f32,
}

/// Stencil operation state
#[derive(Debug, Clone)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

/// Stencil operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StencilOp {
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

/// Color blend state
#[derive(Debug, Clone)]
pub struct ColorBlendState {
    pub logic_op_enable: bool,
    pub logic_op: LogicOp,
    pub attachments: Vec<ColorBlendAttachment>,
    pub blend_constants: [f32; 4],
}

/// Logic operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOp {
    Clear,
    And,
    AndReverse,
    Copy,
    AndInverted,
    NoOp,
    Xor,
    Or,
    Nor,
    Equivalent,
    Invert,
    OrReverse,
    CopyInverted,
    OrInverted,
    Nand,
    Set,
}

/// Color blend attachment
#[derive(Debug, Clone)]
pub struct ColorBlendAttachment {
    pub blend_enable: bool,
    pub src_color_blend_factor: BlendFactor,
    pub dst_color_blend_factor: BlendFactor,
    pub color_blend_op: BlendOp,
    pub src_alpha_blend_factor: BlendFactor,
    pub dst_alpha_blend_factor: BlendFactor,
    pub alpha_blend_op: BlendOp,
    pub color_write_mask: ColorComponentFlags,
}

/// Blend factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

/// Blend operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// Color component flags
#[derive(Debug, Clone, Copy)]
pub struct ColorComponentFlags {
    pub r: bool,
    pub g: bool,
    pub b: bool,
    pub a: bool,
}

/// Dynamic state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicState {
    Viewport,
    Scissor,
    LineWidth,
    DepthBias,
    BlendConstants,
    DepthBounds,
    StencilCompareMask,
    StencilWriteMask,
    StencilReference,
    CullMode,
    FrontFace,
    PrimitiveTopology,
    ViewportWithCount,
    ScissorWithCount,
    VertexInputBindingStride,
    DepthTestEnable,
    DepthWriteEnable,
    DepthCompareOp,
    DepthBoundsTestEnable,
    StencilTestEnable,
    StencilOp,
    RasterizerDiscardEnable,
    DepthBiasEnable,
    PrimitiveRestartEnable,
}

/// Compute pipeline state
#[derive(Debug, Clone)]
pub struct ComputePipelineState {
    pub name: Option<String>,
    pub shader: ShaderStageState,
    pub layout_id: u64,
    pub local_size: [u32; 3],
}

/// Ray tracing pipeline state
#[derive(Debug, Clone)]
pub struct RayTracingPipelineState {
    pub name: Option<String>,
    pub stages: Vec<ShaderStageState>,
    pub groups: Vec<RayTracingShaderGroup>,
    pub max_ray_recursion_depth: u32,
    pub layout_id: u64,
}

/// Ray tracing shader group
#[derive(Debug, Clone)]
pub struct RayTracingShaderGroup {
    pub group_type: RayTracingGroupType,
    pub general_shader: Option<u32>,
    pub closest_hit_shader: Option<u32>,
    pub any_hit_shader: Option<u32>,
    pub intersection_shader: Option<u32>,
}

/// Ray tracing group type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RayTracingGroupType {
    General,
    TrianglesHitGroup,
    ProceduralHitGroup,
}

/// Pipeline analysis result
#[derive(Debug, Clone)]
pub struct PipelineAnalysis {
    pub pipeline_id: u64,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    pub estimated_cost: PipelineCost,
}

/// Estimated pipeline cost
#[derive(Debug, Clone)]
pub struct PipelineCost {
    pub vertex_cost: f32,
    pub fragment_cost: f32,
    pub state_switch_cost: f32,
    pub overall: f32,
}

fn estimate_pipeline_cost(state: &PipelineState) -> PipelineCost {
    match state {
        PipelineState::Graphics(gfx) => {
            let mut vertex_cost = 1.0;
            let mut fragment_cost = 1.0;
            let mut state_cost = 0.0;
            
            // Tessellation adds cost
            if gfx.tessellation_state.is_some() {
                vertex_cost += 0.5;
            }
            
            // Multisampling adds cost
            fragment_cost *= gfx.multisample_state.rasterization_samples as f32;
            
            // Blending adds cost
            for blend in &gfx.blend_state.attachments {
                if blend.blend_enable {
                    fragment_cost += 0.1;
                }
            }
            
            // Dynamic state reduces switch cost
            state_cost = 1.0 - (gfx.dynamic_state.len() as f32 * 0.05);
            
            PipelineCost {
                vertex_cost,
                fragment_cost,
                state_switch_cost: state_cost.max(0.1),
                overall: vertex_cost + fragment_cost + state_cost,
            }
        }
        PipelineState::Compute(_) => PipelineCost {
            vertex_cost: 0.0,
            fragment_cost: 0.0,
            state_switch_cost: 0.2,
            overall: 1.0,
        },
        PipelineState::RayTracing(rt) => PipelineCost {
            vertex_cost: 0.0,
            fragment_cost: 0.0,
            state_switch_cost: 0.5,
            overall: rt.max_ray_recursion_depth as f32 * 2.0,
        },
    }
}

/// Pipeline difference
#[derive(Debug, Clone)]
pub struct PipelineDiff {
    pub changed_stages: Vec<ShaderStage>,
    pub changed_state: Vec<String>,
    pub compatible: bool,
}

fn diff_pipelines(a: &PipelineState, b: &PipelineState) -> PipelineDiff {
    match (a, b) {
        (PipelineState::Graphics(ga), PipelineState::Graphics(gb)) => {
            let mut changed_stages = Vec::new();
            let mut changed_state = Vec::new();
            
            // Compare shader stages
            for stage_a in &ga.shader_stages {
                let stage_b = gb.shader_stages.iter()
                    .find(|s| s.stage == stage_a.stage);
                
                if let Some(sb) = stage_b {
                    if stage_a.module_id != sb.module_id {
                        changed_stages.push(stage_a.stage);
                    }
                } else {
                    changed_stages.push(stage_a.stage);
                }
            }
            
            // Compare states
            if ga.rasterization_state.cull_mode != gb.rasterization_state.cull_mode {
                changed_state.push(String::from("cull_mode"));
            }
            if ga.depth_stencil_state.depth_test_enable != gb.depth_stencil_state.depth_test_enable {
                changed_state.push(String::from("depth_test"));
            }
            
            PipelineDiff {
                changed_stages,
                changed_state,
                compatible: ga.layout_id == gb.layout_id,
            }
        }
        _ => PipelineDiff {
            changed_stages: Vec::new(),
            changed_state: vec![String::from("pipeline_type")],
            compatible: false,
        },
    }
}
