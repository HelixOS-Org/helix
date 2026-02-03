//! # Post-Processing Effects
//!
//! Complete post-processing pipeline.

use alloc::string::String;
use alloc::vec::Vec;

/// Post-processing pipeline
pub struct PostFxPipeline {
    effects: Vec<PostEffect>,
    enabled: bool,
}

impl PostFxPipeline {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            enabled: true,
        }
    }

    /// Add an effect
    pub fn add(&mut self, effect: PostEffect) {
        self.effects.push(effect);
    }

    /// Remove effect by name
    pub fn remove(&mut self, name: &str) {
        self.effects.retain(|e| e.name() != name);
    }

    /// Get effect by name
    pub fn get(&self, name: &str) -> Option<&PostEffect> {
        self.effects.iter().find(|e| e.name() == name)
    }

    /// Get mutable effect
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PostEffect> {
        self.effects.iter_mut().find(|e| e.name() == name)
    }

    /// Enable/disable pipeline
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for PostFxPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Post-processing effect
pub enum PostEffect {
    Tonemapping(Tonemapping),
    Bloom(Bloom),
    ColorGrading(ColorGrading),
    Vignette(Vignette),
    ChromaticAberration(ChromaticAberration),
    FilmGrain(FilmGrain),
    MotionBlur(MotionBlur),
    DepthOfField(DepthOfField),
    Fxaa(Fxaa),
    Taa(Taa),
    Sharpening(Sharpening),
    Fog(Fog),
    LensFlare(LensFlare),
    ScreenSpaceReflections(Ssr),
}

impl PostEffect {
    pub fn name(&self) -> &str {
        match self {
            PostEffect::Tonemapping(_) => "tonemapping",
            PostEffect::Bloom(_) => "bloom",
            PostEffect::ColorGrading(_) => "color_grading",
            PostEffect::Vignette(_) => "vignette",
            PostEffect::ChromaticAberration(_) => "chromatic_aberration",
            PostEffect::FilmGrain(_) => "film_grain",
            PostEffect::MotionBlur(_) => "motion_blur",
            PostEffect::DepthOfField(_) => "depth_of_field",
            PostEffect::Fxaa(_) => "fxaa",
            PostEffect::Taa(_) => "taa",
            PostEffect::Sharpening(_) => "sharpening",
            PostEffect::Fog(_) => "fog",
            PostEffect::LensFlare(_) => "lens_flare",
            PostEffect::ScreenSpaceReflections(_) => "ssr",
        }
    }
}

/// Tonemapping
#[derive(Debug, Clone)]
pub struct Tonemapping {
    pub operator: TonemapOperator,
    pub exposure: f32,
    pub gamma: f32,
    pub white_point: f32,
}

impl Default for Tonemapping {
    fn default() -> Self {
        Self {
            operator: TonemapOperator::AcesFilmic,
            exposure: 1.0,
            gamma: 2.2,
            white_point: 11.2,
        }
    }
}

/// Tonemap operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TonemapOperator {
    Linear,
    Reinhard,
    ReinhardExtended,
    AcesFilmic,
    AgxDefault,
    AgxPunchy,
    Neutral,
    Uncharted2,
}

impl TonemapOperator {
    pub fn apply(&self, color: [f32; 3], exposure: f32, white_point: f32) -> [f32; 3] {
        let c = [
            color[0] * exposure,
            color[1] * exposure,
            color[2] * exposure,
        ];

        match self {
            TonemapOperator::Linear => c,
            TonemapOperator::Reinhard => [
                c[0] / (c[0] + 1.0),
                c[1] / (c[1] + 1.0),
                c[2] / (c[2] + 1.0),
            ],
            TonemapOperator::ReinhardExtended => {
                let w2 = white_point * white_point;
                [
                    c[0] * (1.0 + c[0] / w2) / (1.0 + c[0]),
                    c[1] * (1.0 + c[1] / w2) / (1.0 + c[1]),
                    c[2] * (1.0 + c[2] / w2) / (1.0 + c[2]),
                ]
            },
            TonemapOperator::AcesFilmic => Self::aces_filmic(c),
            _ => c,
        }
    }

    fn aces_filmic(x: [f32; 3]) -> [f32; 3] {
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;

        [
            ((x[0] * (a * x[0] + b)) / (x[0] * (c * x[0] + d) + e)).clamp(0.0, 1.0),
            ((x[1] * (a * x[1] + b)) / (x[1] * (c * x[1] + d) + e)).clamp(0.0, 1.0),
            ((x[2] * (a * x[2] + b)) / (x[2] * (c * x[2] + d) + e)).clamp(0.0, 1.0),
        ]
    }
}

/// Bloom effect
#[derive(Debug, Clone)]
pub struct Bloom {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub soft_threshold: f32,
    pub radius: f32,
    pub mip_count: u32,
    pub dirt_intensity: f32,
    pub dirt_texture: Option<u64>,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 0.8,
            threshold: 1.0,
            soft_threshold: 0.5,
            radius: 4.0,
            mip_count: 5,
            dirt_intensity: 0.0,
            dirt_texture: None,
        }
    }
}

