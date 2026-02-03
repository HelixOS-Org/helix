//! IR Functions
//!
//! This module defines function representations in the Lumina IR.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::types::{IrType, AddressSpace, BuiltinKind};
use crate::instruction::{FunctionId, BlockId, FunctionControl};
use crate::block::{BasicBlock, BlockMap};
use crate::value::{ValueId, ValueTable};

/// Shader execution model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExecutionModel {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    GLCompute,
    Kernel,
    TaskNV,
    MeshNV,
    TaskEXT,
    MeshEXT,
    RayGenerationKHR,
    IntersectionKHR,
    AnyHitKHR,
    ClosestHitKHR,
    MissKHR,
    CallableKHR,
}

impl ExecutionModel {
    /// Get the name of the execution model
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::TessellationControl => "tess_control",
            Self::TessellationEvaluation => "tess_eval",
            Self::Geometry => "geometry",
            Self::Fragment => "fragment",
            Self::GLCompute => "compute",
            Self::Kernel => "kernel",
            Self::TaskNV => "task_nv",
            Self::MeshNV => "mesh_nv",
            Self::TaskEXT => "task",
            Self::MeshEXT => "mesh",
            Self::RayGenerationKHR => "ray_gen",
            Self::IntersectionKHR => "intersection",
            Self::AnyHitKHR => "any_hit",
            Self::ClosestHitKHR => "closest_hit",
            Self::MissKHR => "miss",
            Self::CallableKHR => "callable",
        }
    }

    /// Check if this is a raytracing shader
    pub const fn is_raytracing(&self) -> bool {
        matches!(
            self,
            Self::RayGenerationKHR
                | Self::IntersectionKHR
                | Self::AnyHitKHR
                | Self::ClosestHitKHR
                | Self::MissKHR
                | Self::CallableKHR
        )
    }

    /// Check if this is a mesh shader
    pub const fn is_mesh_shader(&self) -> bool {
        matches!(self, Self::TaskNV | Self::MeshNV | Self::TaskEXT | Self::MeshEXT)
    }

    /// Check if this is a compute shader
    pub const fn is_compute(&self) -> bool {
        matches!(self, Self::GLCompute | Self::Kernel)
    }

    /// Check if this uses workgroups
    pub const fn uses_workgroups(&self) -> bool {
        self.is_compute() || self.is_mesh_shader() || matches!(self, Self::TaskNV | Self::TaskEXT)
    }
}

/// Execution mode for shader entry points
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    /// Invocations per primitive for geometry shader
    Invocations(u32),
    /// Spacing for tessellation
    SpacingEqual,
    SpacingFractionalEven,
    SpacingFractionalOdd,
    /// Vertex order for tessellation
    VertexOrderCw,
    VertexOrderCcw,
    /// Point mode for tessellation
    PointMode,
    /// Primitive topology output
    OutputPoints,
    OutputLineStrip,
    OutputTriangleStrip,
    OutputLinesEXT,
    OutputTrianglesEXT,
    /// Tessellation domain
    Triangles,
    Quads,
    Isolines,
    /// Fragment origin
    OriginUpperLeft,
    OriginLowerLeft,
    /// Early fragment tests
    EarlyFragmentTests,
    /// Depth mode
    DepthReplacing,
    DepthGreater,
    DepthLess,
    DepthUnchanged,
    /// Local size for compute
    LocalSize { x: u32, y: u32, z: u32 },
    LocalSizeId { x: ValueId, y: ValueId, z: ValueId },
    LocalSizeHint { x: u32, y: u32, z: u32 },
    /// Max output vertices for geometry/mesh
    OutputVertices(u32),
    /// Max output primitives for mesh
    OutputPrimitivesEXT(u32),
    /// Derivative group
    DerivativeGroupQuadsKHR,
    DerivativeGroupLinearKHR,
    /// Subgroup size
    SubgroupSize(u32),
    SubgroupsPerWorkgroupId(ValueId),
    /// Pixel interlock
    PixelInterlockOrderedEXT,
    PixelInterlockUnorderedEXT,
    SampleInterlockOrderedEXT,
    SampleInterlockUnorderedEXT,
    ShadingRateInterlockOrderedEXT,
    ShadingRateInterlockUnorderedEXT,
    /// Stencil export
    StencilRefReplacingEXT,
    /// Post depth coverage
    PostDepthCoverage,
    /// Denorm mode
    DenormPreserve { width: u32 },
    DenormFlushToZero { width: u32 },
    /// Signed zero infinity nan preservation
    SignedZeroInfNanPreserve { width: u32 },
    /// Rounding mode
    RoundingModeRTE { width: u32 },
    RoundingModeRTZ { width: u32 },
    /// Early and late fragment tests
    EarlyAndLateFragmentTestsAMD,
    /// Maximum primitives for mesh
    MaxNodeRecursionAMDX(u32),
    /// Shader index for callable
    ShaderIndexAMDX(u32),
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter value ID
    pub value_id: ValueId,
    /// Parameter type
    pub ty: IrType,
    /// Parameter name
    pub name: Option<String>,
    /// Is this by-reference
    pub by_ref: bool,
    /// Decorations
    pub decorations: ParameterDecorations,
}

