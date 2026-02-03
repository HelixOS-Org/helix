//! Procedural Texture Generation
//!
//! This module provides GPU-accelerated procedural texture generation
//! including noise functions, patterns, and texture synthesis.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::f32::consts::PI;

// ============================================================================
// Noise Types
// ============================================================================

/// Noise algorithm type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoiseType {
    /// Perlin noise.
    Perlin,
    /// Simplex noise.
    Simplex,
    /// Worley/Voronoi noise.
    Worley,
    /// Value noise.
    Value,
    /// Gradient noise.
    Gradient,
    /// Curl noise.
    Curl,
    /// Wavelet noise.
    Wavelet,
    /// Blue noise.
    Blue,
}

/// Fractal type for layering noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FractalType {
    /// No fractal.
    None,
    /// Fractional Brownian Motion.
    FBm,
    /// Ridged multifractal.
    Ridged,
    /// Billow (abs value).
    Billow,
    /// Domain warp.
    DomainWarp,
    /// Hybrid multifractal.
    Hybrid,
}

/// Noise parameters.
#[derive(Debug, Clone)]
pub struct NoiseParams {
    /// Noise type.
    pub noise_type: NoiseType,
    /// Fractal type.
    pub fractal_type: FractalType,
    /// Frequency.
    pub frequency: f32,
    /// Amplitude.
    pub amplitude: f32,
    /// Number of octaves.
    pub octaves: u32,
    /// Lacunarity (frequency multiplier per octave).
    pub lacunarity: f32,
    /// Gain/persistence (amplitude multiplier per octave).
    pub gain: f32,
    /// Seed.
    pub seed: u32,
    /// Offset.
    pub offset: [f32; 3],
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            noise_type: NoiseType::Perlin,
            fractal_type: FractalType::FBm,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 4,
            lacunarity: 2.0,
            gain: 0.5,
            seed: 0,
            offset: [0.0; 3],
        }
    }
}

impl NoiseParams {
    /// Create Perlin noise.
    pub fn perlin() -> Self {
        Self::default()
    }

    /// Create simplex noise.
    pub fn simplex() -> Self {
        Self {
            noise_type: NoiseType::Simplex,
            ..Default::default()
        }
    }

    /// Create Worley noise.
    pub fn worley() -> Self {
        Self {
            noise_type: NoiseType::Worley,
            fractal_type: FractalType::None,
            ..Default::default()
        }
    }

    /// Set frequency.
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency;
        self
    }

    /// Set octaves.
    pub fn with_octaves(mut self, octaves: u32) -> Self {
        self.octaves = octaves;
        self
    }

    /// Set seed.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }
}

// ============================================================================
// Pattern Types
// ============================================================================

/// Pattern type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternType {
    /// Solid color.
    Solid,
    /// Gradient.
    Gradient,
    /// Checkerboard.
    Checker,
    /// Stripe.
    Stripe,
    /// Grid.
    Grid,
    /// Brick.
    Brick,
    /// Tile.
    Tile,
    /// Hexagon.
    Hexagon,
    /// Voronoi cells.
    Voronoi,
    /// Circle.
    Circle,
    /// Ring.
    Ring,
    /// Star.
    Star,
    /// Polygon.
    Polygon,
    /// Wave.
    Wave,
    /// Spiral.
    Spiral,
    /// Dots.
    Dots,
}

/// Gradient mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GradientMode {
    /// Linear gradient.
    Linear,
    /// Radial gradient.
    Radial,
    /// Angular gradient.
    Angular,
    /// Diamond gradient.
    Diamond,
    /// Conical gradient.
    Conical,
    /// Spherical gradient.
    Spherical,
}

/// Gradient stop.
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    /// Position (0-1).
    pub position: f32,
    /// Color (RGBA).
    pub color: [f32; 4],
}

/// Pattern parameters.
#[derive(Debug, Clone)]
pub struct PatternParams {
    /// Pattern type.
    pub pattern_type: PatternType,
    /// Scale.
    pub scale: [f32; 2],
    /// Rotation (radians).
    pub rotation: f32,
    /// Offset.
    pub offset: [f32; 2],
    /// Primary color.
    pub color1: [f32; 4],
    /// Secondary color.
    pub color2: [f32; 4],
    /// Edge smoothness.
    pub smoothness: f32,
    /// Aspect ratio.
    pub aspect: f32,
    /// Gradient stops.
    pub gradient_stops: Vec<GradientStop>,
    /// Gradient mode.
    pub gradient_mode: GradientMode,
}

