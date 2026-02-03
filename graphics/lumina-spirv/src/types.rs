//! SPIR-V Type System
//!
//! Type representation for SPIR-V code generation.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, vec::Vec};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use crate::instruction::{
    AddressingModel, Capability, Decoration, Dim, Id, ImageFormat, MemoryModel, StorageClass,
};

/// SPIR-V type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpirVType {
    /// Void type
    Void,
    /// Boolean type
    Bool,
    /// Integer type
    Int {
        /// Width in bits
        width: u32,
        /// Whether signed
        signed: bool,
    },
    /// Floating point type
    Float {
        /// Width in bits
        width: u32,
    },
    /// Vector type
    Vector {
        /// Component type ID
        component: Id,
        /// Number of components
        count: u32,
    },
    /// Matrix type
    Matrix {
        /// Column type ID (must be vector)
        column: Id,
        /// Number of columns
        columns: u32,
    },
    /// Array type
    Array {
        /// Element type ID
        element: Id,
        /// Length (constant ID)
        length: Id,
    },
    /// Runtime array type
    RuntimeArray {
        /// Element type ID
        element: Id,
    },
    /// Struct type
    Struct {
        /// Member type IDs
        members: Vec<Id>,
        /// Member decorations
        decorations: Vec<MemberDecorations>,
    },
    /// Pointer type
    Pointer {
        /// Storage class
        storage_class: StorageClass,
        /// Pointee type ID
        pointee: Id,
    },
    /// Function type
    Function {
        /// Return type ID
        return_type: Id,
        /// Parameter type IDs
        parameters: Vec<Id>,
    },
    /// Image type
    Image {
        /// Sampled type ID
        sampled_type: Id,
        /// Dimension
        dim: Dim,
        /// Depth flag (0 = not depth, 1 = depth, 2 = unknown)
        depth: u32,
        /// Arrayed flag
        arrayed: bool,
        /// Multisampled flag
        multisampled: bool,
        /// Sampled flag (0 = runtime, 1 = sampled, 2 = storage)
        sampled: u32,
        /// Image format
        format: ImageFormat,
    },
    /// Sampler type
    Sampler,
    /// Sampled image type
    SampledImage {
        /// Image type ID
        image: Id,
    },
    /// Acceleration structure type
    AccelerationStructure,
    /// Ray query type
    RayQuery,
}

/// Member decorations
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct MemberDecorations {
    /// Offset in bytes
    pub offset: Option<u32>,
    /// Array stride
    pub array_stride: Option<u32>,
    /// Matrix stride
    pub matrix_stride: Option<u32>,
    /// Row major layout
    pub row_major: bool,
    /// Column major layout
    pub col_major: bool,
    /// Built-in value
    pub builtin: Option<BuiltIn>,
    /// No perspective interpolation
    pub no_perspective: bool,
    /// Flat interpolation
    pub flat: bool,
    /// Centroid interpolation
    pub centroid: bool,
    /// Sample interpolation
    pub sample: bool,
    /// Non-writable
    pub non_writable: bool,
    /// Non-readable
    pub non_readable: bool,
}

