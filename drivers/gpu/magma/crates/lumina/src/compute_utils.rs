//! Compute Utility Types for Lumina
//!
//! This module provides compute shader infrastructure including
//! dispatch commands, work group calculations, and common compute patterns.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Compute Handles
// ============================================================================

/// Compute pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputePipelineHandle(pub u64);

impl ComputePipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for ComputePipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compute shader handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputeShaderHandle(pub u64);

impl ComputeShaderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for ComputeShaderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Work Group
// ============================================================================

/// Work group size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct WorkGroupSize {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl WorkGroupSize {
    /// Common 1D size (256)
    pub const SIZE_256: Self = Self::new(256, 1, 1);
    /// Common 1D size (512)
    pub const SIZE_512: Self = Self::new(512, 1, 1);
    /// Common 1D size (1024)
    pub const SIZE_1024: Self = Self::new(1024, 1, 1);
    /// Common 2D size (8x8)
    pub const SIZE_8X8: Self = Self::new(8, 8, 1);
    /// Common 2D size (16x16)
    pub const SIZE_16X16: Self = Self::new(16, 16, 1);
    /// Common 2D size (32x32)
    pub const SIZE_32X32: Self = Self::new(32, 32, 1);
    /// Common 3D size (4x4x4)
    pub const SIZE_4X4X4: Self = Self::new(4, 4, 4);
    /// Common 3D size (8x8x8)
    pub const SIZE_8X8X8: Self = Self::new(8, 8, 8);

    /// Creates new work group size
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Creates 1D work group
    #[inline]
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// Creates 2D work group
    #[inline]
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// Creates 3D work group
    #[inline]
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    /// Total invocations
    #[inline]
    pub const fn total(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Is 1D
    #[inline]
    pub const fn is_1d(&self) -> bool {
        self.y == 1 && self.z == 1
    }

    /// Is 2D
    #[inline]
    pub const fn is_2d(&self) -> bool {
        self.z == 1 && self.y > 1
    }

    /// Is 3D
    #[inline]
    pub const fn is_3d(&self) -> bool {
        self.z > 1
    }

    /// Calculates dispatch count for data size
    pub const fn dispatch_count(&self, data_x: u32, data_y: u32, data_z: u32) -> DispatchSize {
        DispatchSize {
            x: div_ceil(data_x, self.x),
            y: div_ceil(data_y, self.y),
            z: div_ceil(data_z, self.z),
        }
    }

    /// Calculates 1D dispatch count
    pub const fn dispatch_1d(&self, count: u32) -> DispatchSize {
        self.dispatch_count(count, 1, 1)
    }

    /// Calculates 2D dispatch count
    pub const fn dispatch_2d(&self, width: u32, height: u32) -> DispatchSize {
        self.dispatch_count(width, height, 1)
    }

    /// Calculates 3D dispatch count
    pub const fn dispatch_3d(&self, width: u32, height: u32, depth: u32) -> DispatchSize {
        self.dispatch_count(width, height, depth)
    }
}

impl Default for WorkGroupSize {
    fn default() -> Self {
        Self::SIZE_256
    }
}

/// Ceiling division
const fn div_ceil(a: u32, b: u32) -> u32 {
    (a + b - 1) / b
}

/// Dispatch size (number of work groups)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DispatchSize {
    /// X groups
    pub x: u32,
    /// Y groups
    pub y: u32,
    /// Z groups
    pub z: u32,
}