/// Parameter decorations
#[derive(Debug, Clone, Default)]
pub struct ParameterDecorations {
    /// Flat interpolation
    pub flat: bool,
    /// No perspective interpolation
    pub no_perspective: bool,
    /// Centroid interpolation
    pub centroid: bool,
    /// Sample interpolation
    pub sample: bool,
    /// Invariant
    pub invariant: bool,
    /// Restrict
    pub restrict: bool,
    /// Aliased
    pub aliased: bool,
    /// Volatile
    pub volatile: bool,
    /// Coherent
    pub coherent: bool,
    /// NonWritable (readonly)
    pub non_writable: bool,
    /// NonReadable (writeonly)
    pub non_readable: bool,
}

/// Function definition
#[derive(Debug, Clone)]
pub struct Function {
    /// Function ID
    pub id: FunctionId,
    /// Function name
    pub name: String,
    /// Return type
    pub return_type: IrType,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Basic blocks
    pub blocks: BlockMap,
    /// Local variables
    pub locals: Vec<LocalVariable>,
    /// Function control hints
    pub control: FunctionControl,
    /// Is this an entry point
    pub is_entry_point: bool,
    /// Execution model (for entry points)
    pub execution_model: Option<ExecutionModel>,
    /// Execution modes (for entry points)
    pub execution_modes: Vec<ExecutionMode>,
    /// Interface variables (for entry points)
    pub interface: Vec<ValueId>,
}

impl Function {
    /// Create a new function
    pub fn new(id: FunctionId, name: impl Into<String>, return_type: IrType) -> Self {
        Self {
            id,
            name: name.into(),
            return_type,
            parameters: Vec::new(),
            blocks: BlockMap::new(),
            locals: Vec::new(),
            control: FunctionControl::default(),
            is_entry_point: false,
            execution_model: None,
            execution_modes: Vec::new(),
            interface: Vec::new(),
        }
    }

    /// Create a new entry point
    pub fn entry_point(
        id: FunctionId,
        name: impl Into<String>,
        execution_model: ExecutionModel,
    ) -> Self {
        let mut func = Self::new(id, name, IrType::void());
        func.is_entry_point = true;
        func.execution_model = Some(execution_model);
        func
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, value_id: ValueId, ty: IrType) -> &mut Parameter {
        self.parameters.push(Parameter {
            value_id,
            ty,
            name: None,
            by_ref: false,
            decorations: ParameterDecorations::default(),
        });
        self.parameters.last_mut().unwrap()
    }

    /// Add a local variable
    pub fn add_local(&mut self, local: LocalVariable) {
        self.locals.push(local);
    }

