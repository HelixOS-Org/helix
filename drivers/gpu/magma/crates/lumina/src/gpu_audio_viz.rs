//! GPU Audio Visualization System for Lumina
//!
//! This module provides comprehensive GPU-accelerated audio visualization including
//! spectrum analyzers, waveforms, beat detection, and reactive effects.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Audio Viz System Handles
// ============================================================================

/// GPU audio viz system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuAudioVizSystemHandle(pub u64);

impl GpuAudioVizSystemHandle {
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

impl Default for GpuAudioVizSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Visualizer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VisualizerHandle(pub u64);

impl VisualizerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for VisualizerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Audio source handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AudioSourceHandle(pub u64);

impl AudioSourceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for AudioSourceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AudioEffectHandle(pub u64);

impl AudioEffectHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for AudioEffectHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Audio Viz System Creation
// ============================================================================

/// GPU audio viz system create info
#[derive(Clone, Debug)]
pub struct GpuAudioVizSystemCreateInfo {
    /// Name
    pub name: String,
    /// FFT size
    pub fft_size: u32,
    /// Sample rate
    pub sample_rate: u32,
    /// Max visualizers
    pub max_visualizers: u32,
    /// Features
    pub features: AudioVizFeatures,
    /// Analysis settings
    pub analysis: AudioAnalysisSettings,
}

impl GpuAudioVizSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            fft_size: 2048,
            sample_rate: 44100,
            max_visualizers: 32,
            features: AudioVizFeatures::all(),
            analysis: AudioAnalysisSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With FFT size
    pub fn with_fft_size(mut self, size: u32) -> Self {
        self.fft_size = size;
        self
    }

    /// With sample rate
    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// With features
    pub fn with_features(mut self, features: AudioVizFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With analysis
    pub fn with_analysis(mut self, analysis: AudioAnalysisSettings) -> Self {
        self.analysis = analysis;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_fft_size(4096)
            .with_sample_rate(48000)
            .with_analysis(AudioAnalysisSettings::high_quality())
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_fft_size(1024)
            .with_features(AudioVizFeatures::BASIC)
            .with_analysis(AudioAnalysisSettings::mobile())
    }
}

impl Default for GpuAudioVizSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Audio viz features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct AudioVizFeatures: u32 {
        /// None
        const NONE = 0;
        /// Spectrum analyzer
        const SPECTRUM = 1 << 0;
        /// Waveform display
        const WAVEFORM = 1 << 1;
        /// Beat detection
        const BEAT_DETECT = 1 << 2;
        /// Frequency bands
        const BANDS = 1 << 3;
        /// 3D visualization
        const VIZ_3D = 1 << 4;
        /// Particle effects
        const PARTICLES = 1 << 5;
        /// Color reactive
        const COLOR_REACTIVE = 1 << 6;
        /// GPU analysis
        const GPU_ANALYSIS = 1 << 7;
        /// Basic
        const BASIC = Self::SPECTRUM.bits() | Self::WAVEFORM.bits() | Self::BANDS.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for AudioVizFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Audio Analysis
// ============================================================================

/// Audio analysis settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AudioAnalysisSettings {
    /// Smoothing factor (0-1)
    pub smoothing: f32,
    /// Min decibels
    pub min_db: f32,
    /// Max decibels
    pub max_db: f32,
    /// Beat detection sensitivity
    pub beat_sensitivity: f32,
    /// Beat decay rate
    pub beat_decay: f32,
    /// Frequency band count
    pub band_count: u32,
    /// Octave bands (vs linear)
    pub octave_bands: bool,
    /// Window function
    pub window: WindowFunction,
}

impl AudioAnalysisSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            smoothing: 0.8,
            min_db: -100.0,
            max_db: 0.0,
            beat_sensitivity: 0.5,
            beat_decay: 0.9,
            band_count: 32,
            octave_bands: true,
            window: WindowFunction::Hann,
        }
    }

    /// With smoothing
    pub const fn with_smoothing(mut self, smoothing: f32) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// With beat sensitivity
    pub const fn with_beat_sensitivity(mut self, sensitivity: f32) -> Self {
        self.beat_sensitivity = sensitivity;
        self
    }

    /// With band count
    pub const fn with_bands(mut self, count: u32) -> Self {
        self.band_count = count;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            smoothing: 0.85,
            min_db: -100.0,
            max_db: 0.0,
            beat_sensitivity: 0.6,
            beat_decay: 0.92,
            band_count: 64,
            octave_bands: true,
            window: WindowFunction::Blackman,
        }
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            smoothing: 0.75,
            min_db: -80.0,
            max_db: 0.0,
            beat_sensitivity: 0.4,
            beat_decay: 0.85,
            band_count: 16,
            octave_bands: true,
            window: WindowFunction::Hann,
        }
    }
}

