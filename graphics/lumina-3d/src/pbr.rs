//! # PBR Materials
//!
//! Physically-based rendering implementation.

use alloc::string::String;
use alloc::vec::Vec;

/// PBR material
#[derive(Debug, Clone)]
pub struct PbrMaterial {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub reflectance: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,

    // Textures
    pub albedo_map: Option<u64>,
    pub normal_map: Option<u64>,
    pub metallic_roughness_map: Option<u64>,
    pub ao_map: Option<u64>,
    pub emissive_map: Option<u64>,
    pub height_map: Option<u64>,

    // Advanced
    pub clear_coat: f32,
    pub clear_coat_roughness: f32,
    pub anisotropy: f32,
    pub anisotropy_rotation: f32,
    pub subsurface: SubsurfaceParams,
    pub sheen: SheenParams,

    // Rendering
    pub blend_mode: super::BlendMode,
    pub double_sided: bool,
    pub alpha_cutoff: f32,
    pub parallax_scale: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: String::new(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            reflectance: 0.5,
            emissive: [0.0; 3],
            emissive_strength: 1.0,
            albedo_map: None,
            normal_map: None,
            metallic_roughness_map: None,
            ao_map: None,
            emissive_map: None,
            height_map: None,
            clear_coat: 0.0,
            clear_coat_roughness: 0.0,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            subsurface: SubsurfaceParams::default(),
            sheen: SheenParams::default(),
            blend_mode: super::BlendMode::Opaque,
            double_sided: false,
            alpha_cutoff: 0.5,
            parallax_scale: 0.0,
        }
    }
}

/// Subsurface scattering parameters
#[derive(Debug, Clone, Default)]
pub struct SubsurfaceParams {
    pub enabled: bool,
    pub color: [f32; 3],
    pub radius: f32,
}

/// Sheen parameters (fabric/velvet)
#[derive(Debug, Clone, Default)]
pub struct SheenParams {
    pub enabled: bool,
    pub color: [f32; 3],
    pub roughness: f32,
}

/// BRDF model
pub struct Brdf;

impl Brdf {
    /// GGX/Trowbridge-Reitz normal distribution
    pub fn d_ggx(n_dot_h: f32, roughness: f32) -> f32 {
        let a = roughness * roughness;
        let a2 = a * a;
        let n_dot_h_2 = n_dot_h * n_dot_h;

        let denom = n_dot_h_2 * (a2 - 1.0) + 1.0;
        a2 / (core::f32::consts::PI * denom * denom)
    }

    /// Smith-Schlick geometry function
    pub fn g_smith_schlick(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
        let k = (roughness + 1.0).powi(2) / 8.0;

        let g_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
        let g_l = n_dot_l / (n_dot_l * (1.0 - k) + k);

        g_v * g_l
    }

    /// Fresnel-Schlick approximation
    pub fn f_schlick(cos_theta: f32, f0: [f32; 3]) -> [f32; 3] {
        let t = 1.0 - cos_theta;
        let t5 = t * t * t * t * t;
        [
            f0[0] + (1.0 - f0[0]) * t5,
            f0[1] + (1.0 - f0[1]) * t5,
            f0[2] + (1.0 - f0[2]) * t5,
        ]
    }

    /// Fresnel-Schlick with roughness
    pub fn f_schlick_roughness(cos_theta: f32, f0: [f32; 3], roughness: f32) -> [f32; 3] {
        let t = 1.0 - cos_theta;
        let t5 = t * t * t * t * t;
        let max_f0 = 1.0 - roughness;
        [
            f0[0] + (max_f0.max(f0[0]) - f0[0]) * t5,
            f0[1] + (max_f0.max(f0[1]) - f0[1]) * t5,
            f0[2] + (max_f0.max(f0[2]) - f0[2]) * t5,
        ]
    }

    /// Calculate F0 from metallic and albedo
    pub fn calculate_f0(albedo: [f32; 3], metallic: f32) -> [f32; 3] {
        let dielectric_f0 = 0.04;
        [
            dielectric_f0 * (1.0 - metallic) + albedo[0] * metallic,
            dielectric_f0 * (1.0 - metallic) + albedo[1] * metallic,
            dielectric_f0 * (1.0 - metallic) + albedo[2] * metallic,
        ]
    }
}

/// Environment BRDF lookup table
pub struct EnvBrdfLut {
    data: Vec<[f32; 2]>,
    size: u32,
}

impl EnvBrdfLut {
    /// Generate BRDF LUT using importance sampling
    pub fn generate(size: u32) -> Self {
        let mut data = Vec::with_capacity((size * size) as usize);

        for y in 0..size {
            let roughness = (y as f32 + 0.5) / size as f32;
            for x in 0..size {
                let n_dot_v = (x as f32 + 0.5) / size as f32;

                // Integrate BRDF
                let (scale, bias) = Self::integrate_brdf(n_dot_v, roughness, 1024);
                data.push([scale, bias]);
            }
        }

        Self { data, size }
    }