/// Built-in values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BuiltIn {
    Position             = 0,
    PointSize            = 1,
    ClipDistance         = 3,
    CullDistance         = 4,
    VertexId             = 5,
    InstanceId           = 6,
    PrimitiveId          = 7,
    InvocationId         = 8,
    Layer                = 9,
    ViewportIndex        = 10,
    TessLevelOuter       = 11,
    TessLevelInner       = 12,
    TessCoord            = 13,
    PatchVertices        = 14,
    FragCoord            = 15,
    PointCoord           = 16,
    FrontFacing          = 17,
    SampleId             = 18,
    SamplePosition       = 19,
    SampleMask           = 20,
    FragDepth            = 22,
    HelperInvocation     = 23,
    NumWorkgroups        = 24,
    WorkgroupSize        = 25,
    WorkgroupId          = 26,
    LocalInvocationId    = 27,
    GlobalInvocationId   = 28,
    LocalInvocationIndex = 29,
    WorkDim              = 30,
    GlobalSize           = 31,
    EnqueuedWorkgroupSize = 32,
    GlobalOffset         = 33,
    GlobalLinearId       = 34,
    SubgroupSize         = 36,
    SubgroupMaxSize      = 37,
    NumSubgroups         = 38,
    NumEnqueuedSubgroups = 39,
    SubgroupId           = 40,
    SubgroupLocalInvocationId = 41,
    VertexIndex          = 42,
    InstanceIndex        = 43,
    SubgroupEqMask       = 4416,
    SubgroupGeMask       = 4417,
    SubgroupGtMask       = 4418,
    SubgroupLeMask       = 4419,
    SubgroupLtMask       = 4420,
    BaseVertex           = 4424,
    BaseInstance         = 4425,
    DrawIndex            = 4426,
    PrimitiveShadingRateKHR = 4432,
    DeviceIndex          = 4438,
    ViewIndex            = 4440,
    ShadingRateKHR       = 4444,
    BaryCoordNoPerspAMD  = 4992,
    BaryCoordNoPerspCentroidAMD = 4993,
    BaryCoordNoPerspSampleAMD = 4994,
    BaryCoordSmoothAMD   = 4995,
    BaryCoordSmoothCentroidAMD = 4996,
    BaryCoordSmoothSampleAMD = 4997,
    BaryCoordPullModelAMD = 4998,
    FragStencilRefEXT    = 5014,
    ViewportMaskNV       = 5253,
    SecondaryPositionNV  = 5257,
    SecondaryViewportMaskNV = 5258,
    PositionPerViewNV    = 5261,
    ViewportMaskPerViewNV = 5262,
    FullyCoveredEXT      = 5264,
    TaskCountNV          = 5274,
    PrimitiveCountNV     = 5275,
    PrimitiveIndicesNV   = 5276,
    ClipDistancePerViewNV = 5277,
    CullDistancePerViewNV = 5278,
    LayerPerViewNV       = 5279,
    MeshViewCountNV      = 5280,
    MeshViewIndicesNV    = 5281,
    BaryCoordKHR         = 5286,
    BaryCoordNoPerspKHR  = 5287,
    FragSizeEXT          = 5292,
    FragInvocationCountEXT = 5293,
    PrimitivePointIndicesEXT = 5294,
    PrimitiveLineIndicesEXT = 5295,
    PrimitiveTriangleIndicesEXT = 5296,
    CullPrimitiveEXT     = 5299,
    LaunchIdKHR          = 5319,
    LaunchSizeKHR        = 5320,
    WorldRayOriginKHR    = 5321,
    WorldRayDirectionKHR = 5322,
    ObjectRayOriginKHR   = 5323,
    ObjectRayDirectionKHR = 5324,
    RayTminKHR           = 5325,
    RayTmaxKHR           = 5326,
    InstanceCustomIndexKHR = 5327,
    ObjectToWorldKHR     = 5330,
    WorldToObjectKHR     = 5331,
    HitTNV               = 5332,
    HitKindKHR           = 5333,
    CurrentRayTimeNV     = 5334,
    HitTriangleVertexPositionsKHR = 5335,
    IncomingRayFlagsKHR  = 5351,
    RayGeometryIndexKHR  = 5352,
    WarpsPerSMNV         = 5374,
    SMCountNV            = 5375,
    WarpIDNV             = 5376,
    SMIDNV               = 5377,
    CullMaskKHR          = 6021,
}

/// Type registry for SPIR-V generation
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Types by ID
    types: BTreeMap<Id, SpirVType>,
    /// ID lookup by type
    type_ids: BTreeMap<SpirVType, Id>,
    /// Next available ID
    next_id: Id,
    /// Required capabilities
    capabilities: Vec<Capability>,
}