impl DispatchSize {
    /// Creates dispatch size
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Creates 1D dispatch
    #[inline]
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// Creates 2D dispatch
    #[inline]
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// Creates 3D dispatch
    #[inline]
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    /// Total work groups
    #[inline]
    pub const fn total_groups(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Total invocations with work group size
    #[inline]
    pub const fn total_invocations(&self, wg_size: WorkGroupSize) -> u64 {
        self.total_groups() as u64 * wg_size.total() as u64
    }

    /// To array
    #[inline]
    pub const fn to_array(&self) -> [u32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Default for DispatchSize {
    fn default() -> Self {
        Self::d1(1)
    }
}

// ============================================================================
// Compute Pipeline Create Info
// ============================================================================

/// Compute pipeline create info
#[derive(Clone, Debug)]
pub struct ComputePipelineCreateInfo {
    /// Name
    pub name: String,
    /// Shader handle
    pub shader: ComputeShaderHandle,
    /// Entry point
    pub entry_point: String,
    /// Work group size (if not specified in shader)
    pub work_group_size: Option<WorkGroupSize>,
    /// Specialization constants
    pub specialization: Vec<SpecializationConstant>,
    /// Pipeline layout
    pub layout: u64,
    /// Flags
    pub flags: ComputePipelineFlags,
}

impl ComputePipelineCreateInfo {
    /// Creates new compute pipeline
    pub fn new(name: &str, shader: ComputeShaderHandle) -> Self {
        Self {
            name: String::from(name),
            shader,
            entry_point: String::from("main"),
            work_group_size: None,
            specialization: Vec::new(),
            layout: 0,
            flags: ComputePipelineFlags::DEFAULT,
        }
    }

    /// With entry point
    pub fn with_entry_point(mut self, entry: &str) -> Self {
        self.entry_point = String::from(entry);
        self
    }

    /// With work group size
    pub fn with_work_group(mut self, size: WorkGroupSize) -> Self {
        self.work_group_size = Some(size);
        self
    }

    /// With specialization constant
    pub fn with_spec_const(mut self, id: u32, value: SpecConstValue) -> Self {
        self.specialization
            .push(SpecializationConstant { id, value });
        self
    }

    /// With layout
    pub fn with_layout(mut self, layout: u64) -> Self {
        self.layout = layout;
        self
    }
}

/// Compute pipeline flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ComputePipelineFlags(pub u32);

impl ComputePipelineFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 0);
    /// Is derivative
    pub const DERIVATIVE: Self = Self(1 << 1);
    /// Dispatch base
    pub const DISPATCH_BASE: Self = Self(1 << 2);
    /// Default
    pub const DEFAULT: Self = Self::NONE;

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

/// Specialization constant
#[derive(Clone, Debug)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Value
    pub value: SpecConstValue,
}

/// Specialization constant value
#[derive(Clone, Copy, Debug)]
pub enum SpecConstValue {
    /// Bool
    Bool(bool),
    /// Int
    Int(i32),
    /// UInt
    UInt(u32),
    /// Float
    Float(f32),
}

impl SpecConstValue {
    /// To bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        match self {
            Self::Bool(v) => (if *v { 1u32 } else { 0u32 }).to_ne_bytes(),
            Self::Int(v) => v.to_ne_bytes(),
            Self::UInt(v) => v.to_ne_bytes(),
            Self::Float(v) => v.to_ne_bytes(),
        }
    }
}

// ============================================================================
// Dispatch Commands
// ============================================================================

/// Dispatch command
#[derive(Clone, Copy, Debug)]
pub struct DispatchCommand {
    /// Work groups X
    pub groups_x: u32,
    /// Work groups Y
    pub groups_y: u32,
    /// Work groups Z
    pub groups_z: u32,
}

impl DispatchCommand {
    /// Creates dispatch command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            groups_x: x,
            groups_y: y,
            groups_z: z,
        }
    }

    /// From dispatch size
    pub const fn from_size(size: DispatchSize) -> Self {
        Self::new(size.x, size.y, size.z)
    }

    /// 1D dispatch
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// 2D dispatch
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }
}

impl Default for DispatchCommand {
    fn default() -> Self {
        Self::d1(1)
    }
}

/// Indirect dispatch command (GPU-driven)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// Work groups X
    pub groups_x: u32,
    /// Work groups Y
    pub groups_y: u32,
    /// Work groups Z
    pub groups_z: u32,
}

impl DispatchIndirectCommand {
    /// Creates command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            groups_x: x,
            groups_y: y,
            groups_z: z,
        }
    }
}

// ============================================================================
// Compute Patterns
// ============================================================================

/// Reduction operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ReductionOp {
    /// Sum
    Sum     = 0,
    /// Product
    Product = 1,
    /// Min
    Min     = 2,
    /// Max
    Max     = 3,
    /// And
    And     = 4,
    /// Or
    Or      = 5,
    /// Xor
    Xor     = 6,
}

/// Scan direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ScanDirection {
    /// Prefix (exclusive)
    #[default]
    Prefix    = 0,
    /// Inclusive
    Inclusive = 1,
}

/// Parallel reduction parameters
#[derive(Clone, Debug)]
pub struct ReductionParams {
    /// Input buffer
    pub input: u64,
    /// Output buffer
    pub output: u64,
    /// Element count
    pub count: u32,
    /// Operation
    pub operation: ReductionOp,
    /// Element size (bytes)
    pub element_size: u32,
}

