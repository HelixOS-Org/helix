//! Shader Graph System
//!
//! This module provides a node-based shader authoring system for
//! procedural materials and complex shader effects.

use alloc::{string::String, vec::Vec, collections::BTreeMap, boxed::Box, format};
use core::hash::{Hash, Hasher};

// ============================================================================
// Node ID
// ============================================================================

/// Unique node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Invalid node ID.
    pub const INVALID: Self = Self(u32::MAX);

    /// Create a new node ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

// ============================================================================
// Data Types
// ============================================================================

/// Shader data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Boolean.
    Bool,
    /// 32-bit integer.
    Int,
    /// 32-bit unsigned integer.
    Uint,
    /// 32-bit float.
    Float,
    /// 2D vector.
    Vec2,
    /// 3D vector.
    Vec3,
    /// 4D vector.
    Vec4,
    /// 2x2 matrix.
    Mat2,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
    /// 2D sampler.
    Sampler2D,
    /// 3D sampler.
    Sampler3D,
    /// Cube sampler.
    SamplerCube,
}

impl DataType {
    /// Check if compatible for connection.
    pub fn is_compatible(&self, other: &DataType) -> bool {
        if self == other {
            return true;
        }

        // Allow implicit conversions
        match (self, other) {
            (DataType::Float, DataType::Vec2 | DataType::Vec3 | DataType::Vec4) => true,
            (DataType::Vec2 | DataType::Vec3 | DataType::Vec4, DataType::Float) => true,
            (DataType::Int, DataType::Float) => true,
            (DataType::Uint, DataType::Float | DataType::Int) => true,
            _ => false,
        }
    }

    /// Get component count.
    pub fn components(&self) -> u32 {
        match self {
            DataType::Bool | DataType::Int | DataType::Uint | DataType::Float => 1,
            DataType::Vec2 => 2,
            DataType::Vec3 => 3,
            DataType::Vec4 | DataType::Mat2 => 4,
            DataType::Mat3 => 9,
            DataType::Mat4 => 16,
            DataType::Sampler2D | DataType::Sampler3D | DataType::SamplerCube => 0,
        }
    }

    /// Get GLSL type name.
    pub fn glsl_name(&self) -> &'static str {
        match self {
            DataType::Bool => "bool",
            DataType::Int => "int",
            DataType::Uint => "uint",
            DataType::Float => "float",
            DataType::Vec2 => "vec2",
            DataType::Vec3 => "vec3",
            DataType::Vec4 => "vec4",
            DataType::Mat2 => "mat2",
            DataType::Mat3 => "mat3",
            DataType::Mat4 => "mat4",
            DataType::Sampler2D => "sampler2D",
            DataType::Sampler3D => "sampler3D",
            DataType::SamplerCube => "samplerCube",
        }
    }
}

// ============================================================================
// Node Input/Output
// ============================================================================

/// Node input slot.
#[derive(Debug, Clone)]
pub struct NodeInput {
    /// Input name.
    pub name: String,
    /// Data type.
    pub data_type: DataType,
    /// Default value.
    pub default_value: NodeValue,
    /// Connected output.
    pub connection: Option<Connection>,
}

impl NodeInput {
    /// Create a new input.
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            default_value: NodeValue::default_for_type(data_type),
            connection: None,
        }
    }

    /// Set default value.
    pub fn default(mut self, value: NodeValue) -> Self {
        self.default_value = value;
        self
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

/// Node output slot.
#[derive(Debug, Clone)]
pub struct NodeOutput {
    /// Output name.
    pub name: String,
    /// Data type.
    pub data_type: DataType,
}

impl NodeOutput {
    /// Create a new output.
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
        }
    }
}

/// Connection between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Connection {
    /// Source node.
    pub from_node: NodeId,
    /// Source output index.
    pub from_output: u32,
}

impl Connection {
    /// Create a new connection.
    pub fn new(from_node: NodeId, from_output: u32) -> Self {
        Self {
            from_node,
            from_output,
        }
    }
}

