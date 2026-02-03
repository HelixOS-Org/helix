//! Material system types for PBR rendering
//!
//! This module provides material representation for physically-based rendering.

/// Material flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MaterialFlags(pub u32);

impl MaterialFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Material is double sided
    pub const DOUBLE_SIDED: Self = Self(1 << 0);
    /// Material uses alpha blending
    pub const ALPHA_BLEND: Self = Self(1 << 1);
    /// Material uses alpha testing
    pub const ALPHA_TEST: Self = Self(1 << 2);
    /// Material is emissive
    pub const EMISSIVE: Self = Self(1 << 3);
    /// Material has normal map
    pub const HAS_NORMAL_MAP: Self = Self(1 << 4);
    /// Material has metallic-roughness map
    pub const HAS_METALLIC_ROUGHNESS: Self = Self(1 << 5);
    /// Material has occlusion map
    pub const HAS_OCCLUSION: Self = Self(1 << 6);
    /// Material has emissive map
    pub const HAS_EMISSIVE_MAP: Self = Self(1 << 7);
    /// Material uses vertex colors
    pub const VERTEX_COLORS: Self = Self(1 << 8);
    /// Material is unlit
    pub const UNLIT: Self = Self(1 << 9);
    /// Material uses clear coat
    pub const CLEAR_COAT: Self = Self(1 << 10);
    /// Material uses subsurface scattering
    pub const SUBSURFACE: Self = Self(1 << 11);
    /// Material uses anisotropy
    pub const ANISOTROPIC: Self = Self(1 << 12);
    /// Material uses sheen
    pub const SHEEN: Self = Self(1 << 13);
    /// Material uses transmission
    pub const TRANSMISSION: Self = Self(1 << 14);
    /// Material uses IOR
    pub const IOR: Self = Self(1 << 15);

    /// Standard PBR material
    pub const PBR: Self = Self(
        Self::HAS_NORMAL_MAP.0 | Self::HAS_METALLIC_ROUGHNESS.0 | Self::HAS_OCCLUSION.0
    );

    /// Contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl core::ops::BitOr for MaterialFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for MaterialFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Alpha mode for materials
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum AlphaMode {
    /// Fully opaque
    #[default]
    Opaque = 0,
    /// Alpha mask (binary)
    Mask = 1,
    /// Alpha blend (transparent)
    Blend = 2,
}

/// PBR material data for GPU
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GpuMaterial {
    /// Base color factor (RGBA)
    pub base_color_factor: [f32; 4],
    /// Emissive factor (RGB) and emissive strength
    pub emissive_factor: [f32; 4],
    /// Metallic factor
    pub metallic_factor: f32,
    /// Roughness factor
    pub roughness_factor: f32,
    /// Normal scale
    pub normal_scale: f32,
    /// Occlusion strength
    pub occlusion_strength: f32,
    /// Alpha cutoff (for mask mode)
    pub alpha_cutoff: f32,
    /// Index of refraction
    pub ior: f32,
    /// Clear coat factor
    pub clear_coat_factor: f32,
    /// Clear coat roughness
    pub clear_coat_roughness: f32,
    /// Subsurface color (RGB)
    pub subsurface_color: [f32; 3],
    /// Subsurface radius
    pub subsurface_radius: f32,
    /// Sheen color (RGB)
    pub sheen_color: [f32; 3],
    /// Sheen roughness
    pub sheen_roughness: f32,
    /// Anisotropy strength
    pub anisotropy_strength: f32,
    /// Anisotropy rotation
    pub anisotropy_rotation: f32,
    /// Transmission factor
    pub transmission_factor: f32,
    /// Thickness (for transmission)
    pub thickness: f32,
    /// Attenuation color (RGB)
    pub attenuation_color: [f32; 3],
    /// Attenuation distance
    pub attenuation_distance: f32,
    /// Texture indices packed (base, normal, metallic_roughness, emissive)
    pub texture_indices: [u32; 4],
    /// Texture indices packed (occlusion, clear_coat, transmission, sheen)
    pub texture_indices2: [u32; 4],
    /// Material flags
    pub flags: MaterialFlags,
    /// Alpha mode
    pub alpha_mode: AlphaMode,
    /// Padding
    _pad: [u8; 2],
    /// UV transform (2x3 matrix for texture animation)
    pub uv_transform: [f32; 6],
    /// Padding for alignment
    _pad2: [f32; 2],
}