impl ReductionParams {
    /// Creates reduction params
    pub fn new(input: u64, output: u64, count: u32, operation: ReductionOp) -> Self {
        Self {
            input,
            output,
            count,
            operation,
            element_size: 4,
        }
    }

    /// With element size
    pub fn with_element_size(mut self, size: u32) -> Self {
        self.element_size = size;
        self
    }

    /// Calculates required dispatch passes
    pub fn dispatch_passes(&self, work_group_size: u32) -> u32 {
        let mut count = self.count;
        let mut passes = 0;
        while count > 1 {
            count = div_ceil(count, work_group_size * 2);
            passes += 1;
        }
        passes
    }
}

/// Parallel scan parameters
#[derive(Clone, Debug)]
pub struct ScanParams {
    /// Input buffer
    pub input: u64,
    /// Output buffer
    pub output: u64,
    /// Element count
    pub count: u32,
    /// Operation
    pub operation: ReductionOp,
    /// Direction
    pub direction: ScanDirection,
    /// Element size (bytes)
    pub element_size: u32,
}

impl ScanParams {
    /// Creates scan params
    pub fn new(input: u64, output: u64, count: u32) -> Self {
        Self {
            input,
            output,
            count,
            operation: ReductionOp::Sum,
            direction: ScanDirection::Prefix,
            element_size: 4,
        }
    }

    /// With operation
    pub fn with_operation(mut self, op: ReductionOp) -> Self {
        self.operation = op;
        self
    }

    /// Inclusive scan
    pub fn inclusive(mut self) -> Self {
        self.direction = ScanDirection::Inclusive;
        self
    }
}

/// Histogram parameters
#[derive(Clone, Debug)]
pub struct HistogramParams {
    /// Input buffer
    pub input: u64,
    /// Output histogram buffer
    pub output: u64,
    /// Element count
    pub count: u32,
    /// Bin count
    pub bin_count: u32,
    /// Min value
    pub min_value: f32,
    /// Max value
    pub max_value: f32,
}

impl HistogramParams {
    /// Creates histogram params
    pub fn new(input: u64, output: u64, count: u32, bin_count: u32) -> Self {
        Self {
            input,
            output,
            count,
            bin_count,
            min_value: 0.0,
            max_value: 1.0,
        }
    }

    /// With range
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }
}

/// Sort parameters
#[derive(Clone, Debug)]
pub struct SortParams {
    /// Keys buffer
    pub keys: u64,
    /// Values buffer (optional)
    pub values: Option<u64>,
    /// Element count
    pub count: u32,
    /// Key size (bytes)
    pub key_size: u32,
    /// Ascending order
    pub ascending: bool,
}

impl SortParams {
    /// Creates sort params
    pub fn new(keys: u64, count: u32) -> Self {
        Self {
            keys,
            values: None,
            count,
            key_size: 4,
            ascending: true,
        }
    }

    /// Key-value sort
    pub fn key_value(keys: u64, values: u64, count: u32) -> Self {
        Self {
            keys,
            values: Some(values),
            count,
            key_size: 4,
            ascending: true,
        }
    }

    /// Descending order
    pub fn descending(mut self) -> Self {
        self.ascending = false;
        self
    }

    /// With key size
    pub fn with_key_size(mut self, size: u32) -> Self {
        self.key_size = size;
        self
    }
}

// ============================================================================
// Image Processing Patterns
// ============================================================================

/// Image dimensions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageDimensions {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl ImageDimensions {
    /// Creates 2D dimensions
    pub const fn d2(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Creates 3D dimensions
    pub const fn d3(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// Calculates dispatch for work group
    pub const fn dispatch(&self, wg: WorkGroupSize) -> DispatchSize {
        wg.dispatch_3d(self.width, self.height, self.depth)
    }
}

/// Blit operation
#[derive(Clone, Debug)]
pub struct BlitParams {
    /// Source image
    pub src: u64,
    /// Source region
    pub src_region: ImageRegion,
    /// Destination image
    pub dst: u64,
    /// Destination region
    pub dst_region: ImageRegion,
    /// Filter
    pub filter: BlitFilter,
}

impl BlitParams {
    /// Creates blit params
    pub fn new(src: u64, dst: u64) -> Self {
        Self {
            src,
            src_region: ImageRegion::default(),
            dst,
            dst_region: ImageRegion::default(),
            filter: BlitFilter::Linear,
        }
    }

    /// With source region
    pub fn with_src_region(mut self, region: ImageRegion) -> Self {
        self.src_region = region;
        self
    }

    /// With destination region
    pub fn with_dst_region(mut self, region: ImageRegion) -> Self {
        self.dst_region = region;
        self
    }

    /// With filter
    pub fn with_filter(mut self, filter: BlitFilter) -> Self {
        self.filter = filter;
        self
    }
}

/// Image region
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageRegion {
    /// X offset
    pub x: u32,
    /// Y offset
    pub y: u32,
    /// Z offset
    pub z: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
    /// Mip level
    pub mip_level: u32,
    /// Array layer
    pub array_layer: u32,
}

impl ImageRegion {
    /// Creates 2D region
    pub fn d2(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            z: 0,
            width,
            height,
            depth: 1,
            mip_level: 0,
            array_layer: 0,
        }
    }

    /// Creates full 2D region
    pub fn full_2d(width: u32, height: u32) -> Self {
        Self::d2(0, 0, width, height)
    }

    /// With mip level
    pub fn with_mip(mut self, level: u32) -> Self {
        self.mip_level = level;
        self
    }
}