    /// Get entry block
    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.entry().and_then(|id| self.blocks.get(id))
    }

    /// Get entry block mutably
    pub fn entry_block_mut(&mut self) -> Option<&mut BasicBlock> {
        if let Some(id) = self.blocks.entry() {
            self.blocks.get_mut(id)
        } else {
            None
        }
    }

    /// Create entry block if not exists
    pub fn ensure_entry_block(&mut self) -> BlockId {
        if let Some(id) = self.blocks.entry() {
            id
        } else {
            self.blocks.create_entry_block()
        }
    }

    /// Add an execution mode
    pub fn add_execution_mode(&mut self, mode: ExecutionMode) {
        self.execution_modes.push(mode);
    }

    /// Set local workgroup size
    pub fn set_local_size(&mut self, x: u32, y: u32, z: u32) {
        self.execution_modes.retain(|m| !matches!(m, ExecutionMode::LocalSize { .. }));
        self.execution_modes.push(ExecutionMode::LocalSize { x, y, z });
    }

    /// Get local workgroup size
    pub fn local_size(&self) -> Option<(u32, u32, u32)> {
        self.execution_modes.iter().find_map(|m| {
            if let ExecutionMode::LocalSize { x, y, z } = m {
                Some((*x, *y, *z))
            } else {
                None
            }
        })
    }

    /// Add interface variable
    pub fn add_interface(&mut self, var: ValueId) {
        if !self.interface.contains(&var) {
            self.interface.push(var);
        }
    }

    /// Check if function has side effects
    pub fn has_side_effects(&self) -> bool {
        for block in self.blocks.iter() {
            for inst in block.iter() {
                if inst.has_side_effects() {
                    return true;
                }
            }
        }
        false
    }

    /// Check if function is pure (no side effects)
    pub fn is_pure(&self) -> bool {
        !self.has_side_effects()
    }

    /// Get all blocks
    pub fn blocks(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks.iter()
    }

    /// Get instruction count
    pub fn instruction_count(&self) -> usize {
        self.blocks.iter().map(|b| b.len()).sum()
    }

    /// Get block count
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

/// Local variable
#[derive(Debug, Clone)]
pub struct LocalVariable {
    /// Value ID
    pub value_id: ValueId,
    /// Type (pointer type)
    pub ty: IrType,
    /// Name
    pub name: Option<String>,
    /// Initializer
    pub initializer: Option<ValueId>,
}

impl LocalVariable {
    /// Create a new local variable
    pub fn new(value_id: ValueId, ty: IrType) -> Self {
        Self {
            value_id,
            ty,
            name: None,
            initializer: None,
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the initializer
    pub fn with_initializer(mut self, init: ValueId) -> Self {
        self.initializer = Some(init);
        self
    }
}

/// Global variable
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    /// Value ID
    pub value_id: ValueId,
    /// Type
    pub ty: IrType,
    /// Address space
    pub address_space: AddressSpace,
    /// Name
    pub name: Option<String>,
    /// Initializer
    pub initializer: Option<ValueId>,
    /// Decorations
    pub decorations: GlobalDecorations,
}

/// Global variable decorations
#[derive(Debug, Clone, Default)]
pub struct GlobalDecorations {
    /// Binding point
    pub binding: Option<u32>,
    /// Descriptor set
    pub descriptor_set: Option<u32>,
    /// Location
    pub location: Option<u32>,
    /// Builtin
    pub builtin: Option<BuiltinKind>,
    /// Input attachment index
    pub input_attachment_index: Option<u32>,
    /// Flat interpolation
    pub flat: bool,
    /// No perspective interpolation
    pub no_perspective: bool,
    /// Centroid interpolation
    pub centroid: bool,
    /// Sample interpolation
    pub sample: bool,
    /// Invariant
    pub invariant: bool,
    /// Non-writable (readonly)
    pub non_writable: bool,
    /// Non-readable (writeonly)
    pub non_readable: bool,
    /// Restrict
    pub restrict: bool,
    /// Coherent
    pub coherent: bool,
    /// Volatile
    pub volatile: bool,
    /// Component
    pub component: Option<u32>,
    /// Index (for dual-source blending)
    pub index: Option<u32>,
    /// Offset (explicit offset in struct)
    pub offset: Option<u32>,
    /// XfbBuffer
    pub xfb_buffer: Option<u32>,
    /// XfbStride
    pub xfb_stride: Option<u32>,
    /// Aliased
    pub aliased: bool,
    /// Per-vertex (for mesh shaders)
    pub per_vertex: bool,
    /// Per-primitive (for mesh shaders)
    pub per_primitive: bool,
}

impl GlobalVariable {
    /// Create a new global variable
    pub fn new(value_id: ValueId, ty: IrType, address_space: AddressSpace) -> Self {
        Self {
            value_id,
            ty,
            address_space,
            name: None,
            initializer: None,
            decorations: GlobalDecorations::default(),
        }
    }

    /// Create an input variable
    pub fn input(value_id: ValueId, ty: IrType, location: u32) -> Self {
        let mut var = Self::new(value_id, ty, AddressSpace::Input);
        var.decorations.location = Some(location);
        var
    }