impl Default for AudioAnalysisSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Window function for FFT
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WindowFunction {
    /// Rectangular (no window)
    Rectangular = 0,
    /// Hann window
    #[default]
    Hann        = 1,
    /// Hamming window
    Hamming     = 2,
    /// Blackman window
    Blackman    = 3,
    /// Kaiser window
    Kaiser      = 4,
}

// ============================================================================
// Visualizer Types
// ============================================================================

/// Visualizer create info
#[derive(Clone, Debug)]
pub struct VisualizerCreateInfo {
    /// Name
    pub name: String,
    /// Visualizer type
    pub viz_type: VisualizerType,
    /// Position
    pub position: [f32; 3],
    /// Size
    pub size: [f32; 2],
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Settings
    pub settings: VisualizerSettings,
}

impl VisualizerCreateInfo {
    /// Creates new visualizer
    pub fn new(name: impl Into<String>, viz_type: VisualizerType) -> Self {
        Self {
            name: name.into(),
            viz_type,
            position: [0.0; 3],
            size: [1.0, 0.5],
            color_scheme: ColorScheme::default(),
            settings: VisualizerSettings::default(),
        }
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With size
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }

    /// With color scheme
    pub fn with_colors(mut self, scheme: ColorScheme) -> Self {
        self.color_scheme = scheme;
        self
    }

    /// With settings
    pub fn with_settings(mut self, settings: VisualizerSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Spectrum bars preset
    pub fn spectrum_bars(name: impl Into<String>) -> Self {
        Self::new(name, VisualizerType::SpectrumBars)
    }

    /// Waveform preset
    pub fn waveform(name: impl Into<String>) -> Self {
        Self::new(name, VisualizerType::Waveform)
    }

    /// Circular preset
    pub fn circular(name: impl Into<String>) -> Self {
        Self::new(name, VisualizerType::Circular)
    }

    /// 3D terrain preset
    pub fn terrain_3d(name: impl Into<String>) -> Self {
        Self::new(name, VisualizerType::Terrain3D)
    }

    /// Particle swarm preset
    pub fn particles(name: impl Into<String>) -> Self {
        Self::new(name, VisualizerType::Particles)
    }
}

impl Default for VisualizerCreateInfo {
    fn default() -> Self {
        Self::new("Visualizer", VisualizerType::SpectrumBars)
    }
}

/// Visualizer type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VisualizerType {
    /// Spectrum bars
    #[default]
    SpectrumBars = 0,
    /// Waveform line
    Waveform     = 1,
    /// Circular spectrum
    Circular     = 2,
    /// 3D terrain
    Terrain3D    = 3,
    /// Particle system
    Particles    = 4,
    /// VU meter
    VuMeter      = 5,
    /// Oscilloscope
    Oscilloscope = 6,
    /// Spectrogram
    Spectrogram  = 7,
}

/// Visualizer settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VisualizerSettings {
    /// Amplitude scale
    pub amplitude_scale: f32,
    /// Frequency scale
    pub frequency_scale: f32,
    /// Glow intensity
    pub glow: f32,
    /// Blur amount
    pub blur: f32,
    /// Bar width (for bars)
    pub bar_width: f32,
    /// Bar gap (for bars)
    pub bar_gap: f32,
    /// Line width (for waveform)
    pub line_width: f32,
    /// Mirror mode
    pub mirror: bool,
    /// Reflection
    pub reflection: bool,
    /// Smoothing
    pub smoothing: f32,
}