impl TypeRegistry {
    /// Create a new type registry
    pub fn new() -> Self {
        Self {
            types: BTreeMap::new(),
            type_ids: BTreeMap::new(),
            next_id: 1,
            capabilities: Vec::new(),
        }
    }

    /// Allocate a new ID
    pub fn alloc_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Register a type
    pub fn register(&mut self, ty: SpirVType) -> Id {
        if let Some(&id) = self.type_ids.get(&ty) {
            return id;
        }

        // Check for required capabilities
        self.check_capabilities(&ty);

        let id = self.alloc_id();
        self.types.insert(id, ty.clone());
        self.type_ids.insert(ty, id);
        id
    }

    /// Get a type by ID
    pub fn get(&self, id: Id) -> Option<&SpirVType> {
        self.types.get(&id)
    }

    /// Get ID for a type
    pub fn get_id(&self, ty: &SpirVType) -> Option<Id> {
        self.type_ids.get(ty).copied()
    }

    /// Get all types
    pub fn types(&self) -> impl Iterator<Item = (Id, &SpirVType)> {
        self.types.iter().map(|(&id, ty)| (id, ty))
    }

    /// Get required capabilities
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Check and add required capabilities
    fn check_capabilities(&mut self, ty: &SpirVType) {
        match ty {
            SpirVType::Float { width: 16 } => {
                self.add_capability(Capability::Float16);
            },
            SpirVType::Float { width: 64 } => {
                self.add_capability(Capability::Float64);
            },
            SpirVType::Int { width: 8, .. } => {
                self.add_capability(Capability::Int8);
            },
            SpirVType::Int { width: 16, .. } => {
                self.add_capability(Capability::Int16);
            },
            SpirVType::Int { width: 64, .. } => {
                self.add_capability(Capability::Int64);
            },
            SpirVType::Image {
                dim: Dim::Dim1D, ..
            } => {
                self.add_capability(Capability::Sampled1D);
            },
            SpirVType::Image { dim: Dim::Cube, .. } => {
                self.add_capability(Capability::ImageCubeArray);
            },
            SpirVType::Image { dim: Dim::Rect, .. } => {
                self.add_capability(Capability::SampledRect);
            },
            SpirVType::Image {
                dim: Dim::Buffer, ..
            } => {
                self.add_capability(Capability::SampledBuffer);
            },
            SpirVType::AccelerationStructure => {
                self.add_capability(Capability::RayTracingKHR);
            },
            SpirVType::RayQuery => {
                self.add_capability(Capability::RayQueryKHR);
            },
            _ => {},
        }
    }