impl Default for PatternParams {
    fn default() -> Self {
        Self {
            pattern_type: PatternType::Checker,
            scale: [1.0, 1.0],
            rotation: 0.0,
            offset: [0.0, 0.0],
            color1: [1.0, 1.0, 1.0, 1.0],
            color2: [0.0, 0.0, 0.0, 1.0],
            smoothness: 0.0,
            aspect: 1.0,
            gradient_stops: Vec::new(),
            gradient_mode: GradientMode::Linear,
        }
    }
}

impl PatternParams {
    /// Create checkerboard.
    pub fn checker(scale: f32) -> Self {
        Self {
            pattern_type: PatternType::Checker,
            scale: [scale, scale],
            ..Default::default()
        }
    }

    /// Create stripe pattern.
    pub fn stripe(frequency: f32, angle: f32) -> Self {
        Self {
            pattern_type: PatternType::Stripe,
            scale: [frequency, frequency],
            rotation: angle,
            ..Default::default()
        }
    }

    /// Create brick pattern.
    pub fn brick(width: f32, height: f32) -> Self {
        Self {
            pattern_type: PatternType::Brick,
            scale: [width, height],
            aspect: width / height,
            ..Default::default()
        }
    }

    /// Create hexagon pattern.
    pub fn hexagon(size: f32) -> Self {
        Self {
            pattern_type: PatternType::Hexagon,
            scale: [size, size],
            ..Default::default()
        }
    }

    /// Set colors.
    pub fn with_colors(mut self, color1: [f32; 4], color2: [f32; 4]) -> Self {
        self.color1 = color1;
        self.color2 = color2;
        self
    }

    /// Set smoothness.
    pub fn with_smoothness(mut self, smoothness: f32) -> Self {
        self.smoothness = smoothness;
        self
    }
}

// ============================================================================
// Procedural Texture
// ============================================================================

/// Procedural texture definition.
#[derive(Debug, Clone)]
pub struct ProceduralTexture {
    /// Name.
    pub name: String,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Layers for compositing.
    pub layers: Vec<ProceduralLayer>,
    /// Output channels.
    pub channels: TextureChannels,
    /// HDR output.
    pub hdr: bool,
    /// Generate mipmaps.
    pub generate_mipmaps: bool,
}

/// Texture channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureChannels {
    /// Single channel (grayscale).
    R,
    /// Two channels.
    RG,
    /// Three channels (RGB).
    RGB,
    /// Four channels (RGBA).
    RGBA,
}

impl TextureChannels {
    /// Get channel count.
    pub fn count(&self) -> usize {
        match self {
            Self::R => 1,
            Self::RG => 2,
            Self::RGB => 3,
            Self::RGBA => 4,
        }
    }
}

/// Procedural layer.
#[derive(Debug, Clone)]
pub struct ProceduralLayer {
    /// Layer type.
    pub layer_type: LayerType,
    /// Blend mode.
    pub blend_mode: BlendMode,
    /// Opacity.
    pub opacity: f32,
    /// Mask.
    pub mask: Option<Box<ProceduralLayer>>,
    /// Channel mapping.
    pub channel_map: [ChannelSource; 4],
}

/// Layer type.
#[derive(Debug, Clone)]
pub enum LayerType {
    /// Noise layer.
    Noise(NoiseParams),
    /// Pattern layer.
    Pattern(PatternParams),
    /// Gradient.
    Gradient(GradientParams),
    /// Warp.
    Warp(WarpParams),
    /// Filter.
    Filter(FilterParams),
    /// Combine.
    Combine(CombineParams),
}

/// Channel source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSource {
    /// Zero.
    Zero,
    /// One.
    One,
    /// Layer output R.
    R,
    /// Layer output G.
    G,
    /// Layer output B.
    B,
    /// Layer output A.
    A,
}

/// Blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Normal.
    Normal,
    /// Multiply.
    Multiply,
    /// Screen.
    Screen,
    /// Overlay.
    Overlay,
    /// Add.
    Add,
    /// Subtract.
    Subtract,
    /// Difference.
    Difference,
    /// Divide.
    Divide,
    /// Darken.
    Darken,
    /// Lighten.
    Lighten,
    /// SoftLight.
    SoftLight,
    /// HardLight.
    HardLight,
    /// LinearDodge.
    LinearDodge,
    /// LinearBurn.
    LinearBurn,
    /// ColorDodge.
    ColorDodge,
    /// ColorBurn.
    ColorBurn,
}

/// Gradient parameters.
#[derive(Debug, Clone)]
pub struct GradientParams {
    /// Mode.
    pub mode: GradientMode,
    /// Stops.
    pub stops: Vec<GradientStop>,
    /// Angle (for linear).
    pub angle: f32,
    /// Center (for radial).
    pub center: [f32; 2],
    /// Repeat.
    pub repeat: bool,
}

