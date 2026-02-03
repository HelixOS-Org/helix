//! Specialization Constants
//!
//! This module provides specialization constant support for shader customization
//! at pipeline creation time.

use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

// ============================================================================
// Specialization Entry
// ============================================================================

/// Specialization constant type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecializationType {
    /// Boolean (4 bytes, non-zero = true).
    Bool,
    /// 32-bit signed integer.
    Int32,
    /// 32-bit unsigned integer.
    Uint32,
    /// 32-bit float.
    Float32,
    /// 64-bit signed integer.
    Int64,
    /// 64-bit unsigned integer.
    Uint64,
    /// 64-bit float.
    Float64,
}

impl SpecializationType {
    /// Get the size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Bool | Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
        }
    }
}

/// Specialization constant value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecializationValue {
    /// Boolean value.
    Bool(bool),
    /// 32-bit signed integer.
    Int32(i32),
    /// 32-bit unsigned integer.
    Uint32(u32),
    /// 32-bit float.
    Float32(f32),
    /// 64-bit signed integer.
    Int64(i64),
    /// 64-bit unsigned integer.
    Uint64(u64),
    /// 64-bit float.
    Float64(f64),
}

impl SpecializationValue {
    /// Get the type.
    pub fn value_type(&self) -> SpecializationType {
        match self {
            Self::Bool(_) => SpecializationType::Bool,
            Self::Int32(_) => SpecializationType::Int32,
            Self::Uint32(_) => SpecializationType::Uint32,
            Self::Float32(_) => SpecializationType::Float32,
            Self::Int64(_) => SpecializationType::Int64,
            Self::Uint64(_) => SpecializationType::Uint64,
            Self::Float64(_) => SpecializationType::Float64,
        }
    }

    /// Get the size in bytes.
    pub fn size(&self) -> usize {
        self.value_type().size()
    }

    /// Convert to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Bool(v) => (if *v { 1u32 } else { 0u32 }).to_le_bytes().to_vec(),
            Self::Int32(v) => v.to_le_bytes().to_vec(),
            Self::Uint32(v) => v.to_le_bytes().to_vec(),
            Self::Float32(v) => v.to_le_bytes().to_vec(),
            Self::Int64(v) => v.to_le_bytes().to_vec(),
            Self::Uint64(v) => v.to_le_bytes().to_vec(),
            Self::Float64(v) => v.to_le_bytes().to_vec(),
        }
    }
}

/// Specialization constant entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecializationEntry {
    /// Constant ID (matches SPIR-V SpecId).
    pub constant_id: u32,
    /// Offset in the data buffer.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
    /// Value.
    pub value: SpecializationValue,
}

impl SpecializationEntry {
    /// Create a new entry.
    pub fn new(constant_id: u32, value: SpecializationValue) -> Self {
        Self {
            constant_id,
            offset: 0, // Will be set when building
            size: value.size() as u32,
            value,
        }
    }

    /// Create a boolean entry.
    pub fn bool(constant_id: u32, value: bool) -> Self {
        Self::new(constant_id, SpecializationValue::Bool(value))
    }

    /// Create an i32 entry.
    pub fn int32(constant_id: u32, value: i32) -> Self {
        Self::new(constant_id, SpecializationValue::Int32(value))
    }

    /// Create a u32 entry.
    pub fn uint32(constant_id: u32, value: u32) -> Self {
        Self::new(constant_id, SpecializationValue::Uint32(value))
    }

    /// Create an f32 entry.
    pub fn float32(constant_id: u32, value: f32) -> Self {
        Self::new(constant_id, SpecializationValue::Float32(value))
    }

    /// Create an i64 entry.
    pub fn int64(constant_id: u32, value: i64) -> Self {
        Self::new(constant_id, SpecializationValue::Int64(value))
    }

    /// Create a u64 entry.
    pub fn uint64(constant_id: u32, value: u64) -> Self {
        Self::new(constant_id, SpecializationValue::Uint64(value))
    }

    /// Create an f64 entry.
    pub fn float64(constant_id: u32, value: f64) -> Self {
        Self::new(constant_id, SpecializationValue::Float64(value))
    }
}

// ============================================================================
// Specialization Constants
// ============================================================================

/// Collection of specialization constants.
#[derive(Debug, Clone)]
pub struct SpecializationConstants {
    /// Entries.
    entries: Vec<SpecializationEntry>,
    /// Packed data.
    data: Vec<u8>,
    /// Hash for caching.
    hash: u64,
}