impl VisualizerSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            amplitude_scale: 1.0,
            frequency_scale: 1.0,
            glow: 0.5,
            blur: 0.0,
            bar_width: 0.8,
            bar_gap: 0.1,
            line_width: 2.0,
            mirror: false,
            reflection: false,
            smoothing: 0.8,
        }
    }

    /// With amplitude scale
    pub const fn with_amplitude(mut self, scale: f32) -> Self {
        self.amplitude_scale = scale;
        self
    }

    /// With glow
    pub const fn with_glow(mut self, glow: f32) -> Self {
        self.glow = glow;
        self
    }

    /// With mirror
    pub const fn with_mirror(mut self) -> Self {
        self.mirror = true;
        self
    }

    /// With reflection
    pub const fn with_reflection(mut self) -> Self {
        self.reflection = true;
        self
    }

    /// Neon style
    pub const fn neon() -> Self {
        Self {
            amplitude_scale: 1.2,
            frequency_scale: 1.0,
            glow: 1.0,
            blur: 0.2,
            bar_width: 0.7,
            bar_gap: 0.15,
            line_width: 3.0,
            mirror: false,
            reflection: true,
            smoothing: 0.85,
        }
    }

    /// Minimal style
    pub const fn minimal() -> Self {
        Self {
            amplitude_scale: 0.8,
            frequency_scale: 1.0,
            glow: 0.0,
            blur: 0.0,
            bar_width: 0.9,
            bar_gap: 0.05,
            line_width: 1.0,
            mirror: false,
            reflection: false,
            smoothing: 0.7,
        }
    }
}

impl Default for VisualizerSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Color Schemes
// ============================================================================

/// Color scheme
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorScheme {
    /// Primary color
    pub primary: [f32; 4],
    /// Secondary color
    pub secondary: [f32; 4],
    /// Accent color
    pub accent: [f32; 4],
    /// Background color
    pub background: [f32; 4],
    /// Color mode
    pub mode: ColorMode,
    /// Gradient direction
    pub gradient_angle: f32,
}

impl ColorScheme {
    /// Creates new color scheme
    pub const fn new(primary: [f32; 4], secondary: [f32; 4]) -> Self {
        Self {
            primary,
            secondary,
            accent: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.0, 0.0, 1.0],
            mode: ColorMode::Gradient,
            gradient_angle: 90.0,
        }
    }

    /// With accent
    pub const fn with_accent(mut self, accent: [f32; 4]) -> Self {
        self.accent = accent;
        self
    }

    /// With mode
    pub const fn with_mode(mut self, mode: ColorMode) -> Self {
        self.mode = mode;
        self
    }

    /// Neon cyan preset
    pub const fn neon_cyan() -> Self {
        Self {
            primary: [0.0, 1.0, 1.0, 1.0],
            secondary: [1.0, 0.0, 1.0, 1.0],
            accent: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.05, 0.1, 1.0],
            mode: ColorMode::Gradient,
            gradient_angle: 90.0,
        }
    }

    /// Fire preset
    pub const fn fire() -> Self {
        Self {
            primary: [1.0, 0.0, 0.0, 1.0],
            secondary: [1.0, 0.5, 0.0, 1.0],
            accent: [1.0, 1.0, 0.0, 1.0],
            background: [0.1, 0.0, 0.0, 1.0],
            mode: ColorMode::Gradient,
            gradient_angle: 90.0,
        }
    }

    /// Ocean preset
    pub const fn ocean() -> Self {
        Self {
            primary: [0.0, 0.3, 0.8, 1.0],
            secondary: [0.0, 0.8, 0.8, 1.0],
            accent: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.0, 0.1, 1.0],
            mode: ColorMode::Gradient,
            gradient_angle: 90.0,
        }
    }

    /// Rainbow preset
    pub const fn rainbow() -> Self {
        Self {
            primary: [1.0, 0.0, 0.0, 1.0],
            secondary: [0.0, 0.0, 1.0, 1.0],
            accent: [0.0, 1.0, 0.0, 1.0],
            background: [0.05, 0.05, 0.05, 1.0],
            mode: ColorMode::Frequency,
            gradient_angle: 0.0,
        }
    }

    /// Mono white preset
    pub const fn mono_white() -> Self {
        Self {
            primary: [1.0, 1.0, 1.0, 1.0],
            secondary: [0.7, 0.7, 0.7, 1.0],
            accent: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.0, 0.0, 1.0],
            mode: ColorMode::Solid,
            gradient_angle: 0.0,
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::neon_cyan()
    }
}

/// Color mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorMode {
    /// Solid color
    Solid        = 0,
    /// Gradient
    #[default]
    Gradient     = 1,
    /// Frequency-based
    Frequency    = 2,
    /// Amplitude-based
    Amplitude    = 3,
    /// Beat-reactive
    BeatReactive = 4,
}

// ============================================================================
// Frequency Bands
// ============================================================================

