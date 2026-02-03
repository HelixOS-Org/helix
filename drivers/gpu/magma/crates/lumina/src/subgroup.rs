//! Subgroup operations types
//!
//! This module provides types for GPU subgroup/wave/warp operations.

/// Subgroup size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubgroupSize(pub u32);

impl SubgroupSize {
    /// Minimum subgroup size (4)
    pub const MIN: Self = Self(4);
    /// Common sizes
    pub const SIZE_8: Self = Self(8);
    pub const SIZE_16: Self = Self(16);
    pub const SIZE_32: Self = Self(32);
    pub const SIZE_64: Self = Self(64);
    pub const SIZE_128: Self = Self(128);

    /// Creates a new subgroup size
    pub const fn new(size: u32) -> Self {
        Self(size)
    }

    /// Gets the size value
    pub const fn size(&self) -> u32 {
        self.0
    }

    /// Checks if power of two
    pub const fn is_power_of_two(&self) -> bool {
        self.0.is_power_of_two()
    }
}

/// Subgroup feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubgroupFeatures(pub u32);

impl SubgroupFeatures {
    /// Basic subgroup operations
    pub const BASIC: Self = Self(1 << 0);
    /// Vote operations
    pub const VOTE: Self = Self(1 << 1);
    /// Arithmetic operations
    pub const ARITHMETIC: Self = Self(1 << 2);
    /// Ballot operations
    pub const BALLOT: Self = Self(1 << 3);
    /// Shuffle operations
    pub const SHUFFLE: Self = Self(1 << 4);
    /// Shuffle relative operations
    pub const SHUFFLE_RELATIVE: Self = Self(1 << 5);
    /// Clustered operations
    pub const CLUSTERED: Self = Self(1 << 6);
    /// Quad operations
    pub const QUAD: Self = Self(1 << 7);
    /// Partitioned operations (NV)
    pub const PARTITIONED: Self = Self(1 << 8);

    /// All features
    pub const ALL: Self = Self(0x1FF);

    /// Checks if contains feature
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Combines features
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if supports basic
    pub const fn supports_basic(&self) -> bool {
        self.contains(Self::BASIC)
    }

    /// Checks if supports vote
    pub const fn supports_vote(&self) -> bool {
        self.contains(Self::VOTE)
    }

    /// Checks if supports arithmetic
    pub const fn supports_arithmetic(&self) -> bool {
        self.contains(Self::ARITHMETIC)
    }

    /// Checks if supports ballot
    pub const fn supports_ballot(&self) -> bool {
        self.contains(Self::BALLOT)
    }

    /// Checks if supports shuffle
    pub const fn supports_shuffle(&self) -> bool {
        self.contains(Self::SHUFFLE)
    }

    /// Checks if supports quad
    pub const fn supports_quad(&self) -> bool {
        self.contains(Self::QUAD)
    }
}

impl core::ops::BitOr for SubgroupFeatures {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Subgroup properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SubgroupProperties {
    /// Subgroup size
    pub subgroup_size: u32,
    /// Supported stages
    pub supported_stages: SubgroupStageFlags,
    /// Supported operations
    pub supported_operations: SubgroupFeatures,
    /// Quad operations in all stages
    pub quad_operations_in_all_stages: bool,
}

impl SubgroupProperties {
    /// NVIDIA GPU defaults
    pub const fn nvidia() -> Self {
        Self {
            subgroup_size: 32,
            supported_stages: SubgroupStageFlags::ALL,
            supported_operations: SubgroupFeatures::ALL,
            quad_operations_in_all_stages: true,
        }
    }

    /// AMD GPU defaults
    pub const fn amd() -> Self {
        Self {
            subgroup_size: 64,
            supported_stages: SubgroupStageFlags::ALL,
            supported_operations: SubgroupFeatures::ALL,
            quad_operations_in_all_stages: true,
        }
    }

    /// Intel GPU defaults
    pub const fn intel() -> Self {
        Self {
            subgroup_size: 8,
            supported_stages: SubgroupStageFlags::ALL,
            supported_operations: SubgroupFeatures::ALL,
            quad_operations_in_all_stages: false,
        }
    }
}

/// Subgroup stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubgroupStageFlags(pub u32);

impl SubgroupStageFlags {
    /// Vertex shader
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 2);
    /// Geometry shader
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Fragment shader
    pub const FRAGMENT: Self = Self(1 << 4);
    /// Compute shader
    pub const COMPUTE: Self = Self(1 << 5);
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All stages
    pub const ALL: Self = Self(0x3F);

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Subgroup ballot
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct SubgroupBallot {
    /// Ballot bits (4 x u32 = 128 lanes max)
    pub bits: [u32; 4],
}

impl SubgroupBallot {
    /// Empty ballot
    pub const EMPTY: Self = Self { bits: [0; 4] };