/// Node value.
#[derive(Debug, Clone)]
pub enum NodeValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i32),
    /// Unsigned integer value.
    Uint(u32),
    /// Float value.
    Float(f32),
    /// Vec2 value.
    Vec2([f32; 2]),
    /// Vec3 value.
    Vec3([f32; 3]),
    /// Vec4 value.
    Vec4([f32; 4]),
    /// Mat4 value.
    Mat4([[f32; 4]; 4]),
    /// Texture reference.
    Texture(u32),
}

impl NodeValue {
    /// Get default value for a type.
    pub fn default_for_type(data_type: DataType) -> Self {
        match data_type {
            DataType::Bool => Self::Bool(false),
            DataType::Int => Self::Int(0),
            DataType::Uint => Self::Uint(0),
            DataType::Float => Self::Float(0.0),
            DataType::Vec2 => Self::Vec2([0.0, 0.0]),
            DataType::Vec3 => Self::Vec3([0.0, 0.0, 0.0]),
            DataType::Vec4 => Self::Vec4([0.0, 0.0, 0.0, 0.0]),
            DataType::Mat2 | DataType::Mat3 | DataType::Mat4 => Self::Mat4([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]),
            DataType::Sampler2D | DataType::Sampler3D | DataType::SamplerCube => Self::Texture(0),
        }
    }

    /// Convert to GLSL literal.
    pub fn to_glsl(&self) -> String {
        match self {
            Self::Bool(v) => if *v { "true".into() } else { "false".into() },
            Self::Int(v) => format!("{}", v),
            Self::Uint(v) => format!("{}u", v),
            Self::Float(v) => format!("{:.6}", v),
            Self::Vec2(v) => format!("vec2({:.6}, {:.6})", v[0], v[1]),
            Self::Vec3(v) => format!("vec3({:.6}, {:.6}, {:.6})", v[0], v[1], v[2]),
            Self::Vec4(v) => format!("vec4({:.6}, {:.6}, {:.6}, {:.6})", v[0], v[1], v[2], v[3]),
            Self::Mat4(v) => format!(
                "mat4({:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6})",
                v[0][0], v[0][1], v[0][2], v[0][3],
                v[1][0], v[1][1], v[1][2], v[1][3],
                v[2][0], v[2][1], v[2][2], v[2][3],
                v[3][0], v[3][1], v[3][2], v[3][3]
            ),
            Self::Texture(v) => format!("texture_{}", v),
        }
    }
}

// ============================================================================
// Node Types
// ============================================================================

/// Node type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    /// Input nodes.
    Input,
    /// Output nodes.
    Output,
    /// Math operations.
    Math,
    /// Vector operations.
    Vector,
    /// Texture operations.
    Texture,
    /// Utility nodes.
    Utility,
    /// Procedural generators.
    Procedural,
    /// PBR nodes.
    Pbr,
}

/// Node type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeType {
    // Input nodes
    VertexPosition,
    VertexNormal,
    VertexTangent,
    VertexUV(u32),
    VertexColor,
    Time,
    CameraPosition,
    ViewDirection,
    ScreenPosition,

    // Constant nodes
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,

    // Math nodes
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    SquareRoot,
    Abs,
    Negate,
    Floor,
    Ceil,
    Fract,
    Mod,
    Min,
    Max,
    Clamp,
    Saturate,
    Lerp,
    SmoothStep,
    Step,
    Sin,
    Cos,
    Tan,
    ASin,
    ACos,
    ATan,
    ATan2,
    Exp,
    Log,
    Log2,

    // Vector nodes
    Normalize,
    Length,
    Distance,
    Dot,
    Cross,
    Reflect,
    Refract,
    Split,
    Combine,
    Swizzle(String),

    // Transform nodes
    TransformPoint,
    TransformVector,
    TransformNormal,
    WorldToTangent,
    TangentToWorld,

    // Texture nodes
    SampleTexture2D,
    SampleTexture3D,
    SampleTextureCube,
    TextureSize,
    Texel,
    Triplanar,

    // Procedural nodes
    Noise,
    Voronoi,
    Gradient,
    Checker,
    Brick,
    Wave,

    // PBR nodes
    Fresnel,
    FresnelSchlick,
    GGX,
    SmithG,
    Lambert,
    
    // Output nodes
    Output,
    PbrOutput,

    // Custom node
    Custom(String),
}