/// Frequency band
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FrequencyBand {
    /// Low frequency (Hz)
    pub low_freq: f32,
    /// High frequency (Hz)
    pub high_freq: f32,
    /// Current amplitude (0-1)
    pub amplitude: f32,
    /// Peak amplitude
    pub peak: f32,
    /// Average amplitude
    pub average: f32,
}

impl FrequencyBand {
    /// Creates new band
    pub const fn new(low: f32, high: f32) -> Self {
        Self {
            low_freq: low,
            high_freq: high,
            amplitude: 0.0,
            peak: 0.0,
            average: 0.0,
        }
    }

    /// Sub bass (20-60 Hz)
    pub const fn sub_bass() -> Self {
        Self::new(20.0, 60.0)
    }

    /// Bass (60-250 Hz)
    pub const fn bass() -> Self {
        Self::new(60.0, 250.0)
    }

    /// Low mids (250-500 Hz)
    pub const fn low_mids() -> Self {
        Self::new(250.0, 500.0)
    }

    /// Mids (500-2000 Hz)
    pub const fn mids() -> Self {
        Self::new(500.0, 2000.0)
    }

    /// High mids (2000-4000 Hz)
    pub const fn high_mids() -> Self {
        Self::new(2000.0, 4000.0)
    }

    /// Presence (4000-6000 Hz)
    pub const fn presence() -> Self {
        Self::new(4000.0, 6000.0)
    }

    /// Brilliance (6000-20000 Hz)
    pub const fn brilliance() -> Self {
        Self::new(6000.0, 20000.0)
    }
}

impl Default for FrequencyBand {
    fn default() -> Self {
        Self::new(20.0, 20000.0)
    }
}

/// Standard frequency bands
#[derive(Clone, Debug)]
pub struct StandardBands {
    /// Bands
    pub bands: Vec<FrequencyBand>,
}

impl StandardBands {
    /// Creates 8-band equalizer bands
    pub fn eq_8() -> Self {
        Self {
            bands: vec![
                FrequencyBand::new(20.0, 60.0),
                FrequencyBand::new(60.0, 150.0),
                FrequencyBand::new(150.0, 400.0),
                FrequencyBand::new(400.0, 1000.0),
                FrequencyBand::new(1000.0, 2500.0),
                FrequencyBand::new(2500.0, 6000.0),
                FrequencyBand::new(6000.0, 12000.0),
                FrequencyBand::new(12000.0, 20000.0),
            ],
        }
    }

    /// Creates octave bands
    pub fn octave() -> Self {
        Self {
            bands: vec![
                FrequencyBand::new(31.25, 62.5),
                FrequencyBand::new(62.5, 125.0),
                FrequencyBand::new(125.0, 250.0),
                FrequencyBand::new(250.0, 500.0),
                FrequencyBand::new(500.0, 1000.0),
                FrequencyBand::new(1000.0, 2000.0),
                FrequencyBand::new(2000.0, 4000.0),
                FrequencyBand::new(4000.0, 8000.0),
                FrequencyBand::new(8000.0, 16000.0),
            ],
        }
    }
}

impl Default for StandardBands {
    fn default() -> Self {
        Self::eq_8()
    }
}

// ============================================================================
// Beat Detection
// ============================================================================

/// Beat detection settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BeatDetectSettings {
    /// Detection threshold
    pub threshold: f32,
    /// Sensitivity
    pub sensitivity: f32,
    /// Decay rate
    pub decay: f32,
    /// Minimum beat interval (seconds)
    pub min_interval: f32,
    /// Energy history size
    pub history_size: u32,
    /// Detection mode
    pub mode: BeatDetectMode,
}

impl BeatDetectSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            threshold: 1.5,
            sensitivity: 0.5,
            decay: 0.9,
            min_interval: 0.1,
            history_size: 43,
            mode: BeatDetectMode::Energy,
        }
    }

    /// With threshold
    pub const fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// With sensitivity
    pub const fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    /// Electronic music preset
    pub const fn electronic() -> Self {
        Self {
            threshold: 1.3,
            sensitivity: 0.7,
            decay: 0.85,
            min_interval: 0.08,
            history_size: 50,
            mode: BeatDetectMode::BassEnergy,
        }
    }

    /// Rock music preset
    pub const fn rock() -> Self {
        Self {
            threshold: 1.6,
            sensitivity: 0.5,
            decay: 0.9,
            min_interval: 0.15,
            history_size: 43,
            mode: BeatDetectMode::Energy,
        }
    }
}