    /// Full ballot (all active)
    pub const fn full(subgroup_size: u32) -> Self {
        let mut bits = [0u32; 4];
        let mut remaining = subgroup_size;
        let mut i = 0;
        while remaining > 0 && i < 4 {
            if remaining >= 32 {
                bits[i] = 0xFFFFFFFF;
                remaining -= 32;
            } else {
                bits[i] = (1u32 << remaining) - 1;
                remaining = 0;
            }
            i += 1;
        }
        Self { bits }
    }

    /// Checks if lane is active
    pub const fn is_active(&self, lane: u32) -> bool {
        let word = (lane / 32) as usize;
        let bit = lane % 32;
        if word < 4 {
            (self.bits[word] & (1 << bit)) != 0
        } else {
            false
        }
    }

    /// Sets lane active
    pub fn set_active(&mut self, lane: u32, active: bool) {
        let word = (lane / 32) as usize;
        let bit = lane % 32;
        if word < 4 {
            if active {
                self.bits[word] |= 1 << bit;
            } else {
                self.bits[word] &= !(1 << bit);
            }
        }
    }

    /// Count of active lanes
    pub const fn count_active(&self) -> u32 {
        self.bits[0].count_ones()
            + self.bits[1].count_ones()
            + self.bits[2].count_ones()
            + self.bits[3].count_ones()
    }

    /// Finds first active lane
    pub const fn find_first_active(&self) -> Option<u32> {
        let mut i = 0;
        while i < 4 {
            if self.bits[i] != 0 {
                return Some(i as u32 * 32 + self.bits[i].trailing_zeros());
            }
            i += 1;
        }
        None
    }

    /// Finds last active lane
    pub const fn find_last_active(&self) -> Option<u32> {
        let mut i = 3i32;
        while i >= 0 {
            if self.bits[i as usize] != 0 {
                return Some(i as u32 * 32 + 31 - self.bits[i as usize].leading_zeros());
            }
            i -= 1;
        }
        None
    }

    /// AND operation
    pub const fn and(self, other: Self) -> Self {
        Self {
            bits: [
                self.bits[0] & other.bits[0],
                self.bits[1] & other.bits[1],
                self.bits[2] & other.bits[2],
                self.bits[3] & other.bits[3],
            ],
        }
    }

    /// OR operation
    pub const fn or(self, other: Self) -> Self {
        Self {
            bits: [
                self.bits[0] | other.bits[0],
                self.bits[1] | other.bits[1],
                self.bits[2] | other.bits[2],
                self.bits[3] | other.bits[3],
            ],
        }
    }

    /// XOR operation
    pub const fn xor(self, other: Self) -> Self {
        Self {
            bits: [
                self.bits[0] ^ other.bits[0],
                self.bits[1] ^ other.bits[1],
                self.bits[2] ^ other.bits[2],
                self.bits[3] ^ other.bits[3],
            ],
        }
    }

    /// NOT operation
    pub const fn not(self, subgroup_size: u32) -> Self {
        let mask = Self::full(subgroup_size);
        Self {
            bits: [
                (!self.bits[0]) & mask.bits[0],
                (!self.bits[1]) & mask.bits[1],
                (!self.bits[2]) & mask.bits[2],
                (!self.bits[3]) & mask.bits[3],
            ],
        }
    }
}

/// Subgroup vote result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum VoteResult {
    /// All voted true
    AllTrue,
    /// All voted false
    AllFalse,
    /// Mixed votes
    Mixed,
}

/// Subgroup operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SubgroupOp {
    /// Add
    Add,
    /// Multiply
    Mul,
    /// Minimum
    Min,
    /// Maximum
    Max,
    /// Bitwise AND
    And,
    /// Bitwise OR
    Or,
    /// Bitwise XOR
    Xor,
}

/// Shuffle operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShuffleOp {
    /// Source value lane index
    pub src_lane: u32,
    /// Width of shuffle (power of 2)
    pub width: u32,
}

impl ShuffleOp {
    /// Creates a shuffle operation
    pub const fn new(src_lane: u32, width: u32) -> Self {
        Self { src_lane, width }
    }

    /// Shuffle within quad (width=4)
    pub const fn quad(src_lane: u32) -> Self {
        Self { src_lane, width: 4 }
    }
}

/// Shuffle XOR operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShuffleXorOp {
    /// XOR mask
    pub mask: u32,
}

impl ShuffleXorOp {
    /// Creates a shuffle XOR operation
    pub const fn new(mask: u32) -> Self {
        Self { mask }
    }

    /// Butterfly shuffle (for reduction)
    pub const fn butterfly(step: u32) -> Self {
        Self { mask: 1 << step }
    }
}