impl Default for GradientParams {
    fn default() -> Self {
        Self {
            mode: GradientMode::Linear,
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                GradientStop {
                    position: 1.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
            ],
            angle: 0.0,
            center: [0.5, 0.5],
            repeat: false,
        }
    }
}

/// Warp parameters.
#[derive(Debug, Clone)]
pub struct WarpParams {
    /// Warp type.
    pub warp_type: WarpType,
    /// Strength.
    pub strength: f32,
    /// Frequency.
    pub frequency: f32,
    /// Noise for warping.
    pub noise: Option<NoiseParams>,
}

/// Warp type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarpType {
    /// Noise-based warp.
    Noise,
    /// Swirl.
    Swirl,
    /// Pinch.
    Pinch,
    /// Bulge.
    Bulge,
    /// Wave.
    Wave,
    /// Ripple.
    Ripple,
    /// Twist.
    Twist,
}

impl Default for WarpParams {
    fn default() -> Self {
        Self {
            warp_type: WarpType::Noise,
            strength: 0.1,
            frequency: 1.0,
            noise: Some(NoiseParams::simplex().with_frequency(4.0)),
        }
    }
}

/// Filter parameters.
#[derive(Debug, Clone)]
pub struct FilterParams {
    /// Filter type.
    pub filter_type: FilterType,
    /// Parameters.
    pub params: [f32; 4],
}

/// Filter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterType {
    /// Blur.
    Blur,
    /// Sharpen.
    Sharpen,
    /// Emboss.
    Emboss,
    /// Edge detect.
    Edge,
    /// Levels adjustment.
    Levels,
    /// Curves.
    Curves,
    /// Invert.
    Invert,
    /// Threshold.
    Threshold,
    /// Posterize.
    Posterize,
    /// Normal map from height.
    NormalMap,
    /// Height from normal.
    HeightFromNormal,
    /// Ambient occlusion.
    AO,
}

/// Combine parameters.
#[derive(Debug, Clone)]
pub struct CombineParams {
    /// Combine operation.
    pub operation: CombineOp,
    /// Layers to combine.
    pub layers: Vec<usize>,
}

/// Combine operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombineOp {
    /// Add.
    Add,
    /// Subtract.
    Subtract,
    /// Multiply.
    Multiply,
    /// Divide.
    Divide,
    /// Min.
    Min,
    /// Max.
    Max,
    /// Average.
    Average,
}

// ============================================================================
// Texture Generator
// ============================================================================

/// Procedural texture generator.
pub struct TextureGenerator {
    /// Permutation table for noise.
    perm: [u8; 512],
    /// Gradient table.
    grad: [[f32; 3]; 16],
}