impl NodeType {
    /// Get the category.
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::VertexPosition
            | Self::VertexNormal
            | Self::VertexTangent
            | Self::VertexUV(_)
            | Self::VertexColor
            | Self::Time
            | Self::CameraPosition
            | Self::ViewDirection
            | Self::ScreenPosition
            | Self::Float
            | Self::Vec2
            | Self::Vec3
            | Self::Vec4
            | Self::Color => NodeCategory::Input,

            Self::Output | Self::PbrOutput => NodeCategory::Output,

            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Power
            | Self::SquareRoot
            | Self::Abs
            | Self::Negate
            | Self::Floor
            | Self::Ceil
            | Self::Fract
            | Self::Mod
            | Self::Min
            | Self::Max
            | Self::Clamp
            | Self::Saturate
            | Self::Lerp
            | Self::SmoothStep
            | Self::Step
            | Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::ASin
            | Self::ACos
            | Self::ATan
            | Self::ATan2
            | Self::Exp
            | Self::Log
            | Self::Log2 => NodeCategory::Math,

            Self::Normalize
            | Self::Length
            | Self::Distance
            | Self::Dot
            | Self::Cross
            | Self::Reflect
            | Self::Refract
            | Self::Split
            | Self::Combine
            | Self::Swizzle(_)
            | Self::TransformPoint
            | Self::TransformVector
            | Self::TransformNormal
            | Self::WorldToTangent
            | Self::TangentToWorld => NodeCategory::Vector,

            Self::SampleTexture2D
            | Self::SampleTexture3D
            | Self::SampleTextureCube
            | Self::TextureSize
            | Self::Texel
            | Self::Triplanar => NodeCategory::Texture,

            Self::Noise
            | Self::Voronoi
            | Self::Gradient
            | Self::Checker
            | Self::Brick
            | Self::Wave => NodeCategory::Procedural,

            Self::Fresnel
            | Self::FresnelSchlick
            | Self::GGX
            | Self::SmithG
            | Self::Lambert => NodeCategory::Pbr,

            Self::Custom(_) => NodeCategory::Utility,
        }
    }

    /// Get default inputs.
    pub fn default_inputs(&self) -> Vec<NodeInput> {
        match self {
            Self::Float => vec![NodeInput::new("value", DataType::Float)],
            Self::Vec2 => vec![
                NodeInput::new("x", DataType::Float),
                NodeInput::new("y", DataType::Float),
            ],
            Self::Vec3 => vec![
                NodeInput::new("x", DataType::Float),
                NodeInput::new("y", DataType::Float),
                NodeInput::new("z", DataType::Float),
            ],
            Self::Vec4 | Self::Color => vec![
                NodeInput::new("x", DataType::Float),
                NodeInput::new("y", DataType::Float),
                NodeInput::new("z", DataType::Float),
                NodeInput::new("w", DataType::Float).default(NodeValue::Float(1.0)),
            ],
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => vec![
                NodeInput::new("a", DataType::Float),
                NodeInput::new("b", DataType::Float),
            ],
            Self::Power => vec![
                NodeInput::new("base", DataType::Float),
                NodeInput::new("exp", DataType::Float),
            ],
            Self::Lerp => vec![
                NodeInput::new("a", DataType::Float),
                NodeInput::new("b", DataType::Float),
                NodeInput::new("t", DataType::Float),
            ],
            Self::Clamp => vec![
                NodeInput::new("value", DataType::Float),
                NodeInput::new("min", DataType::Float),
                NodeInput::new("max", DataType::Float),
            ],
            Self::SampleTexture2D => vec![
                NodeInput::new("texture", DataType::Sampler2D),
                NodeInput::new("uv", DataType::Vec2),
            ],
            Self::Noise => vec![
                NodeInput::new("position", DataType::Vec3),
                NodeInput::new("scale", DataType::Float).default(NodeValue::Float(1.0)),
                NodeInput::new("octaves", DataType::Int).default(NodeValue::Int(4)),
            ],
            Self::Fresnel => vec![
                NodeInput::new("normal", DataType::Vec3),
                NodeInput::new("view", DataType::Vec3),
                NodeInput::new("power", DataType::Float).default(NodeValue::Float(5.0)),
            ],
            Self::PbrOutput => vec![
                NodeInput::new("albedo", DataType::Vec3),
                NodeInput::new("normal", DataType::Vec3),
                NodeInput::new("metallic", DataType::Float),
                NodeInput::new("roughness", DataType::Float),
                NodeInput::new("ao", DataType::Float).default(NodeValue::Float(1.0)),
                NodeInput::new("emissive", DataType::Vec3),
            ],
            _ => Vec::new(),
        }
    }

    /// Get default outputs.
    pub fn default_outputs(&self) -> Vec<NodeOutput> {
        match self {
            Self::VertexPosition | Self::VertexNormal | Self::VertexTangent | Self::CameraPosition | Self::ViewDirection => {
                vec![NodeOutput::new("xyz", DataType::Vec3)]
            }
            Self::VertexUV(_) | Self::ScreenPosition => {
                vec![NodeOutput::new("uv", DataType::Vec2)]
            }
            Self::VertexColor | Self::Color => {
                vec![NodeOutput::new("rgba", DataType::Vec4)]
            }
            Self::Time => vec![NodeOutput::new("time", DataType::Float)],
            Self::Float | Self::Sin | Self::Cos | Self::Length | Self::Dot => {
                vec![NodeOutput::new("value", DataType::Float)]
            }
            Self::Vec2 => vec![NodeOutput::new("xy", DataType::Vec2)],
            Self::Vec3 | Self::Cross | Self::Normalize | Self::Reflect => {
                vec![NodeOutput::new("xyz", DataType::Vec3)]
            }
            Self::Vec4 | Self::SampleTexture2D => vec![NodeOutput::new("rgba", DataType::Vec4)],
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Lerp | Self::Clamp => {
                vec![NodeOutput::new("result", DataType::Float)]
            }
            Self::Split => vec![
                NodeOutput::new("x", DataType::Float),
                NodeOutput::new("y", DataType::Float),
                NodeOutput::new("z", DataType::Float),
                NodeOutput::new("w", DataType::Float),
            ],
            Self::Noise | Self::Fresnel => vec![NodeOutput::new("value", DataType::Float)],
            _ => Vec::new(),
        }
    }
}