    fn integrate_brdf(n_dot_v: f32, roughness: f32, sample_count: u32) -> (f32, f32) {
        let v = [(1.0 - n_dot_v * n_dot_v).sqrt(), 0.0, n_dot_v];

        let mut a = 0.0f32;
        let mut b = 0.0f32;

        let n = [0.0f32, 0.0, 1.0];

        for i in 0..sample_count {
            let xi = Self::hammersley(i, sample_count);
            let h = Self::importance_sample_ggx(xi, n, roughness);

            let l = Self::reflect_neg(v, h);

            let n_dot_l = l[2].max(0.0);
            let n_dot_h = h[2].max(0.0);
            let v_dot_h = (v[0] * h[0] + v[1] * h[1] + v[2] * h[2]).max(0.0);

            if n_dot_l > 0.0 {
                let g = Brdf::g_smith_schlick(n_dot_v, n_dot_l, roughness);
                let g_vis = (g * v_dot_h) / (n_dot_h * n_dot_v);
                let fc = (1.0 - v_dot_h).powi(5);

                a += (1.0 - fc) * g_vis;
                b += fc * g_vis;
            }
        }

        (a / sample_count as f32, b / sample_count as f32)
    }

    fn hammersley(i: u32, n: u32) -> [f32; 2] {
        let mut bits = i;
        bits = (bits << 16) | (bits >> 16);
        bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
        bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
        bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
        bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);

        [i as f32 / n as f32, bits as f32 * 2.3283064365386963e-10]
    }

    fn importance_sample_ggx(xi: [f32; 2], n: [f32; 3], roughness: f32) -> [f32; 3] {
        let a = roughness * roughness;

        let phi = 2.0 * core::f32::consts::PI * xi[0];
        let cos_theta = ((1.0 - xi[1]) / (1.0 + (a * a - 1.0) * xi[1])).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let h = [phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta];

        // Transform to world space
        let up = if n[2].abs() < 0.999 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };

        let tangent = Self::normalize(Self::cross(up, n));
        let bitangent = Self::cross(n, tangent);

        Self::normalize([
            tangent[0] * h[0] + bitangent[0] * h[1] + n[0] * h[2],
            tangent[1] * h[0] + bitangent[1] * h[1] + n[1] * h[2],
            tangent[2] * h[0] + bitangent[2] * h[1] + n[2] * h[2],
        ])
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn normalize(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 {
            [v[0] / len, v[1] / len, v[2] / len]
        } else {
            v
        }
    }

    fn reflect_neg(v: [f32; 3], n: [f32; 3]) -> [f32; 3] {
        let dot = v[0] * n[0] + v[1] * n[1] + v[2] * n[2];
        [
            2.0 * dot * n[0] - v[0],
            2.0 * dot * n[1] - v[1],
            2.0 * dot * n[2] - v[2],
        ]
    }

    /// Sample the LUT
    pub fn sample(&self, n_dot_v: f32, roughness: f32) -> [f32; 2] {
        let x = ((n_dot_v * self.size as f32) as usize).min(self.size as usize - 1);
        let y = ((roughness * self.size as f32) as usize).min(self.size as usize - 1);
        self.data[y * self.size as usize + x]
    }
}

/// Material uniform buffer
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
    pub emissive: [f32; 4],
    pub metallic_roughness_ao_reflectance: [f32; 4],
    pub clear_coat_anisotropy: [f32; 4],
    pub sheen_color_roughness: [f32; 4],
    pub subsurface_color_radius: [f32; 4],
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub parallax_scale: f32,
    pub _pad: f32,
}

impl From<&PbrMaterial> for MaterialUniform {
    fn from(mat: &PbrMaterial) -> Self {
        let mut flags = 0u32;
        if mat.albedo_map.is_some() {
            flags |= 1 << 0;
        }
        if mat.normal_map.is_some() {
            flags |= 1 << 1;
        }
        if mat.metallic_roughness_map.is_some() {
            flags |= 1 << 2;
        }
        if mat.ao_map.is_some() {
            flags |= 1 << 3;
        }
        if mat.emissive_map.is_some() {
            flags |= 1 << 4;
        }
        if mat.height_map.is_some() {
            flags |= 1 << 5;
        }
        if mat.double_sided {
            flags |= 1 << 6;
        }
        if mat.clear_coat > 0.0 {
            flags |= 1 << 7;
        }
        if mat.anisotropy != 0.0 {
            flags |= 1 << 8;
        }
        if mat.sheen.enabled {
            flags |= 1 << 9;
        }
        if mat.subsurface.enabled {
            flags |= 1 << 10;
        }

        Self {
            base_color: mat.base_color,
            emissive: [
                mat.emissive[0] * mat.emissive_strength,
                mat.emissive[1] * mat.emissive_strength,
                mat.emissive[2] * mat.emissive_strength,
                0.0,
            ],
            metallic_roughness_ao_reflectance: [mat.metallic, mat.roughness, 1.0, mat.reflectance],
            clear_coat_anisotropy: [
                mat.clear_coat,
                mat.clear_coat_roughness,
                mat.anisotropy,
                mat.anisotropy_rotation,
            ],
            sheen_color_roughness: [
                mat.sheen.color[0],
                mat.sheen.color[1],
                mat.sheen.color[2],
                mat.sheen.roughness,
            ],
            subsurface_color_radius: [
                mat.subsurface.color[0],
                mat.subsurface.color[1],
                mat.subsurface.color[2],
                mat.subsurface.radius,
            ],
            flags,
            alpha_cutoff: mat.alpha_cutoff,
            parallax_scale: mat.parallax_scale,
            _pad: 0.0,
        }
    }
}