impl TextureGenerator {
    /// Create a new generator.
    pub fn new(seed: u32) -> Self {
        let mut perm = [0u8; 512];
        let mut grad = [[0.0f32; 3]; 16];

        // Initialize permutation table
        let mut p: [u8; 256] = core::array::from_fn(|i| i as u8);

        // Fisher-Yates shuffle with LCG
        let mut rng = seed;
        for i in (1..256).rev() {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let j = (rng as usize >> 16) % (i + 1);
            p.swap(i, j);
        }

        for i in 0..512 {
            perm[i] = p[i & 255];
        }

        // Initialize gradients
        let gradients = [
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
            [1.0, 0.0, -1.0],
            [-1.0, 0.0, -1.0],
            [0.0, 1.0, 1.0],
            [0.0, -1.0, 1.0],
            [0.0, 1.0, -1.0],
            [0.0, -1.0, -1.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [0.0, -1.0, 1.0],
            [0.0, -1.0, -1.0],
        ];
        grad = gradients;

        Self { perm, grad }
    }

    /// Generate texture data.
    pub fn generate(&self, texture: &ProceduralTexture) -> Vec<f32> {
        let channels = texture.channels.count();
        let size = texture.width as usize * texture.height as usize * channels;
        let mut data = vec![0.0f32; size];

        // Process each layer
        for layer in &texture.layers {
            let layer_data = self.generate_layer(layer, texture.width, texture.height);
            self.blend_layer(
                &mut data,
                &layer_data,
                layer,
                texture.width,
                texture.height,
                channels,
            );
        }

        data
    }

    /// Generate a single layer.
    fn generate_layer(&self, layer: &ProceduralLayer, width: u32, height: u32) -> Vec<f32> {
        let size = width as usize * height as usize * 4;
        let mut data = vec![0.0f32; size];

        match &layer.layer_type {
            LayerType::Noise(params) => {
                self.generate_noise(&mut data, params, width, height);
            },
            LayerType::Pattern(params) => {
                self.generate_pattern(&mut data, params, width, height);
            },
            LayerType::Gradient(params) => {
                self.generate_gradient(&mut data, params, width, height);
            },
            LayerType::Warp(_params) => {
                // Warp would be applied to existing data
            },
            LayerType::Filter(_params) => {
                // Filters applied to existing data
            },
            LayerType::Combine(_params) => {
                // Combine multiple layers
            },
        }

        data
    }

    /// Generate noise data.
    fn generate_noise(&self, data: &mut [f32], params: &NoiseParams, width: u32, height: u32) {
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;

                let px = (u + params.offset[0]) * params.frequency;
                let py = (v + params.offset[1]) * params.frequency;

                let value = match params.fractal_type {
                    FractalType::None => self.sample_noise(params.noise_type, px, py, 0.0),
                    FractalType::FBm => self.fbm(params, px, py),
                    FractalType::Ridged => self.ridged(params, px, py),
                    FractalType::Billow => self.billow(params, px, py),
                    _ => self.fbm(params, px, py),
                };

                let value = (value * 0.5 + 0.5) * params.amplitude;
                let idx = ((y * width + x) * 4) as usize;
                data[idx] = value;
                data[idx + 1] = value;
                data[idx + 2] = value;
                data[idx + 3] = 1.0;
            }
        }
    }

    /// Sample noise at a point.
    fn sample_noise(&self, noise_type: NoiseType, x: f32, y: f32, z: f32) -> f32 {
        match noise_type {
            NoiseType::Perlin => self.perlin_3d(x, y, z),
            NoiseType::Simplex => self.simplex_2d(x, y),
            NoiseType::Worley => self.worley_2d(x, y),
            NoiseType::Value => self.value_2d(x, y),
            _ => self.perlin_3d(x, y, z),
        }
    }

    /// Perlin noise 3D.
    fn perlin_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;

        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();

        let u = Self::fade(xf);
        let v = Self::fade(yf);
        let w = Self::fade(zf);

        let aaa = self.hash(xi, yi, zi);
        let aba = self.hash(xi, yi + 1, zi);
        let aab = self.hash(xi, yi, zi + 1);
        let abb = self.hash(xi, yi + 1, zi + 1);
        let baa = self.hash(xi + 1, yi, zi);
        let bba = self.hash(xi + 1, yi + 1, zi);
        let bab = self.hash(xi + 1, yi, zi + 1);
        let bbb = self.hash(xi + 1, yi + 1, zi + 1);

        let x1 = Self::lerp(
            self.grad_3d(aaa, xf, yf, zf),
            self.grad_3d(baa, xf - 1.0, yf, zf),
            u,
        );
        let x2 = Self::lerp(
            self.grad_3d(aba, xf, yf - 1.0, zf),
            self.grad_3d(bba, xf - 1.0, yf - 1.0, zf),
            u,
        );
        let y1 = Self::lerp(x1, x2, v);

        let x1 = Self::lerp(
            self.grad_3d(aab, xf, yf, zf - 1.0),
            self.grad_3d(bab, xf - 1.0, yf, zf - 1.0),
            u,
        );
        let x2 = Self::lerp(
            self.grad_3d(abb, xf, yf - 1.0, zf - 1.0),
            self.grad_3d(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
            u,
        );
        let y2 = Self::lerp(x1, x2, v);

        Self::lerp(y1, y2, w)
    }

    /// Simplex noise 2D.
    fn simplex_2d(&self, x: f32, y: f32) -> f32 {
        const F2: f32 = 0.366025403784; // (sqrt(3)-1)/2
        const G2: f32 = 0.211324865405; // (3-sqrt(3))/6

        let s = (x + y) * F2;
        let i = (x + s).floor() as i32;
        let j = (y + s).floor() as i32;

        let t = (i + j) as f32 * G2;
        let x0 = x - (i as f32 - t);
        let y0 = y - (j as f32 - t);

        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

        let x1 = x0 - i1 as f32 + G2;
        let y1 = y0 - j1 as f32 + G2;
        let x2 = x0 - 1.0 + 2.0 * G2;
        let y2 = y0 - 1.0 + 2.0 * G2;

        let gi0 = self.hash(i, j, 0) & 7;
        let gi1 = self.hash(i + i1, j + j1, 0) & 7;
        let gi2 = self.hash(i + 1, j + 1, 0) & 7;

        let mut n0 = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;

        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 >= 0.0 {
            let t0 = t0 * t0;
            n0 = t0 * t0 * self.grad_2d(gi0 as u8, x0, y0);
        }

        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 >= 0.0 {
            let t1 = t1 * t1;
            n1 = t1 * t1 * self.grad_2d(gi1 as u8, x1, y1);
        }

        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 >= 0.0 {
            let t2 = t2 * t2;
            n2 = t2 * t2 * self.grad_2d(gi2 as u8, x2, y2);
        }

        70.0 * (n0 + n1 + n2)
    }

    /// Worley noise 2D.
    fn worley_2d(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        let mut min_dist = f32::MAX;

        for dx in -1..=1 {
            for dy in -1..=1 {
                let cx = xi + dx;
                let cy = yi + dy;

                // Random point in cell
                let h = self.hash(cx, cy, 0);
                let px = cx as f32 + (h as f32 / 255.0);
                let py = cy as f32 + ((h >> 8) as f32 / 255.0);

                let dist = (x - px) * (x - px) + (y - py) * (y - py);
                min_dist = min_dist.min(dist);
            }
        }

        1.0 - min_dist.sqrt().min(1.0)
    }

    /// Value noise 2D.
    fn value_2d(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        let xf = x - x.floor();
        let yf = y - y.floor();

        let u = Self::fade(xf);
        let v = Self::fade(yf);

        let aa = self.hash(xi, yi, 0) as f32 / 255.0;
        let ab = self.hash(xi, yi + 1, 0) as f32 / 255.0;
        let ba = self.hash(xi + 1, yi, 0) as f32 / 255.0;
        let bb = self.hash(xi + 1, yi + 1, 0) as f32 / 255.0;

        let x1 = Self::lerp(aa, ba, u);
        let x2 = Self::lerp(ab, bb, u);

        Self::lerp(x1, x2, v) * 2.0 - 1.0
    }

    /// FBm.
    fn fbm(&self, params: &NoiseParams, x: f32, y: f32) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for _ in 0..params.octaves {
            value +=
                self.sample_noise(params.noise_type, x * frequency, y * frequency, 0.0) * amplitude;
            max_value += amplitude;
            amplitude *= params.gain;
            frequency *= params.lacunarity;
        }

        value / max_value
    }

    /// Ridged multifractal.
    fn ridged(&self, params: &NoiseParams, x: f32, y: f32) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut weight = 1.0;

        for _ in 0..params.octaves {
            let signal = self.sample_noise(params.noise_type, x * frequency, y * frequency, 0.0);
            let signal = 1.0 - signal.abs();
            let signal = signal * signal * weight;
            weight = (signal * 2.0).min(1.0);
            value += signal * amplitude;
            amplitude *= params.gain;
            frequency *= params.lacunarity;
        }

        value
    }

    /// Billow noise.
    fn billow(&self, params: &NoiseParams, x: f32, y: f32) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for _ in 0..params.octaves {
            let signal = self.sample_noise(params.noise_type, x * frequency, y * frequency, 0.0);
            value += signal.abs() * amplitude;
            max_value += amplitude;
            amplitude *= params.gain;
            frequency *= params.lacunarity;
        }

        value / max_value * 2.0 - 1.0
    }

    /// Generate pattern.
    fn generate_pattern(&self, data: &mut [f32], params: &PatternParams, width: u32, height: u32) {
        let cos_r = params.rotation.cos();
        let sin_r = params.rotation.sin();

        for y in 0..height {
            for x in 0..width {
                let mut u = x as f32 / width as f32 - 0.5;
                let mut v = y as f32 / height as f32 - 0.5;

                // Apply rotation
                let ru = u * cos_r - v * sin_r;
                let rv = u * sin_r + v * cos_r;
                u = ru + 0.5 - params.offset[0];
                v = rv + 0.5 - params.offset[1];

                // Scale
                u *= params.scale[0];
                v *= params.scale[1];

                let value = match params.pattern_type {
                    PatternType::Checker => self.checker(u, v, params.smoothness),
                    PatternType::Stripe => self.stripe(u, params.smoothness),
                    PatternType::Grid => self.grid(u, v, params.smoothness),
                    PatternType::Brick => self.brick(u, v, params.aspect, params.smoothness),
                    PatternType::Hexagon => self.hexagon(u, v, params.smoothness),
                    PatternType::Circle => self.circle(u - 0.5, v - 0.5, 0.4, params.smoothness),
                    PatternType::Dots => self.dots(u, v, 0.3, params.smoothness),
                    PatternType::Wave => self.wave(u, v, params.smoothness),
                    PatternType::Spiral => self.spiral(u - 0.5, v - 0.5, params.smoothness),
                    _ => 0.5,
                };

                // Interpolate colors
                let color = Self::mix_color(params.color1, params.color2, value);

                let idx = ((y * width + x) * 4) as usize;
                data[idx] = color[0];
                data[idx + 1] = color[1];
                data[idx + 2] = color[2];
                data[idx + 3] = color[3];
            }
        }
    }

    /// Generate gradient.
    fn generate_gradient(
        &self,
        data: &mut [f32],
        params: &GradientParams,
        width: u32,
        height: u32,
    ) {
        let cos_a = params.angle.cos();
        let sin_a = params.angle.sin();

        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;

                let t = match params.mode {
                    GradientMode::Linear => {
                        let ru = (u - 0.5) * cos_a - (v - 0.5) * sin_a + 0.5;
                        ru
                    },
                    GradientMode::Radial => {
                        let dx = u - params.center[0];
                        let dy = v - params.center[1];
                        (dx * dx + dy * dy).sqrt()
                    },
                    GradientMode::Angular => {
                        let dx = u - params.center[0];
                        let dy = v - params.center[1];
                        (dy.atan2(dx) / PI + 1.0) * 0.5
                    },
                    GradientMode::Diamond => {
                        let dx = (u - params.center[0]).abs();
                        let dy = (v - params.center[1]).abs();
                        dx + dy
                    },
                    GradientMode::Conical => {
                        let dx = u - params.center[0];
                        let dy = v - params.center[1];
                        dy.atan2(dx) / (2.0 * PI) + 0.5
                    },
                    GradientMode::Spherical => {
                        let dx = u - params.center[0];
                        let dy = v - params.center[1];
                        let d = (dx * dx + dy * dy).sqrt();
                        (1.0 - (d * PI).cos()) * 0.5
                    },
                };

                let t = if params.repeat {
                    t.fract()
                } else {
                    t.clamp(0.0, 1.0)
                };
                let color = self.sample_gradient(&params.stops, t);

                let idx = ((y * width + x) * 4) as usize;
                data[idx] = color[0];
                data[idx + 1] = color[1];
                data[idx + 2] = color[2];
                data[idx + 3] = color[3];
            }
        }
    }

    /// Sample gradient at position.
    fn sample_gradient(&self, stops: &[GradientStop], t: f32) -> [f32; 4] {
        if stops.is_empty() {
            return [0.0, 0.0, 0.0, 1.0];
        }

        if t <= stops[0].position {
            return stops[0].color;
        }

        if t >= stops[stops.len() - 1].position {
            return stops[stops.len() - 1].color;
        }

        for i in 0..stops.len() - 1 {
            if t >= stops[i].position && t <= stops[i + 1].position {
                let range = stops[i + 1].position - stops[i].position;
                let local_t = if range > 0.0 {
                    (t - stops[i].position) / range
                } else {
                    0.0
                };
                return Self::mix_color(stops[i].color, stops[i + 1].color, local_t);
            }
        }

        stops[0].color
    }

    // Pattern helpers
    fn checker(&self, u: f32, v: f32, smooth: f32) -> f32 {
        let fu = (u.floor() as i32) & 1;
        let fv = (v.floor() as i32) & 1;
        let value = (fu ^ fv) as f32;

        if smooth > 0.0 {
            let su = Self::smoothstep(0.5 - smooth, 0.5 + smooth, u.fract());
            let sv = Self::smoothstep(0.5 - smooth, 0.5 + smooth, v.fract());
            Self::lerp(value, 1.0 - value, (su + sv) * 0.5)
        } else {
            value
        }
    }

    fn stripe(&self, u: f32, smooth: f32) -> f32 {
        let f = u.fract();
        if smooth > 0.0 {
            Self::smoothstep(0.5 - smooth, 0.5 + smooth, f)
        } else {
            if f < 0.5 {
                0.0
            } else {
                1.0
            }
        }
    }

    fn grid(&self, u: f32, v: f32, smooth: f32) -> f32 {
        let fu = u.fract();
        let fv = v.fract();
        let line_u = if smooth > 0.0 {
            1.0 - Self::smoothstep(0.0, smooth, fu.min(1.0 - fu))
        } else {
            if fu < 0.1 || fu > 0.9 {
                1.0
            } else {
                0.0
            }
        };
        let line_v = if smooth > 0.0 {
            1.0 - Self::smoothstep(0.0, smooth, fv.min(1.0 - fv))
        } else {
            if fv < 0.1 || fv > 0.9 {
                1.0
            } else {
                0.0
            }
        };
        line_u.max(line_v)
    }

    fn brick(&self, u: f32, v: f32, aspect: f32, smooth: f32) -> f32 {
        let row = v.floor() as i32;
        let offset = if row & 1 == 1 { 0.5 } else { 0.0 };
        let bu = (u + offset).fract();
        let bv = v.fract();

        let mortar = 0.05;
        let mx = if smooth > 0.0 {
            1.0 - Self::smoothstep(0.0, smooth, (bu.min(1.0 - bu) - mortar).max(0.0))
        } else {
            if bu < mortar || bu > 1.0 - mortar {
                1.0
            } else {
                0.0
            }
        };
        let my = if smooth > 0.0 {
            1.0 - Self::smoothstep(0.0, smooth, (bv.min(1.0 - bv) - mortar).max(0.0))
        } else {
            if bv < mortar || bv > 1.0 - mortar {
                1.0
            } else {
                0.0
            }
        };

        1.0 - mx.max(my)
    }

    fn hexagon(&self, u: f32, v: f32, _smooth: f32) -> f32 {
        let v = v * 1.1547; // 2/sqrt(3)
        let row = v.floor() as i32;
        let offset = if row & 1 == 1 { 0.5 } else { 0.0 };
        let hu = (u + offset) * 2.0;
        let hv = v * 2.0;

        let fu = hu.fract() - 0.5;
        let fv = hv.fract() - 0.5;

        (fu.abs() * 0.866 + fv.abs() * 0.5).min(1.0)
    }

    fn circle(&self, u: f32, v: f32, radius: f32, smooth: f32) -> f32 {
        let d = (u * u + v * v).sqrt();
        if smooth > 0.0 {
            1.0 - Self::smoothstep(radius - smooth, radius + smooth, d)
        } else {
            if d < radius {
                1.0
            } else {
                0.0
            }
        }
    }

    fn dots(&self, u: f32, v: f32, radius: f32, smooth: f32) -> f32 {
        let fu = u.fract() - 0.5;
        let fv = v.fract() - 0.5;
        self.circle(fu, fv, radius, smooth)
    }

    fn wave(&self, u: f32, v: f32, smooth: f32) -> f32 {
        let wave = (u * PI * 2.0).sin() * 0.1;
        let d = (v - 0.5 - wave).abs();
        if smooth > 0.0 {
            1.0 - Self::smoothstep(0.0, smooth, d - 0.05)
        } else {
            if d < 0.05 {
                1.0
            } else {
                0.0
            }
        }
    }

    fn spiral(&self, u: f32, v: f32, _smooth: f32) -> f32 {
        let angle = v.atan2(u);
        let dist = (u * u + v * v).sqrt();
        let spiral = (angle / PI + dist * 4.0).fract();
        spiral
    }

    /// Blend layer into output.
    fn blend_layer(
        &self,
        output: &mut [f32],
        layer: &[f32],
        layer_params: &ProceduralLayer,
        width: u32,
        height: u32,
        channels: usize,
    ) {
        let opacity = layer_params.opacity;

        for y in 0..height as usize {
            for x in 0..width as usize {
                let src_idx = (y * width as usize + x) * 4;
                let dst_idx = (y * width as usize + x) * channels;

                for c in 0..channels.min(4) {
                    let src = layer[src_idx + c];
                    let dst = output[dst_idx + c];

                    let blended = match layer_params.blend_mode {
                        BlendMode::Normal => src,
                        BlendMode::Multiply => src * dst,
                        BlendMode::Screen => 1.0 - (1.0 - src) * (1.0 - dst),
                        BlendMode::Overlay => {
                            if dst < 0.5 {
                                2.0 * src * dst
                            } else {
                                1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
                            }
                        },
                        BlendMode::Add => (src + dst).min(1.0),
                        BlendMode::Subtract => (dst - src).max(0.0),
                        BlendMode::Difference => (src - dst).abs(),
                        BlendMode::Divide => {
                            if src > 0.0 {
                                dst / src
                            } else {
                                1.0
                            }
                        },
                        BlendMode::Darken => src.min(dst),
                        BlendMode::Lighten => src.max(dst),
                        _ => src,
                    };

                    output[dst_idx + c] = Self::lerp(dst, blended, opacity);
                }
            }
        }
    }

    // Utility functions
    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + t * (b - a)
    }

    fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn mix_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        [
            Self::lerp(a[0], b[0], t),
            Self::lerp(a[1], b[1], t),
            Self::lerp(a[2], b[2], t),
            Self::lerp(a[3], b[3], t),
        ]
    }

    fn hash(&self, x: i32, y: i32, z: i32) -> u8 {
        let x = x.rem_euclid(256) as usize;
        let y = y.rem_euclid(256) as usize;
        let z = z.rem_euclid(256) as usize;
        self.perm[(self.perm[(self.perm[x] as usize + y) & 255] as usize + z) & 255]
    }

    fn grad_3d(&self, hash: u8, x: f32, y: f32, z: f32) -> f32 {
        let g = &self.grad[(hash & 15) as usize];
        g[0] * x + g[1] * y + g[2] * z
    }

    fn grad_2d(&self, hash: u8, x: f32, y: f32) -> f32 {
        let g = &self.grad[(hash & 7) as usize];
        g[0] * x + g[1] * y
    }
}