/// Color grading
#[derive(Debug, Clone)]
pub struct ColorGrading {
    pub enabled: bool,
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub lift: [f32; 4],
    pub gamma: [f32; 4],
    pub gain: [f32; 4],
    pub hue_shift: f32,
    pub lut: Option<u64>,
    pub lut_contribution: f32,
}

impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            enabled: true,
            temperature: 0.0,
            tint: 0.0,
            saturation: 1.0,
            contrast: 1.0,
            lift: [0.0, 0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0, 1.0],
            gain: [1.0, 1.0, 1.0, 1.0],
            hue_shift: 0.0,
            lut: None,
            lut_contribution: 1.0,
        }
    }
}

/// Vignette
#[derive(Debug, Clone)]
pub struct Vignette {
    pub enabled: bool,
    pub intensity: f32,
    pub smoothness: f32,
    pub roundness: f32,
    pub color: [f32; 3],
    pub center: [f32; 2],
}

impl Default for Vignette {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            smoothness: 0.3,
            roundness: 1.0,
            color: [0.0, 0.0, 0.0],
            center: [0.5, 0.5],
        }
    }
}

/// Chromatic aberration
#[derive(Debug, Clone)]
pub struct ChromaticAberration {
    pub enabled: bool,
    pub intensity: f32,
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.1,
        }
    }
}

/// Film grain
#[derive(Debug, Clone)]
pub struct FilmGrain {
    pub enabled: bool,
    pub intensity: f32,
    pub response: f32,
    pub colored: bool,
}

impl Default for FilmGrain {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.3,
            response: 0.8,
            colored: false,
        }
    }
}

/// Motion blur
#[derive(Debug, Clone)]
pub struct MotionBlur {
    pub enabled: bool,
    pub intensity: f32,
    pub sample_count: u32,
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            sample_count: 8,
        }
    }
}

/// Depth of field
#[derive(Debug, Clone)]
pub struct DepthOfField {
    pub enabled: bool,
    pub focus_distance: f32,
    pub focus_range: f32,
    pub bokeh_shape: BokehShape,
    pub aperture: f32,
    pub max_blur: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 10.0,
            focus_range: 5.0,
            bokeh_shape: BokehShape::Circle,
            aperture: 5.6,
            max_blur: 4.0,
        }
    }
}

/// Bokeh shape
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BokehShape {
    Circle,
    Hexagon,
    Octagon,
}

/// FXAA
#[derive(Debug, Clone)]
pub struct Fxaa {
    pub enabled: bool,
    pub quality: FxaaQuality,
}

impl Default for Fxaa {
    fn default() -> Self {
        Self {
            enabled: true,
            quality: FxaaQuality::High,
        }
    }
}

/// FXAA quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxaaQuality {
    Low,
    Medium,
    High,
    Ultra,
}

/// TAA
#[derive(Debug, Clone)]
pub struct Taa {
    pub enabled: bool,
    pub jitter_scale: f32,
    pub feedback: f32,
    pub sharpness: f32,
}

impl Default for Taa {
    fn default() -> Self {
        Self {
            enabled: true,
            jitter_scale: 1.0,
            feedback: 0.95,
            sharpness: 0.5,
        }
    }
}

/// Sharpening
#[derive(Debug, Clone)]
pub struct Sharpening {
    pub enabled: bool,
    pub intensity: f32,
}

impl Default for Sharpening {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
        }
    }
}

/// Fog
#[derive(Debug, Clone)]
pub struct Fog {
    pub enabled: bool,
    pub color: [f32; 3],
    pub density: f32,
    pub start: f32,
    pub end: f32,
    pub height_falloff: f32,
    pub fog_type: FogType,
}

impl Default for Fog {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [0.5, 0.6, 0.7],
            density: 0.01,
            start: 10.0,
            end: 100.0,
            height_falloff: 0.1,
            fog_type: FogType::Exponential,
        }
    }
}

/// Fog type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogType {
    Linear,
    Exponential,
    ExponentialSquared,
    Height,
}

/// Lens flare
#[derive(Debug, Clone)]
pub struct LensFlare {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub ghost_count: u32,
    pub ghost_spacing: f32,
    pub halo_radius: f32,
}

impl Default for LensFlare {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            threshold: 0.8,
            ghost_count: 8,
            ghost_spacing: 0.125,
            halo_radius: 0.6,
        }
    }
}

/// Screen-space reflections
#[derive(Debug, Clone)]
pub struct Ssr {
    pub enabled: bool,
    pub max_distance: f32,
    pub resolution: f32,
    pub step_count: u32,
    pub thickness: f32,
    pub jitter: f32,
}

impl Default for Ssr {
    fn default() -> Self {
        Self {
            enabled: true,
            max_distance: 100.0,
            resolution: 0.5,
            step_count: 64,
            thickness: 0.5,
            jitter: 1.0,
        }
    }
}