// ============================================================================
// Shader Node
// ============================================================================

/// A node in the shader graph.
pub struct ShaderNode {
    /// Node ID.
    id: NodeId,
    /// Node type.
    node_type: NodeType,
    /// Node name.
    name: String,
    /// Inputs.
    inputs: Vec<NodeInput>,
    /// Outputs.
    outputs: Vec<NodeOutput>,
    /// Position in editor.
    position: [f32; 2],
    /// Custom properties.
    properties: BTreeMap<String, NodeValue>,
}

impl ShaderNode {
    /// Create a new node.
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        let inputs = node_type.default_inputs();
        let outputs = node_type.default_outputs();

        Self {
            id,
            node_type: node_type.clone(),
            name: format!("{:?}", node_type),
            inputs,
            outputs,
            position: [0.0, 0.0],
            properties: BTreeMap::new(),
        }
    }

    /// Get node ID.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get node type.
    pub fn node_type(&self) -> &NodeType {
        &self.node_type
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Get inputs.
    pub fn inputs(&self) -> &[NodeInput] {
        &self.inputs
    }

    /// Get mutable inputs.
    pub fn inputs_mut(&mut self) -> &mut Vec<NodeInput> {
        &mut self.inputs
    }

    /// Get outputs.
    pub fn outputs(&self) -> &[NodeOutput] {
        &self.outputs
    }

    /// Get position.
    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    /// Set position.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = [x, y];
    }

    /// Set property.
    pub fn set_property(&mut self, name: impl Into<String>, value: NodeValue) {
        self.properties.insert(name.into(), value);
    }

    /// Get property.
    pub fn property(&self, name: &str) -> Option<&NodeValue> {
        self.properties.get(name)
    }

    /// Connect input to output.
    pub fn connect_input(&mut self, input_index: u32, connection: Connection) -> bool {
        if let Some(input) = self.inputs.get_mut(input_index as usize) {
            input.connection = Some(connection);
            true
        } else {
            false
        }
    }

    /// Disconnect input.
    pub fn disconnect_input(&mut self, input_index: u32) -> bool {
        if let Some(input) = self.inputs.get_mut(input_index as usize) {
            input.connection = None;
            true
        } else {
            false
        }
    }

    /// Generate GLSL code for this node.
    pub fn generate_glsl(&self, output_var: &str) -> String {
        match &self.node_type {
            NodeType::Add => format!("{} = _i0 + _i1;", output_var),
            NodeType::Subtract => format!("{} = _i0 - _i1;", output_var),
            NodeType::Multiply => format!("{} = _i0 * _i1;", output_var),
            NodeType::Divide => format!("{} = _i0 / _i1;", output_var),
            NodeType::Lerp => format!("{} = mix(_i0, _i1, _i2);", output_var),
            NodeType::Clamp => format!("{} = clamp(_i0, _i1, _i2);", output_var),
            NodeType::Sin => format!("{} = sin(_i0);", output_var),
            NodeType::Cos => format!("{} = cos(_i0);", output_var),
            NodeType::Normalize => format!("{} = normalize(_i0);", output_var),
            NodeType::Dot => format!("{} = dot(_i0, _i1);", output_var),
            NodeType::Cross => format!("{} = cross(_i0, _i1);", output_var),
            NodeType::Length => format!("{} = length(_i0);", output_var),
            NodeType::SampleTexture2D => format!("{} = texture(_i0, _i1);", output_var),
            _ => format!("// Unsupported node type: {:?}", self.node_type),
        }
    }
}