// ============================================================================
// Preset Textures
// ============================================================================

/// Preset procedural textures.
pub struct ProceduralPresets;

impl ProceduralPresets {
    /// Create noise texture.
    pub fn noise(size: u32, noise_type: NoiseType) -> ProceduralTexture {
        ProceduralTexture {
            name: String::from("noise"),
            width: size,
            height: size,
            layers: vec![ProceduralLayer {
                layer_type: LayerType::Noise(NoiseParams {
                    noise_type,
                    ..Default::default()
                }),
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
                mask: None,
                channel_map: [
                    ChannelSource::R,
                    ChannelSource::R,
                    ChannelSource::R,
                    ChannelSource::One,
                ],
            }],
            channels: TextureChannels::RGBA,
            hdr: false,
            generate_mipmaps: true,
        }
    }

    /// Create checkerboard texture.
    pub fn checkerboard(size: u32, scale: f32) -> ProceduralTexture {
        ProceduralTexture {
            name: String::from("checkerboard"),
            width: size,
            height: size,
            layers: vec![ProceduralLayer {
                layer_type: LayerType::Pattern(PatternParams::checker(scale)),
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
                mask: None,
                channel_map: [
                    ChannelSource::R,
                    ChannelSource::G,
                    ChannelSource::B,
                    ChannelSource::A,
                ],
            }],
            channels: TextureChannels::RGBA,
            hdr: false,
            generate_mipmaps: true,
        }
    }