/// Blit filter
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlitFilter {
    /// Nearest neighbor
    Nearest = 0,
    /// Bilinear
    #[default]
    Linear  = 1,
    /// Cubic
    Cubic   = 2,
}

/// Convolution kernel
#[derive(Clone, Debug)]
pub struct ConvolutionKernel {
    /// Kernel data
    pub data: Vec<f32>,
    /// Kernel width
    pub width: u32,
    /// Kernel height
    pub height: u32,
    /// Divisor
    pub divisor: f32,
    /// Bias
    pub bias: f32,
}

impl ConvolutionKernel {
    /// Creates identity kernel
    pub fn identity() -> Self {
        Self {
            data: alloc::vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            width: 3,
            height: 3,
            divisor: 1.0,
            bias: 0.0,
        }
    }

    /// Creates box blur kernel
    pub fn box_blur(size: u32) -> Self {
        let count = (size * size) as usize;
        let value = 1.0 / count as f32;
        Self {
            data: alloc::vec![value; count],
            width: size,
            height: size,
            divisor: 1.0,
            bias: 0.0,
        }
    }

    /// Creates Gaussian blur kernel (3x3)
    pub fn gaussian_3x3() -> Self {
        Self {
            data: alloc::vec![1.0, 2.0, 1.0, 2.0, 4.0, 2.0, 1.0, 2.0, 1.0,],
            width: 3,
            height: 3,
            divisor: 16.0,
            bias: 0.0,
        }
    }

    /// Creates Gaussian blur kernel (5x5)
    pub fn gaussian_5x5() -> Self {
        Self {
            data: alloc::vec![
                1.0, 4.0, 6.0, 4.0, 1.0, 4.0, 16.0, 24.0, 16.0, 4.0, 6.0, 24.0, 36.0, 24.0, 6.0,
                4.0, 16.0, 24.0, 16.0, 4.0, 1.0, 4.0, 6.0, 4.0, 1.0,
            ],
            width: 5,
            height: 5,
            divisor: 256.0,
            bias: 0.0,
        }
    }

    /// Creates sharpen kernel
    pub fn sharpen() -> Self {
        Self {
            data: alloc::vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0,],
            width: 3,
            height: 3,
            divisor: 1.0,
            bias: 0.0,
        }
    }

    /// Creates edge detection kernel (Sobel X)
    pub fn sobel_x() -> Self {
        Self {
            data: alloc::vec![-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0,],
            width: 3,
            height: 3,
            divisor: 1.0,
            bias: 0.0,
        }
    }

    /// Creates edge detection kernel (Sobel Y)
    pub fn sobel_y() -> Self {
        Self {
            data: alloc::vec![-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0,],
            width: 3,
            height: 3,
            divisor: 1.0,
            bias: 0.0,
        }
    }

    /// Creates emboss kernel
    pub fn emboss() -> Self {
        Self {
            data: alloc::vec![-2.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 2.0,],
            width: 3,
            height: 3,
            divisor: 1.0,
            bias: 128.0,
        }
    }

    /// Kernel radius
    pub fn radius(&self) -> u32 {
        self.width / 2
    }
}

// ============================================================================
// Mipmap Generation
// ============================================================================

/// Mipmap generation parameters
#[derive(Clone, Debug)]
pub struct MipmapParams {
    /// Image handle
    pub image: u64,
    /// Base width
    pub width: u32,
    /// Base height
    pub height: u32,
    /// Number of mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Filter
    pub filter: MipmapFilter,
}