    /// Add a capability if not already present
    fn add_capability(&mut self, cap: Capability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    // Common type helpers

    /// Register void type
    pub fn void(&mut self) -> Id {
        self.register(SpirVType::Void)
    }

    /// Register bool type
    pub fn bool(&mut self) -> Id {
        self.register(SpirVType::Bool)
    }

    /// Register i32 type
    pub fn i32(&mut self) -> Id {
        self.register(SpirVType::Int {
            width: 32,
            signed: true,
        })
    }

    /// Register u32 type
    pub fn u32(&mut self) -> Id {
        self.register(SpirVType::Int {
            width: 32,
            signed: false,
        })
    }

    /// Register i64 type
    pub fn i64(&mut self) -> Id {
        self.register(SpirVType::Int {
            width: 64,
            signed: true,
        })
    }

    /// Register u64 type
    pub fn u64(&mut self) -> Id {
        self.register(SpirVType::Int {
            width: 64,
            signed: false,
        })
    }

    /// Register f32 type
    pub fn f32(&mut self) -> Id {
        self.register(SpirVType::Float { width: 32 })
    }

    /// Register f64 type
    pub fn f64(&mut self) -> Id {
        self.register(SpirVType::Float { width: 64 })
    }

    /// Register f16 type
    pub fn f16(&mut self) -> Id {
        self.register(SpirVType::Float { width: 16 })
    }

    /// Register vec2 type
    pub fn vec2(&mut self) -> Id {
        let f32_id = self.f32();
        self.register(SpirVType::Vector {
            component: f32_id,
            count: 2,
        })
    }

    /// Register vec3 type
    pub fn vec3(&mut self) -> Id {
        let f32_id = self.f32();
        self.register(SpirVType::Vector {
            component: f32_id,
            count: 3,
        })
    }

    /// Register vec4 type
    pub fn vec4(&mut self) -> Id {
        let f32_id = self.f32();
        self.register(SpirVType::Vector {
            component: f32_id,
            count: 4,
        })
    }

    /// Register ivec2 type
    pub fn ivec2(&mut self) -> Id {
        let i32_id = self.i32();
        self.register(SpirVType::Vector {
            component: i32_id,
            count: 2,
        })
    }

    /// Register ivec3 type
    pub fn ivec3(&mut self) -> Id {
        let i32_id = self.i32();
        self.register(SpirVType::Vector {
            component: i32_id,
            count: 3,
        })
    }

    /// Register ivec4 type
    pub fn ivec4(&mut self) -> Id {
        let i32_id = self.i32();
        self.register(SpirVType::Vector {
            component: i32_id,
            count: 4,
        })
    }

    /// Register uvec2 type
    pub fn uvec2(&mut self) -> Id {
        let u32_id = self.u32();
        self.register(SpirVType::Vector {
            component: u32_id,
            count: 2,
        })
    }

    /// Register uvec3 type
    pub fn uvec3(&mut self) -> Id {
        let u32_id = self.u32();
        self.register(SpirVType::Vector {
            component: u32_id,
            count: 3,
        })
    }

    /// Register uvec4 type
    pub fn uvec4(&mut self) -> Id {
        let u32_id = self.u32();
        self.register(SpirVType::Vector {
            component: u32_id,
            count: 4,
        })
    }

    /// Register bvec2 type
    pub fn bvec2(&mut self) -> Id {
        let bool_id = self.bool();
        self.register(SpirVType::Vector {
            component: bool_id,
            count: 2,
        })
    }

    /// Register bvec3 type
    pub fn bvec3(&mut self) -> Id {
        let bool_id = self.bool();
        self.register(SpirVType::Vector {
            component: bool_id,
            count: 3,
        })
    }

    /// Register bvec4 type
    pub fn bvec4(&mut self) -> Id {
        let bool_id = self.bool();
        self.register(SpirVType::Vector {
            component: bool_id,
            count: 4,
        })
    }

    /// Register mat2 type
    pub fn mat2(&mut self) -> Id {
        let vec2_id = self.vec2();
        self.register(SpirVType::Matrix {
            column: vec2_id,
            columns: 2,
        })
    }

    /// Register mat3 type
    pub fn mat3(&mut self) -> Id {
        let vec3_id = self.vec3();
        self.register(SpirVType::Matrix {
            column: vec3_id,
            columns: 3,
        })
    }

    /// Register mat4 type
    pub fn mat4(&mut self) -> Id {
        let vec4_id = self.vec4();
        self.register(SpirVType::Matrix {
            column: vec4_id,
            columns: 4,
        })
    }

    /// Register mat2x3 type
    pub fn mat2x3(&mut self) -> Id {
        let vec3_id = self.vec3();
        self.register(SpirVType::Matrix {
            column: vec3_id,
            columns: 2,
        })
    }

    /// Register mat2x4 type
    pub fn mat2x4(&mut self) -> Id {
        let vec4_id = self.vec4();
        self.register(SpirVType::Matrix {
            column: vec4_id,
            columns: 2,
        })
    }

    /// Register mat3x2 type
    pub fn mat3x2(&mut self) -> Id {
        let vec2_id = self.vec2();
        self.register(SpirVType::Matrix {
            column: vec2_id,
            columns: 3,
        })
    }

    /// Register mat3x4 type
    pub fn mat3x4(&mut self) -> Id {
        let vec4_id = self.vec4();
        self.register(SpirVType::Matrix {
            column: vec4_id,
            columns: 3,
        })
    }

    /// Register mat4x2 type
    pub fn mat4x2(&mut self) -> Id {
        let vec2_id = self.vec2();
        self.register(SpirVType::Matrix {
            column: vec2_id,
            columns: 4,
        })
    }

    /// Register mat4x3 type
    pub fn mat4x3(&mut self) -> Id {
        let vec3_id = self.vec3();
        self.register(SpirVType::Matrix {
            column: vec3_id,
            columns: 4,
        })
    }

    /// Register a pointer type
    pub fn pointer(&mut self, storage_class: StorageClass, pointee: Id) -> Id {
        self.register(SpirVType::Pointer {
            storage_class,
            pointee,
        })
    }

    /// Register an array type
    pub fn array(&mut self, element: Id, length: Id) -> Id {
        self.register(SpirVType::Array { element, length })
    }

    /// Register a runtime array type
    pub fn runtime_array(&mut self, element: Id) -> Id {
        self.register(SpirVType::RuntimeArray { element })
    }

    /// Register a struct type
    pub fn struct_type(&mut self, members: Vec<Id>) -> Id {
        let decorations = vec![MemberDecorations::default(); members.len()];
        self.register(SpirVType::Struct {
            members,
            decorations,
        })
    }

    /// Register a function type
    pub fn function(&mut self, return_type: Id, parameters: Vec<Id>) -> Id {
        self.register(SpirVType::Function {
            return_type,
            parameters,
        })
    }

    /// Register a sampler type
    pub fn sampler(&mut self) -> Id {
        self.register(SpirVType::Sampler)
    }

    /// Register a 2D image type
    pub fn image_2d(&mut self, sampled_type: Id, format: ImageFormat) -> Id {
        self.register(SpirVType::Image {
            sampled_type,
            dim: Dim::Dim2D,
            depth: 0,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format,
        })
    }

    /// Register a 2D storage image type
    pub fn storage_image_2d(&mut self, sampled_type: Id, format: ImageFormat) -> Id {
        self.register(SpirVType::Image {
            sampled_type,
            dim: Dim::Dim2D,
            depth: 0,
            arrayed: false,
            multisampled: false,
            sampled: 2,
            format,
        })
    }

    /// Register a cube image type
    pub fn image_cube(&mut self, sampled_type: Id, format: ImageFormat) -> Id {
        self.register(SpirVType::Image {
            sampled_type,
            dim: Dim::Cube,
            depth: 0,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format,
        })
    }

    /// Register a sampled image type
    pub fn sampled_image(&mut self, image: Id) -> Id {
        self.register(SpirVType::SampledImage { image })
    }

    /// Register an acceleration structure type
    pub fn acceleration_structure(&mut self) -> Id {
        self.register(SpirVType::AccelerationStructure)
    }

    /// Register a ray query type
    pub fn ray_query(&mut self) -> Id {
        self.register(SpirVType::RayQuery)
    }
}

/// Layout calculator for std140/std430
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutRules {
    /// std140 layout (uniform buffers)
    Std140,
    /// std430 layout (storage buffers)
    Std430,
    /// Scalar layout extension
    Scalar,
}

impl LayoutRules {
    /// Calculate alignment for a type
    pub fn alignment(&self, registry: &TypeRegistry, ty_id: Id) -> u32 {
        let ty = registry.get(ty_id).expect("Unknown type");

        match ty {
            SpirVType::Bool | SpirVType::Int { width: 32, .. } | SpirVType::Float { width: 32 } => {
                4
            },
            SpirVType::Int { width: 64, .. } | SpirVType::Float { width: 64 } => 8,
            SpirVType::Int { width: 16, .. } | SpirVType::Float { width: 16 } => 2,
            SpirVType::Int { width: 8, .. } => 1,
            SpirVType::Vector { component, count } => {
                let comp_align = self.alignment(registry, *component);
                let n = if *count == 3 { 4 } else { *count };
                match self {
                    LayoutRules::Std140 | LayoutRules::Std430 => comp_align * n,
                    LayoutRules::Scalar => comp_align,
                }
            },
            SpirVType::Matrix { column, .. } => {
                // Matrix alignment is the alignment of a vec4
                match self {
                    LayoutRules::Std140 => 16,
                    LayoutRules::Std430 => self.alignment(registry, *column),
                    LayoutRules::Scalar => self.alignment(registry, *column),
                }
            },
            SpirVType::Array { element, .. } | SpirVType::RuntimeArray { element } => match self {
                LayoutRules::Std140 => {
                    let elem_align = self.alignment(registry, *element);
                    ((elem_align + 15) / 16) * 16
                },
                LayoutRules::Std430 | LayoutRules::Scalar => self.alignment(registry, *element),
            },
            SpirVType::Struct { members, .. } => {
                let max_align = members
                    .iter()
                    .map(|&m| self.alignment(registry, m))
                    .max()
                    .unwrap_or(1);
                match self {
                    LayoutRules::Std140 => ((max_align + 15) / 16) * 16,
                    LayoutRules::Std430 | LayoutRules::Scalar => max_align,
                }
            },
            _ => 4,
        }
    }