impl SpecializationConstants {
    /// Create empty specialization constants.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            data: Vec::new(),
            hash: 0,
        }
    }

    /// Create from entries.
    pub fn from_entries(entries: Vec<SpecializationEntry>) -> Self {
        let mut constants = Self {
            entries,
            data: Vec::new(),
            hash: 0,
        };
        constants.pack_data();
        constants.compute_hash();
        constants
    }

    /// Add an entry.
    pub fn add(&mut self, entry: SpecializationEntry) {
        self.entries.push(entry);
        self.pack_data();
        self.compute_hash();
    }

    /// Add a boolean constant.
    pub fn add_bool(&mut self, constant_id: u32, value: bool) {
        self.add(SpecializationEntry::bool(constant_id, value));
    }

    /// Add an i32 constant.
    pub fn add_int32(&mut self, constant_id: u32, value: i32) {
        self.add(SpecializationEntry::int32(constant_id, value));
    }

    /// Add a u32 constant.
    pub fn add_uint32(&mut self, constant_id: u32, value: u32) {
        self.add(SpecializationEntry::uint32(constant_id, value));
    }

    /// Add an f32 constant.
    pub fn add_float32(&mut self, constant_id: u32, value: f32) {
        self.add(SpecializationEntry::float32(constant_id, value));
    }

    /// Get entries.
    pub fn entries(&self) -> &[SpecializationEntry] {
        &self.entries
    }

    /// Get packed data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get entry by constant ID.
    pub fn get(&self, constant_id: u32) -> Option<&SpecializationEntry> {
        self.entries.iter().find(|e| e.constant_id == constant_id)
    }

    /// Update a constant value.
    pub fn set(&mut self, constant_id: u32, value: SpecializationValue) -> bool {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.constant_id == constant_id)
        {
            entry.value = value;
            entry.size = value.size() as u32;
            self.pack_data();
            self.compute_hash();
            true
        } else {
            false
        }
    }

    /// Pack data from entries.
    fn pack_data(&mut self) {
        self.data.clear();
        let mut offset = 0u32;

        for entry in &mut self.entries {
            entry.offset = offset;
            self.data.extend_from_slice(&entry.value.to_bytes());
            offset += entry.size;
        }
    }

    /// Compute hash.
    fn compute_hash(&mut self) {
        let mut hasher = FnvHasher::new();
        for entry in &self.entries {
            entry.constant_id.hash(&mut hasher);
            entry.size.hash(&mut hasher);
        }
        self.data.hash(&mut hasher);
        self.hash = hasher.finish();
    }
}

impl Default for SpecializationConstants {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Specialization Builder
// ============================================================================

/// Builder for specialization constants.
pub struct SpecializationBuilder {
    entries: Vec<SpecializationEntry>,
}

impl SpecializationBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a boolean constant.
    pub fn bool(mut self, constant_id: u32, value: bool) -> Self {
        self.entries
            .push(SpecializationEntry::bool(constant_id, value));
        self
    }

    /// Add an i32 constant.
    pub fn int32(mut self, constant_id: u32, value: i32) -> Self {
        self.entries
            .push(SpecializationEntry::int32(constant_id, value));
        self
    }

    /// Add a u32 constant.
    pub fn uint32(mut self, constant_id: u32, value: u32) -> Self {
        self.entries
            .push(SpecializationEntry::uint32(constant_id, value));
        self
    }

    /// Add an f32 constant.
    pub fn float32(mut self, constant_id: u32, value: f32) -> Self {
        self.entries
            .push(SpecializationEntry::float32(constant_id, value));
        self
    }

    /// Add an i64 constant.
    pub fn int64(mut self, constant_id: u32, value: i64) -> Self {
        self.entries
            .push(SpecializationEntry::int64(constant_id, value));
        self
    }

    /// Add a u64 constant.
    pub fn uint64(mut self, constant_id: u32, value: u64) -> Self {
        self.entries
            .push(SpecializationEntry::uint64(constant_id, value));
        self
    }

    /// Add an f64 constant.
    pub fn float64(mut self, constant_id: u32, value: f64) -> Self {
        self.entries
            .push(SpecializationEntry::float64(constant_id, value));
        self
    }

    /// Build specialization constants.
    pub fn build(self) -> SpecializationConstants {
        SpecializationConstants::from_entries(self.entries)
    }
}

impl Default for SpecializationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Common Specializations
// ============================================================================

/// Common specialization constant IDs.
pub struct SpecConstantIds;

impl SpecConstantIds {
    /// Local workgroup size X.
    pub const LOCAL_SIZE_X: u32 = 0;
    /// Local workgroup size Y.
    pub const LOCAL_SIZE_Y: u32 = 1;
    /// Local workgroup size Z.
    pub const LOCAL_SIZE_Z: u32 = 2;
    /// Max lights.
    pub const MAX_LIGHTS: u32 = 10;
    /// Shadow cascade count.
    pub const SHADOW_CASCADE_COUNT: u32 = 11;
    /// Enable normal mapping.
    pub const ENABLE_NORMAL_MAP: u32 = 20;
    /// Enable PBR.
    pub const ENABLE_PBR: u32 = 21;
    /// Enable IBL.
    pub const ENABLE_IBL: u32 = 22;
    /// Enable shadows.
    pub const ENABLE_SHADOWS: u32 = 23;
    /// Alpha mode.
    pub const ALPHA_MODE: u32 = 30;
    /// Vertex has tangents.
    pub const HAS_TANGENTS: u32 = 40;
    /// Vertex has colors.
    pub const HAS_VERTEX_COLOR: u32 = 41;
}

/// Create common material specializations.
pub fn material_specializations(
    normal_map: bool,
    pbr: bool,
    ibl: bool,
    shadows: bool,
) -> SpecializationConstants {
    SpecializationBuilder::new()
        .bool(SpecConstantIds::ENABLE_NORMAL_MAP, normal_map)
        .bool(SpecConstantIds::ENABLE_PBR, pbr)
        .bool(SpecConstantIds::ENABLE_IBL, ibl)
        .bool(SpecConstantIds::ENABLE_SHADOWS, shadows)
        .build()
}

/// Create workgroup size specializations.
pub fn workgroup_specializations(x: u32, y: u32, z: u32) -> SpecializationConstants {
    SpecializationBuilder::new()
        .uint32(SpecConstantIds::LOCAL_SIZE_X, x)
        .uint32(SpecConstantIds::LOCAL_SIZE_Y, y)
        .uint32(SpecConstantIds::LOCAL_SIZE_Z, z)
        .build()
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