impl MipmapParams {
    /// Creates mipmap params
    pub fn new(image: u64, width: u32, height: u32) -> Self {
        let mip_levels = Self::calculate_mip_levels(width, height);
        Self {
            image,
            width,
            height,
            mip_levels,
            array_layers: 1,
            filter: MipmapFilter::Box,
        }
    }

    /// Calculates mip levels
    pub fn calculate_mip_levels(width: u32, height: u32) -> u32 {
        let max_dim = width.max(height);
        (32 - max_dim.leading_zeros()).max(1)
    }

    /// With filter
    pub fn with_filter(mut self, filter: MipmapFilter) -> Self {
        self.filter = filter;
        self
    }

    /// With mip count
    pub fn with_mip_count(mut self, count: u32) -> Self {
        self.mip_levels = count;
        self
    }

    /// Gets dimensions for mip level
    pub fn mip_dimensions(&self, level: u32) -> (u32, u32) {
        let w = (self.width >> level).max(1);
        let h = (self.height >> level).max(1);
        (w, h)
    }
}

/// Mipmap filter
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MipmapFilter {
    /// Box filter
    #[default]
    Box      = 0,
    /// Triangle filter
    Triangle = 1,
    /// Kaiser filter
    Kaiser   = 2,
}

// ============================================================================
// GPU Buffer Utilities
// ============================================================================

/// Clear buffer params
#[derive(Clone, Debug)]
pub struct ClearBufferParams {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Clear value (u32)
    pub value: u32,
}

impl ClearBufferParams {
    /// Creates clear params
    pub fn new(buffer: u64, value: u32) -> Self {
        Self {
            buffer,
            offset: 0,
            size: u64::MAX,
            value,
        }
    }

    /// Clear to zero
    pub fn zero(buffer: u64) -> Self {
        Self::new(buffer, 0)
    }

    /// With range
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }
}

/// Copy buffer params
#[derive(Clone, Debug)]
pub struct CopyBufferParams {
    /// Source buffer
    pub src: u64,
    /// Source offset
    pub src_offset: u64,
    /// Destination buffer
    pub dst: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Size
    pub size: u64,
}

impl CopyBufferParams {
    /// Creates copy params
    pub fn new(src: u64, dst: u64, size: u64) -> Self {
        Self {
            src,
            src_offset: 0,
            dst,
            dst_offset: 0,
            size,
        }
    }

    /// With offsets
    pub fn with_offsets(mut self, src_offset: u64, dst_offset: u64) -> Self {
        self.src_offset = src_offset;
        self.dst_offset = dst_offset;
        self
    }
}

// ============================================================================
// Compute Limits
// ============================================================================

/// Compute limits
#[derive(Clone, Copy, Debug)]
pub struct ComputeLimits {
    /// Max work group count X
    pub max_work_group_count_x: u32,
    /// Max work group count Y
    pub max_work_group_count_y: u32,
    /// Max work group count Z
    pub max_work_group_count_z: u32,
    /// Max work group size X
    pub max_work_group_size_x: u32,
    /// Max work group size Y
    pub max_work_group_size_y: u32,
    /// Max work group size Z
    pub max_work_group_size_z: u32,
    /// Max total work group invocations
    pub max_work_group_invocations: u32,
    /// Max shared memory size (bytes)
    pub max_shared_memory: u32,
    /// Subgroup size
    pub subgroup_size: u32,
}

impl ComputeLimits {
    /// Common desktop limits
    pub const DESKTOP: Self = Self {
        max_work_group_count_x: 65535,
        max_work_group_count_y: 65535,
        max_work_group_count_z: 65535,
        max_work_group_size_x: 1024,
        max_work_group_size_y: 1024,
        max_work_group_size_z: 64,
        max_work_group_invocations: 1024,
        max_shared_memory: 49152,
        subgroup_size: 32,
    };

    /// Validates work group size
    pub fn validate_work_group(&self, size: WorkGroupSize) -> bool {
        size.x <= self.max_work_group_size_x
            && size.y <= self.max_work_group_size_y
            && size.z <= self.max_work_group_size_z
            && size.total() <= self.max_work_group_invocations
    }

    /// Validates dispatch size
    pub fn validate_dispatch(&self, size: DispatchSize) -> bool {
        size.x <= self.max_work_group_count_x
            && size.y <= self.max_work_group_count_y
            && size.z <= self.max_work_group_count_z
    }
}

impl Default for ComputeLimits {
    fn default() -> Self {
        Self::DESKTOP
    }
}