impl Default for GpuMaterial {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_factor: [0.0, 0.0, 0.0, 1.0],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            alpha_cutoff: 0.5,
            ior: 1.5,
            clear_coat_factor: 0.0,
            clear_coat_roughness: 0.0,
            subsurface_color: [1.0, 1.0, 1.0],
            subsurface_radius: 0.0,
            sheen_color: [0.0, 0.0, 0.0],
            sheen_roughness: 0.0,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            transmission_factor: 0.0,
            thickness: 0.0,
            attenuation_color: [1.0, 1.0, 1.0],
            attenuation_distance: f32::INFINITY,
            texture_indices: [!0; 4],
            texture_indices2: [!0; 4],
            flags: MaterialFlags::NONE,
            alpha_mode: AlphaMode::Opaque,
            _pad: [0; 2],
            uv_transform: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0], // Identity
            _pad2: [0.0; 2],
        }
    }
}

impl GpuMaterial {
    /// Size in bytes
    pub const SIZE: u32 = 192;

    /// Invalid texture index
    pub const NO_TEXTURE: u32 = !0;

    /// Creates a simple diffuse material
    pub fn diffuse(color: [f32; 4]) -> Self {
        Self {
            base_color_factor: color,
            roughness_factor: 1.0,
            ..Default::default()
        }
    }

    /// Creates a metallic material
    pub fn metallic(color: [f32; 3], metallic: f32, roughness: f32) -> Self {
        Self {
            base_color_factor: [color[0], color[1], color[2], 1.0],
            metallic_factor: metallic,
            roughness_factor: roughness,
            ..Default::default()
        }
    }

    /// Creates an emissive material
    pub fn emissive(color: [f32; 3], strength: f32) -> Self {
        Self {
            emissive_factor: [color[0], color[1], color[2], strength],
            flags: MaterialFlags::EMISSIVE,
            ..Default::default()
        }
    }

    /// Creates a transparent material
    pub fn transparent(color: [f32; 4]) -> Self {
        Self {
            base_color_factor: color,
            alpha_mode: AlphaMode::Blend,
            flags: MaterialFlags::ALPHA_BLEND,
            ..Default::default()
        }
    }

    /// With base color texture
    pub fn with_base_color_texture(mut self, index: u32) -> Self {
        self.texture_indices[0] = index;
        self
    }

    /// With normal map
    pub fn with_normal_map(mut self, index: u32) -> Self {
        self.texture_indices[1] = index;
        self.flags |= MaterialFlags::HAS_NORMAL_MAP;
        self
    }

    /// With metallic-roughness texture
    pub fn with_metallic_roughness_texture(mut self, index: u32) -> Self {
        self.texture_indices[2] = index;
        self.flags |= MaterialFlags::HAS_METALLIC_ROUGHNESS;
        self
    }

    /// With emissive texture
    pub fn with_emissive_texture(mut self, index: u32) -> Self {
        self.texture_indices[3] = index;
        self.flags |= MaterialFlags::HAS_EMISSIVE_MAP;
        self
    }

    /// With occlusion texture
    pub fn with_occlusion_texture(mut self, index: u32) -> Self {
        self.texture_indices2[0] = index;
        self.flags |= MaterialFlags::HAS_OCCLUSION;
        self
    }

    /// Has valid base color texture
    pub const fn has_base_color_texture(&self) -> bool {
        self.texture_indices[0] != Self::NO_TEXTURE
    }

    /// Has valid normal map
    pub const fn has_normal_map(&self) -> bool {
        self.texture_indices[1] != Self::NO_TEXTURE
    }

    /// Is transparent
    pub const fn is_transparent(&self) -> bool {
        !matches!(self.alpha_mode, AlphaMode::Opaque)
    }
}