    /// Calculate size of a type
    pub fn size(&self, registry: &TypeRegistry, ty_id: Id) -> u32 {
        let ty = registry.get(ty_id).expect("Unknown type");

        match ty {
            SpirVType::Bool | SpirVType::Int { width: 32, .. } | SpirVType::Float { width: 32 } => {
                4
            },
            SpirVType::Int { width: 64, .. } | SpirVType::Float { width: 64 } => 8,
            SpirVType::Int { width: 16, .. } | SpirVType::Float { width: 16 } => 2,
            SpirVType::Int { width: 8, .. } => 1,
            SpirVType::Vector { component, count } => {
                let comp_size = self.size(registry, *component);
                comp_size * count
            },
            SpirVType::Matrix { column, columns } => {
                let col_size = self.size(registry, *column);
                let col_align = self.alignment(registry, *column);
                let stride = self.round_up(col_size, col_align);
                stride * columns
            },
            SpirVType::Array { element, .. } => {
                // We'd need to know the length value
                let elem_size = self.size(registry, *element);
                let elem_align = self.alignment(registry, *element);
                self.round_up(elem_size, elem_align)
            },
            SpirVType::Struct { members, .. } => {
                let mut offset = 0u32;
                for &member in members.iter() {
                    let align = self.alignment(registry, member);
                    offset = self.round_up(offset, align);
                    offset += self.size(registry, member);
                }
                let struct_align = self.alignment(registry, ty_id);
                self.round_up(offset, struct_align)
            },
            _ => 0,
        }
    }