impl Default for BeatDetectSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Beat detection mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BeatDetectMode {
    /// Energy-based
    #[default]
    Energy       = 0,
    /// Bass energy
    BassEnergy   = 1,
    /// Spectral flux
    SpectralFlux = 2,
    /// Onset detection
    Onset        = 3,
}

/// Beat info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BeatInfo {
    /// Is beat detected
    pub is_beat: bool,
    /// Beat intensity (0-1)
    pub intensity: f32,
    /// Time since last beat (seconds)
    pub time_since_beat: f32,
    /// Estimated BPM
    pub bpm: f32,
    /// Beat confidence (0-1)
    pub confidence: f32,
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU audio data
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuAudioData {
    /// Time
    pub time: f32,
    /// Beat
    pub beat: f32,
    /// Bass level
    pub bass: f32,
    /// Mid level
    pub mids: f32,
    /// High level
    pub highs: f32,
    /// Overall level
    pub level: f32,
    /// BPM
    pub bpm: f32,
    /// Flags
    pub flags: u32,
}

impl Default for GpuAudioData {
    fn default() -> Self {
        Self {
            time: 0.0,
            beat: 0.0,
            bass: 0.0,
            mids: 0.0,
            highs: 0.0,
            level: 0.0,
            bpm: 120.0,
            flags: 0,
        }
    }
}

/// GPU spectrum data (per-band)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuSpectrumBand {
    /// Current amplitude
    pub amplitude: f32,
    /// Peak amplitude
    pub peak: f32,
    /// Smoothed amplitude
    pub smoothed: f32,
    /// Velocity
    pub velocity: f32,
}

/// GPU visualizer constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuVisualizerConstants {
    /// Time
    pub time: f32,
    /// Delta time
    pub dt: f32,
    /// Band count
    pub band_count: u32,
    /// FFT size
    pub fft_size: u32,
    /// Amplitude scale
    pub amplitude_scale: f32,
    /// Glow intensity
    pub glow: f32,
    /// Beat intensity
    pub beat: f32,
    /// BPM
    pub bpm: f32,
    /// Primary color
    pub primary_color: [f32; 4],
    /// Secondary color
    pub secondary_color: [f32; 4],
    /// Visualizer type
    pub viz_type: u32,
    /// Color mode
    pub color_mode: u32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad: f32,
}

impl Default for GpuVisualizerConstants {
    fn default() -> Self {
        Self {
            time: 0.0,
            dt: 0.016,
            band_count: 32,
            fft_size: 2048,
            amplitude_scale: 1.0,
            glow: 0.5,
            beat: 0.0,
            bpm: 120.0,
            primary_color: [0.0, 1.0, 1.0, 1.0],
            secondary_color: [1.0, 0.0, 1.0, 1.0],
            viz_type: 0,
            color_mode: 0,
            flags: 0,
            _pad: 0.0,
        }
    }
}

/// GPU waveform vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuWaveformVertex {
    /// Position
    pub position: [f32; 2],
    /// Amplitude
    pub amplitude: f32,
    /// Sample index
    pub index: f32,
}

/// GPU spectrum bar
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuSpectrumBar {
    /// Position
    pub position: [f32; 2],
    /// Height
    pub height: f32,
    /// Peak height
    pub peak_height: f32,
    /// Color
    pub color: [f32; 4],
}

// ============================================================================
// Audio Viz Statistics
// ============================================================================

/// Audio viz statistics
#[derive(Clone, Debug, Default)]
pub struct GpuAudioVizStats {
    /// Active visualizers
    pub active_visualizers: u32,
    /// FFT compute time (ms)
    pub fft_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Current BPM
    pub current_bpm: f32,
    /// Beat count
    pub beat_count: u64,
    /// Peak level
    pub peak_level: f32,
    /// RMS level
    pub rms_level: f32,
    /// Samples processed
    pub samples_processed: u64,
}

impl GpuAudioVizStats {
    /// Total time (ms)
    pub fn total_time_ms(&self) -> f32 {
        self.fft_time_ms + self.render_time_ms
    }

    /// Level in dB
    pub fn level_db(&self) -> f32 {
        if self.rms_level > 0.0 {
            20.0 * self.rms_level.log10()
        } else {
            -100.0
        }
    }

    /// Peak in dB
    pub fn peak_db(&self) -> f32 {
        if self.peak_level > 0.0 {
            20.0 * self.peak_level.log10()
        } else {
            -100.0
        }
    }
}