/// Light types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LightType {
    /// Directional light (sun)
    #[default]
    Directional = 0,
    /// Point light
    Point = 1,
    /// Spot light
    Spot = 2,
    /// Area light (rectangle)
    AreaRect = 3,
    /// Area light (disk)
    AreaDisk = 4,
}

/// Light data for GPU
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GpuLight {
    /// Position (point/spot) or direction (directional)
    pub position_or_direction: [f32; 3],
    /// Light type
    pub light_type: LightType,
    /// Color (RGB)
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Direction (for spot lights)
    pub direction: [f32; 3],
    /// Range (point/spot, 0 = infinite)
    pub range: f32,
    /// Inner cone angle (spot, radians)
    pub inner_cone_angle: f32,
    /// Outer cone angle (spot, radians)
    pub outer_cone_angle: f32,
    /// Area size (for area lights)
    pub area_size: [f32; 2],
    /// Shadow map index (-1 = no shadow)
    pub shadow_map_index: i32,
    /// Padding
    _pad: [u32; 3],
}

impl Default for GpuLight {
    fn default() -> Self {
        Self::directional([0.0, -1.0, 0.0], [1.0, 1.0, 1.0], 1.0)
    }
}

impl GpuLight {
    /// Size in bytes
    pub const SIZE: u32 = 80;

    /// Creates a directional light
    pub const fn directional(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            position_or_direction: direction,
            light_type: LightType::Directional,
            color,
            intensity,
            direction: [0.0, 0.0, 0.0],
            range: 0.0,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [0.0, 0.0],
            shadow_map_index: -1,
            _pad: [0; 3],
        }
    }

    /// Creates a point light
    pub const fn point(position: [f32; 3], color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            position_or_direction: position,
            light_type: LightType::Point,
            color,
            intensity,
            direction: [0.0, 0.0, 0.0],
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [0.0, 0.0],
            shadow_map_index: -1,
            _pad: [0; 3],
        }
    }

    /// Creates a spot light
    pub fn spot(
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            position_or_direction: position,
            light_type: LightType::Spot,
            color,
            intensity,
            direction,
            range,
            inner_cone_angle: inner_angle,
            outer_cone_angle: outer_angle,
            area_size: [0.0, 0.0],
            shadow_map_index: -1,
            _pad: [0; 3],
        }
    }

    /// With shadow map
    pub const fn with_shadow(mut self, shadow_map_index: i32) -> Self {
        self.shadow_map_index = shadow_map_index;
        self
    }

    /// Casts shadows
    pub const fn casts_shadows(&self) -> bool {
        self.shadow_map_index >= 0
    }
}

/// Scene uniform data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SceneUniforms {
    /// Ambient color
    pub ambient_color: [f32; 4],
    /// Exposure
    pub exposure: f32,
    /// Gamma
    pub gamma: f32,
    /// Number of active lights
    pub light_count: u32,
    /// IBL intensity
    pub ibl_intensity: f32,
    /// Time (seconds)
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Frame index
    pub frame_index: u32,
    /// Fog density
    pub fog_density: f32,
    /// Fog color
    pub fog_color: [f32; 3],
    /// Fog start distance
    pub fog_start: f32,
    /// Fog end distance
    pub fog_end: f32,
    /// Padding
    _pad: [f32; 2],
}

impl Default for SceneUniforms {
    fn default() -> Self {
        Self {
            ambient_color: [0.03, 0.03, 0.03, 1.0],
            exposure: 1.0,
            gamma: 2.2,
            light_count: 0,
            ibl_intensity: 1.0,
            time: 0.0,
            delta_time: 0.016,
            frame_index: 0,
            fog_density: 0.0,
            fog_color: [0.5, 0.5, 0.5],
            fog_start: 10.0,
            fog_end: 100.0,
            _pad: [0.0; 2],
        }
    }
}

impl SceneUniforms {
    /// Size in bytes
    pub const SIZE: u32 = 80;
}

/// Shadow cascade data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ShadowCascade {
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Split depth
    pub split_depth: f32,
    /// Cascade index
    pub cascade_index: u32,
    /// Padding
    _pad: [f32; 2],
}

impl ShadowCascade {
    /// Size in bytes
    pub const SIZE: u32 = 80;
}