// ============================================================================
// Shader Graph
// ============================================================================

/// Shader graph.
pub struct ShaderGraph {
    /// Graph name.
    name: String,
    /// Nodes.
    nodes: BTreeMap<u32, ShaderNode>,
    /// Next node ID.
    next_id: u32,
    /// Output node ID.
    output_node: Option<NodeId>,
}

impl ShaderGraph {
    /// Create a new graph.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: BTreeMap::new(),
            next_id: 0,
            output_node: None,
        }
    }

    /// Get graph name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add a node.
    pub fn add_node(&mut self, node_type: NodeType) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let node = ShaderNode::new(id, node_type.clone());
        self.nodes.insert(id.0, node);

        // Track output node
        if matches!(node_type, NodeType::Output | NodeType::PbrOutput) {
            self.output_node = Some(id);
        }

        id
    }

    /// Remove a node.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id.0).is_some() {
            // Remove connections to this node
            for node in self.nodes.values_mut() {
                for input in &mut node.inputs {
                    if let Some(conn) = &input.connection {
                        if conn.from_node == id {
                            input.connection = None;
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Get node.
    pub fn node(&self, id: NodeId) -> Option<&ShaderNode> {
        self.nodes.get(&id.0)
    }

    /// Get mutable node.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut ShaderNode> {
        self.nodes.get_mut(&id.0)
    }

    /// Connect nodes.
    pub fn connect(
        &mut self,
        from_node: NodeId,
        from_output: u32,
        to_node: NodeId,
        to_input: u32,
    ) -> Result<(), GraphError> {
        // Validate connection
        let from = self.nodes.get(&from_node.0).ok_or(GraphError::NodeNotFound)?;
        let from_type = from
            .outputs
            .get(from_output as usize)
            .ok_or(GraphError::InvalidOutput)?
            .data_type;

        let to = self.nodes.get(&to_node.0).ok_or(GraphError::NodeNotFound)?;
        let to_type = to
            .inputs
            .get(to_input as usize)
            .ok_or(GraphError::InvalidInput)?
            .data_type;

        if !from_type.is_compatible(&to_type) {
            return Err(GraphError::TypeMismatch);
        }

        // Check for cycles
        if self.would_create_cycle(from_node, to_node) {
            return Err(GraphError::CycleDetected);
        }

        // Make connection
        let to = self.nodes.get_mut(&to_node.0).unwrap();
        to.connect_input(to_input, Connection::new(from_node, from_output));

        Ok(())
    }

    /// Disconnect.
    pub fn disconnect(&mut self, node: NodeId, input: u32) -> bool {
        if let Some(n) = self.nodes.get_mut(&node.0) {
            n.disconnect_input(input)
        } else {
            false
        }
    }

    /// Check if connection would create a cycle.
    fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        let mut visited = Vec::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            if let Some(node) = self.nodes.get(&current.0) {
                for input in &node.inputs {
                    if let Some(conn) = &input.connection {
                        stack.push(conn.from_node);
                    }
                }
            }
        }

        false
    }

    /// Get output node.
    pub fn output_node(&self) -> Option<NodeId> {
        self.output_node
    }

    /// Iterate over nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &ShaderNode> {
        self.nodes.values()
    }

    /// Get topologically sorted nodes.
    pub fn topological_sort(&self) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut visited = Vec::new();

        fn visit(
            graph: &ShaderGraph,
            id: NodeId,
            visited: &mut Vec<NodeId>,
            result: &mut Vec<NodeId>,
        ) {
            if visited.contains(&id) {
                return;
            }
            visited.push(id);

            if let Some(node) = graph.nodes.get(&id.0) {
                for input in &node.inputs {
                    if let Some(conn) = &input.connection {
                        visit(graph, conn.from_node, visited, result);
                    }
                }
            }

            result.push(id);
        }

        if let Some(output) = self.output_node {
            visit(self, output, &mut visited, &mut result);
        }

        result
    }

    /// Validate the graph.
    pub fn validate(&self) -> Vec<GraphError> {
        let mut errors = Vec::new();

        if self.output_node.is_none() {
            errors.push(GraphError::NoOutput);
        }

        // Check for disconnected required inputs
        for node in self.nodes.values() {
            for (i, input) in node.inputs.iter().enumerate() {
                if input.connection.is_none() {
                    // Check if default is acceptable
                }
            }
        }

        errors
    }
}