/// Shuffle up/down operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShuffleRelativeOp {
    /// Delta offset
    pub delta: u32,
    /// Width
    pub width: u32,
}

impl ShuffleRelativeOp {
    /// Shuffle up (to higher lanes)
    pub const fn up(delta: u32, width: u32) -> Self {
        Self { delta, width }
    }

    /// Shuffle down (to lower lanes)
    pub const fn down(delta: u32, width: u32) -> Self {
        Self { delta, width }
    }
}

/// Clustered operation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClusteredOp {
    /// Operation type
    pub op: SubgroupOp,
    /// Cluster size (power of 2, <= subgroup_size)
    pub cluster_size: u32,
}

impl ClusteredOp {
    /// Creates a clustered operation
    pub const fn new(op: SubgroupOp, cluster_size: u32) -> Self {
        Self { op, cluster_size }
    }

    /// Add within cluster
    pub const fn add(cluster_size: u32) -> Self {
        Self::new(SubgroupOp::Add, cluster_size)
    }

    /// Max within cluster
    pub const fn max(cluster_size: u32) -> Self {
        Self::new(SubgroupOp::Max, cluster_size)
    }

    /// Min within cluster
    pub const fn min(cluster_size: u32) -> Self {
        Self::new(SubgroupOp::Min, cluster_size)
    }
}

/// Quad operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuadOp {
    /// Swap diagonal (0<->3, 1<->2)
    SwapDiagonal,
    /// Swap horizontal (0<->1, 2<->3)
    SwapHorizontal,
    /// Swap vertical (0<->2, 1<->3)
    SwapVertical,
    /// Broadcast from lane
    Broadcast,
}

/// Quad direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuadDirection {
    /// Right (+X)
    Right,
    /// Left (-X)
    Left,
    /// Down (+Y)
    Down,
    /// Up (-Y)
    Up,
}

/// Partition info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubgroupPartition {
    /// Partition ballot
    pub ballot: SubgroupBallot,
    /// Partition index
    pub index: u32,
    /// Number of partitions
    pub count: u32,
}

impl SubgroupPartition {
    /// Creates a new partition
    pub const fn new(ballot: SubgroupBallot, index: u32, count: u32) -> Self {
        Self { ballot, index, count }
    }
}

/// Subgroup size control
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubgroupSizeControl {
    /// Minimum subgroup size
    pub min_subgroup_size: u32,
    /// Maximum subgroup size
    pub max_subgroup_size: u32,
    /// Required subgroup size (0 = any)
    pub required_subgroup_size: u32,
    /// Allow varying subgroup size
    pub varying_size: bool,
}

impl Default for SubgroupSizeControl {
    fn default() -> Self {
        Self {
            min_subgroup_size: 1,
            max_subgroup_size: 128,
            required_subgroup_size: 0,
            varying_size: true,
        }
    }
}

impl SubgroupSizeControl {
    /// Requires specific size
    pub const fn require_size(size: u32) -> Self {
        Self {
            min_subgroup_size: size,
            max_subgroup_size: size,
            required_subgroup_size: size,
            varying_size: false,
        }
    }

    /// NVIDIA optimal (32)
    pub const fn nvidia_optimal() -> Self {
        Self::require_size(32)
    }

    /// AMD optimal (64)
    pub const fn amd_optimal() -> Self {
        Self::require_size(64)
    }

    /// Allows full range
    pub const fn full_range() -> Self {
        Self {
            min_subgroup_size: 1,
            max_subgroup_size: 128,
            required_subgroup_size: 0,
            varying_size: true,
        }
    }
}

/// Subgroup uniform value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubgroupUniform<T> {
    /// The uniform value
    pub value: T,
    /// Whether all lanes have same value
    pub is_uniform: bool,
}

impl<T: Copy> SubgroupUniform<T> {
    /// Creates a uniform value
    pub const fn uniform(value: T) -> Self {
        Self {
            value,
            is_uniform: true,
        }
    }

    /// Creates a non-uniform value
    pub const fn non_uniform(value: T) -> Self {
        Self {
            value,
            is_uniform: false,
        }
    }
}

/// Broadcast info for subgroup broadcast
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BroadcastInfo {
    /// Source lane index
    pub src_lane: u32,
}

impl BroadcastInfo {
    /// Broadcast from first lane
    pub const FIRST: Self = Self { src_lane: 0 };

    /// Creates a broadcast from specific lane
    pub const fn from_lane(lane: u32) -> Self {
        Self { src_lane: lane }
    }
}

/// Reduction scope
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ReductionScope {
    /// Subgroup scope
    Subgroup,
    /// Workgroup scope
    Workgroup,
    /// Device scope
    Device,
    /// Queue family scope
    QueueFamily,
}