    /// Create grid texture.
    pub fn grid(size: u32, scale: f32) -> ProceduralTexture {
        ProceduralTexture {
            name: String::from("grid"),
            width: size,
            height: size,
            layers: vec![ProceduralLayer {
                layer_type: LayerType::Pattern(PatternParams {
                    pattern_type: PatternType::Grid,
                    scale: [scale, scale],
                    smoothness: 0.02,
                    ..Default::default()
                }),
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
                mask: None,
                channel_map: [
                    ChannelSource::R,
                    ChannelSource::G,
                    ChannelSource::B,
                    ChannelSource::A,
                ],
            }],
            channels: TextureChannels::RGBA,
            hdr: false,
            generate_mipmaps: true,
        }
    }

    /// Create gradient texture.
    pub fn gradient(size: u32, mode: GradientMode) -> ProceduralTexture {
        ProceduralTexture {
            name: String::from("gradient"),
            width: size,
            height: size,
            layers: vec![ProceduralLayer {
                layer_type: LayerType::Gradient(GradientParams {
                    mode,
                    ..Default::default()
                }),
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
                mask: None,
                channel_map: [
                    ChannelSource::R,
                    ChannelSource::G,
                    ChannelSource::B,
                    ChannelSource::A,
                ],
            }],
            channels: TextureChannels::RGBA,
            hdr: false,
            generate_mipmaps: true,
        }
    }

    /// Create normal map from height.
    pub fn normal_from_height(size: u32) -> ProceduralTexture {
        ProceduralTexture {
            name: String::from("normal"),
            width: size,
            height: size,
            layers: vec![
                // Height layer
                ProceduralLayer {
                    layer_type: LayerType::Noise(NoiseParams::perlin().with_frequency(4.0)),
                    blend_mode: BlendMode::Normal,
                    opacity: 1.0,
                    mask: None,
                    channel_map: [
                        ChannelSource::R,
                        ChannelSource::R,
                        ChannelSource::R,
                        ChannelSource::One,
                    ],
                },
                // Normal map filter
                ProceduralLayer {
                    layer_type: LayerType::Filter(FilterParams {
                        filter_type: FilterType::NormalMap,
                        params: [1.0, 0.0, 0.0, 0.0],
                    }),
                    blend_mode: BlendMode::Normal,
                    opacity: 1.0,
                    mask: None,
                    channel_map: [
                        ChannelSource::R,
                        ChannelSource::G,
                        ChannelSource::B,
                        ChannelSource::One,
                    ],
                },
            ],
            channels: TextureChannels::RGBA,
            hdr: false,
            generate_mipmaps: true,
        }
    }
}