/// Graph error.
#[derive(Debug, Clone)]
pub enum GraphError {
    /// Node not found.
    NodeNotFound,
    /// Invalid input index.
    InvalidInput,
    /// Invalid output index.
    InvalidOutput,
    /// Type mismatch.
    TypeMismatch,
    /// Cycle detected.
    CycleDetected,
    /// No output node.
    NoOutput,
    /// Disconnected input.
    DisconnectedInput { node: NodeId, input: u32 },
}

// ============================================================================
// Graph Compiler
// ============================================================================

/// Graph compiler.
pub struct GraphCompiler {
    /// Generated code.
    code: String,
    /// Variable counter.
    var_counter: u32,
    /// Variable map.
    var_map: BTreeMap<(u32, u32), String>,
}

impl GraphCompiler {
    /// Create a new compiler.
    pub fn new() -> Self {
        Self {
            code: String::new(),
            var_counter: 0,
            var_map: BTreeMap::new(),
        }
    }

    /// Compile a graph.
    pub fn compile(&mut self, graph: &ShaderGraph) -> Result<CompiledGraph, GraphError> {
        self.code.clear();
        self.var_counter = 0;
        self.var_map.clear();

        let sorted = graph.topological_sort();

        for node_id in sorted {
            self.compile_node(graph, node_id)?;
        }

        Ok(CompiledGraph {
            name: graph.name().into(),
            fragment_code: self.code.clone(),
            vertex_code: String::new(),
            uniforms: Vec::new(),
            textures: Vec::new(),
        })
    }

    /// Compile a single node.
    fn compile_node(&mut self, graph: &ShaderGraph, id: NodeId) -> Result<(), GraphError> {
        let node = graph.node(id).ok_or(GraphError::NodeNotFound)?;

        // Generate output variable
        let out_var = format!("_n{}_{}", id.0, node.outputs.get(0).map(|o| &o.name[..]).unwrap_or("out"));
        
        // Store in var map
        for (i, _) in node.outputs.iter().enumerate() {
            self.var_map.insert((id.0, i as u32), out_var.clone());
        }

        let glsl = node.generate_glsl(&out_var);
        self.code.push_str(&glsl);
        self.code.push('\n');

        Ok(())
    }
}

impl Default for GraphCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled shader graph.
#[derive(Debug, Clone)]
pub struct CompiledGraph {
    /// Graph name.
    pub name: String,
    /// Fragment shader code.
    pub fragment_code: String,
    /// Vertex shader code.
    pub vertex_code: String,
    /// Uniforms.
    pub uniforms: Vec<(String, DataType)>,
    /// Textures.
    pub textures: Vec<String>,
}