    /// Round up to alignment
    fn round_up(&self, value: u32, align: u32) -> u32 {
        (value + align - 1) & !(align - 1)
    }

    /// Calculate member offsets for a struct
    pub fn struct_layout(&self, registry: &TypeRegistry, members: &[Id]) -> Vec<u32> {
        let mut offsets = Vec::with_capacity(members.len());
        let mut offset = 0u32;

        for &member in members {
            let align = self.alignment(registry, member);
            offset = self.round_up(offset, align);
            offsets.push(offset);
            offset += self.size(registry, member);
        }

        offsets
    }
}

/// Execution mode for entry points
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExecutionMode {
    Invocations          = 0,
    SpacingEqual         = 1,
    SpacingFractionalEven = 2,
    SpacingFractionalOdd = 3,
    VertexOrderCw        = 4,
    VertexOrderCcw       = 5,
    PixelCenterInteger   = 6,
    OriginUpperLeft      = 7,
    OriginLowerLeft      = 8,
    EarlyFragmentTests   = 9,
    PointMode            = 10,
    Xfb                  = 11,
    DepthReplacing       = 12,
    DepthGreater         = 14,
    DepthLess            = 15,
    DepthUnchanged       = 16,
    LocalSize            = 17,
    LocalSizeHint        = 18,
    InputPoints          = 19,
    InputLines           = 20,
    InputLinesAdjacency  = 21,
    Triangles            = 22,
    InputTrianglesAdjacency = 23,
    Quads                = 24,
    Isolines             = 25,
    OutputVertices       = 26,
    OutputPoints         = 27,
    OutputLineStrip      = 28,
    OutputTriangleStrip  = 29,
    VecTypeHint          = 30,
    ContractionOff       = 31,
    Initializer          = 33,
    Finalizer            = 34,
    SubgroupSize         = 35,
    SubgroupsPerWorkgroup = 36,
    SubgroupsPerWorkgroupId = 37,
    LocalSizeId          = 38,
    LocalSizeHintId      = 39,
    SubgroupUniformControlFlowKHR = 4421,
    PostDepthCoverage    = 4446,
    DenormPreserve       = 4459,
    DenormFlushToZero    = 4460,
    SignedZeroInfNanPreserve = 4461,
    RoundingModeRTE      = 4462,
    RoundingModeRTZ      = 4463,
    EarlyAndLateFragmentTestsAMD = 5017,
    StencilRefReplacingEXT = 5027,
    StencilRefUnchangedFrontAMD = 5079,
    StencilRefGreaterFrontAMD = 5080,
    StencilRefLessFrontAMD = 5081,
    StencilRefUnchangedBackAMD = 5082,
    StencilRefGreaterBackAMD = 5083,
    StencilRefLessBackAMD = 5084,
    OutputLinesEXT       = 5269,
    OutputLinesNV        = 5269,
    OutputPrimitivesEXT  = 5270,
    OutputPrimitivesNV   = 5270,
    DerivativeGroupQuadsNV = 5289,
    DerivativeGroupLinearNV = 5290,
    OutputTrianglesEXT   = 5298,
    OutputTrianglesNV    = 5298,
    PixelInterlockOrderedEXT = 5366,
    PixelInterlockUnorderedEXT = 5367,
    SampleInterlockOrderedEXT = 5368,
    SampleInterlockUnorderedEXT = 5369,
    ShadingRateInterlockOrderedEXT = 5370,
    ShadingRateInterlockUnorderedEXT = 5371,
    SharedLocalMemorySizeINTEL = 5618,
    RoundingModeRTPINTEL = 5620,
    RoundingModeRTNINTEL = 5621,
    FloatingPointModeALTINTEL = 5622,
    FloatingPointModeIEEEINTEL = 5623,
    MaxWorkgroupSizeINTEL = 5893,
    MaxWorkDimINTEL      = 5894,
    NoGlobalOffsetINTEL  = 5895,
    NumSIMDWorkitemsINTEL = 5896,
    SchedulerTargetFmaxMhzINTEL = 5903,
    NamedBarrierCountINTEL = 6417,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_registry() {
        let mut registry = TypeRegistry::new();

        let void_id = registry.void();
        let f32_id = registry.f32();
        let vec4_id = registry.vec4();

        assert_eq!(registry.get(void_id), Some(&SpirVType::Void));
        assert_eq!(registry.get(f32_id), Some(&SpirVType::Float { width: 32 }));

        // vec4 should reference f32
        if let Some(SpirVType::Vector { component, count }) = registry.get(vec4_id) {
            assert_eq!(*component, f32_id);
            assert_eq!(*count, 4);
        } else {
            panic!("Expected vector type");
        }
    }

    #[test]
    fn test_layout_std140() {
        let mut registry = TypeRegistry::new();
        let layout = LayoutRules::Std140;

        let f32_id = registry.f32();
        let vec3_id = registry.vec3();
        let vec4_id = registry.vec4();

        assert_eq!(layout.alignment(&registry, f32_id), 4);
        assert_eq!(layout.alignment(&registry, vec3_id), 16); // vec3 aligned as vec4
        assert_eq!(layout.alignment(&registry, vec4_id), 16);
    }
}