    /// Create an output variable
    pub fn output(value_id: ValueId, ty: IrType, location: u32) -> Self {
        let mut var = Self::new(value_id, ty, AddressSpace::Output);
        var.decorations.location = Some(location);
        var
    }

    /// Create a uniform buffer variable
    pub fn uniform(value_id: ValueId, ty: IrType, set: u32, binding: u32) -> Self {
        let mut var = Self::new(value_id, ty, AddressSpace::Uniform);
        var.decorations.descriptor_set = Some(set);
        var.decorations.binding = Some(binding);
        var
    }

    /// Create a storage buffer variable
    pub fn storage(value_id: ValueId, ty: IrType, set: u32, binding: u32) -> Self {
        let mut var = Self::new(value_id, ty, AddressSpace::StorageBuffer);
        var.decorations.descriptor_set = Some(set);
        var.decorations.binding = Some(binding);
        var
    }

    /// Create a push constant variable
    pub fn push_constant(value_id: ValueId, ty: IrType) -> Self {
        Self::new(value_id, ty, AddressSpace::PushConstant)
    }

    /// Create a builtin variable
    pub fn builtin(value_id: ValueId, ty: IrType, builtin: BuiltinKind, is_input: bool) -> Self {
        let address_space = if is_input {
            AddressSpace::Input
        } else {
            AddressSpace::Output
        };
        let mut var = Self::new(value_id, ty, address_space);
        var.decorations.builtin = Some(builtin);
        var
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Check if this is an input variable
    pub fn is_input(&self) -> bool {
        self.address_space == AddressSpace::Input
    }

    /// Check if this is an output variable
    pub fn is_output(&self) -> bool {
        self.address_space == AddressSpace::Output
    }

    /// Check if this is a uniform variable
    pub fn is_uniform(&self) -> bool {
        self.address_space == AddressSpace::Uniform
    }

    /// Check if this is a storage buffer
    pub fn is_storage(&self) -> bool {
        self.address_space == AddressSpace::StorageBuffer
    }

    /// Check if this is a push constant
    pub fn is_push_constant(&self) -> bool {
        self.address_space == AddressSpace::PushConstant
    }

    /// Check if this is a builtin variable
    pub fn is_builtin(&self) -> bool {
        self.decorations.builtin.is_some()
    }

    /// Get the binding info
    pub fn binding_info(&self) -> Option<(u32, u32)> {
        match (self.decorations.descriptor_set, self.decorations.binding) {
            (Some(set), Some(binding)) => Some((set, binding)),
            _ => None,
        }
    }
}

/// Function collection
#[derive(Debug, Default)]
pub struct FunctionMap {
    functions: Vec<Function>,
    entry_points: Vec<FunctionId>,
    next_id: FunctionId,
}

impl FunctionMap {
    /// Create a new function map
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            entry_points: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a new function
    pub fn create_function(&mut self, name: impl Into<String>, return_type: IrType) -> FunctionId {
        let id = self.next_id;
        self.next_id += 1;
        self.functions.push(Function::new(id, name, return_type));
        id
    }

    /// Create a new entry point
    pub fn create_entry_point(
        &mut self,
        name: impl Into<String>,
        execution_model: ExecutionModel,
    ) -> FunctionId {
        let id = self.next_id;
        self.next_id += 1;
        self.functions.push(Function::entry_point(id, name, execution_model));
        self.entry_points.push(id);
        id
    }

    /// Get a function by ID
    pub fn get(&self, id: FunctionId) -> Option<&Function> {
        self.functions.iter().find(|f| f.id == id)
    }

    /// Get a mutable function by ID
    pub fn get_mut(&mut self, id: FunctionId) -> Option<&mut Function> {
        self.functions.iter_mut().find(|f| f.id == id)
    }

    /// Get a function by name
    pub fn get_by_name(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Get entry points
    pub fn entry_points(&self) -> &[FunctionId] {
        &self.entry_points
    }

    /// Iterate over all functions
    pub fn iter(&self) -> impl Iterator<Item = &Function> {
        self.functions.iter()
    }

    /// Iterate over all functions mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Function> {
        self.functions.iter_mut()
    }

    /// Get the number of functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

/// Call graph for interprocedural analysis
#[derive(Debug, Default)]
pub struct CallGraph {
    /// Edges: caller -> callees
    edges: Vec<(FunctionId, Vec<FunctionId>)>,
}

impl CallGraph {
    /// Create a new call graph
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Build call graph from functions
    pub fn build(functions: &FunctionMap) -> Self {
        use crate::instruction::Instruction;
        
        let mut graph = Self::new();
        
        for func in functions.iter() {
            let mut callees = Vec::new();
            
            for block in func.blocks.iter() {
                for inst in block.iter() {
                    if let Instruction::FunctionCall { function, .. } = inst {
                        if !callees.contains(function) {
                            callees.push(*function);
                        }
                    }
                }
            }
            
            graph.edges.push((func.id, callees));
        }
        
        graph
    }

    /// Get callees for a function
    pub fn callees(&self, func: FunctionId) -> &[FunctionId] {
        self.edges
            .iter()
            .find(|(f, _)| *f == func)
            .map(|(_, c)| c.as_slice())
            .unwrap_or(&[])
    }

    /// Get callers for a function
    pub fn callers(&self, func: FunctionId) -> Vec<FunctionId> {
        self.edges
            .iter()
            .filter(|(_, callees)| callees.contains(&func))
            .map(|(f, _)| *f)
            .collect()
    }

    /// Check if a function is recursive (directly or indirectly)
    pub fn is_recursive(&self, func: FunctionId) -> bool {
        let mut visited = Vec::new();
        let mut stack = vec![func];
        
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                if current == func {
                    return true;
                }
                continue;
            }
            visited.push(current);
            stack.extend(self.callees(current));
        }
        
        false
    }

    /// Get all functions reachable from entry points
    pub fn reachable_from(&self, entry_points: &[FunctionId]) -> Vec<FunctionId> {
        let mut visited = Vec::new();
        let mut stack: Vec<_> = entry_points.to_vec();
        
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);
            stack.extend(self.callees(current));
        }
        
        visited
    }

    /// Topological sort (for inlining order)
    pub fn topological_order(&self) -> Vec<FunctionId> {
        let mut result = Vec::new();
        let mut visited = Vec::new();
        
        fn visit(
            func: FunctionId,
            graph: &CallGraph,
            visited: &mut Vec<FunctionId>,
            result: &mut Vec<FunctionId>,
        ) {
            if visited.contains(&func) {
                return;
            }
            visited.push(func);
            
            for &callee in graph.callees(func) {
                visit(callee, graph, visited, result);
            }
            
            result.push(func);
        }
        
        for (func, _) in &self.edges {
            visit(*func, self, &mut visited, &mut result);
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_creation() {
        let func = Function::new(0, "main", IrType::void());
        assert_eq!(func.name, "main");
        assert!(!func.is_entry_point);
    }

    #[test]
    fn test_entry_point() {
        let func = Function::entry_point(0, "main", ExecutionModel::Fragment);
        assert!(func.is_entry_point);
        assert_eq!(func.execution_model, Some(ExecutionModel::Fragment));
    }

    #[test]
    fn test_local_size() {
        let mut func = Function::entry_point(0, "main", ExecutionModel::GLCompute);
        func.set_local_size(8, 8, 1);
        assert_eq!(func.local_size(), Some((8, 8, 1)));
    }

    #[test]
    fn test_global_variable() {
        let var = GlobalVariable::uniform(0, IrType::mat4f(), 0, 1);
        assert!(var.is_uniform());
        assert_eq!(var.binding_info(), Some((0, 1)));
    }

    #[test]
    fn test_function_map() {
        let mut map = FunctionMap::new();
        let f1 = map.create_function("helper", IrType::f32());
        let f2 = map.create_entry_point("main", ExecutionModel::Vertex);
        
        assert_eq!(map.len(), 2);
        assert_eq!(map.entry_points().len(), 1);
        assert_eq!(map.entry_points()[0], f2);
    }

    #[test]
    fn test_execution_model() {
        assert!(ExecutionModel::GLCompute.is_compute());
        assert!(ExecutionModel::RayGenerationKHR.is_raytracing());
        assert!(ExecutionModel::MeshEXT.is_mesh_shader());
        assert!(ExecutionModel::GLCompute.uses_workgroups());
    }
}
