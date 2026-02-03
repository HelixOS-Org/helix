//! Audio Types for Lumina
//!
//! This module provides audio rendering types for spatial audio
//! visualization and audio-reactive graphics.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Audio Handles
// ============================================================================

/// Audio source handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AudioSourceHandle(pub u64);

impl AudioSourceHandle {
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

impl Default for AudioSourceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Audio listener handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AudioListenerHandle(pub u64);

impl AudioListenerHandle {
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

impl Default for AudioListenerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Audio visualization handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AudioVisualizationHandle(pub u64);

impl AudioVisualizationHandle {
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

impl Default for AudioVisualizationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Audio Source
// ============================================================================

/// Audio source create info
#[derive(Clone, Debug)]
pub struct AudioSourceCreateInfo {
    /// Position
    pub position: [f32; 3],
    /// Direction
    pub direction: [f32; 3],
    /// Velocity
    pub velocity: [f32; 3],
    /// Volume (0-1)
    pub volume: f32,
    /// Pitch
    pub pitch: f32,
    /// Spatialization
    pub spatial: bool,
    /// Attenuation model
    pub attenuation: AttenuationModel,
    /// Inner angle (cone)
    pub inner_angle: f32,
    /// Outer angle (cone)
    pub outer_angle: f32,
    /// Outer volume
    pub outer_volume: f32,
}

impl AudioSourceCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0],
            volume: 1.0,
            pitch: 1.0,
            spatial: true,
            attenuation: AttenuationModel::InverseDistance,
            inner_angle: 360.0,
            outer_angle: 360.0,
            outer_volume: 0.0,
        }
    }

    /// 2D (non-spatial)
    pub fn source_2d() -> Self {
        Self {
            spatial: false,
            ..Self::new()
        }
    }

    /// 3D spatial
    pub fn source_3d(position: [f32; 3]) -> Self {
        Self {
            position,
            spatial: true,
            ..Self::new()
        }
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With volume
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// With cone
    pub fn with_cone(mut self, inner: f32, outer: f32, outer_volume: f32) -> Self {
        self.inner_angle = inner;
        self.outer_angle = outer;
        self.outer_volume = outer_volume;
        self
    }

    /// With attenuation
    pub fn with_attenuation(mut self, model: AttenuationModel) -> Self {
        self.attenuation = model;
        self
    }
}

impl Default for AudioSourceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Attenuation model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AttenuationModel {
    /// No attenuation
    None = 0,
    /// Inverse distance
    #[default]
    InverseDistance = 1,
    /// Inverse distance clamped
    InverseDistanceClamped = 2,
    /// Linear distance
    LinearDistance = 3,
    /// Linear distance clamped
    LinearDistanceClamped = 4,
    /// Exponential distance
    ExponentialDistance = 5,
    /// Exponential distance clamped
    ExponentialDistanceClamped = 6,
}

impl AttenuationModel {
    /// Calculate attenuation at distance
    pub fn calculate(&self, distance: f32, ref_distance: f32, max_distance: f32, rolloff: f32) -> f32 {
        match self {
            Self::None => 1.0,
            Self::InverseDistance => {
                ref_distance / (ref_distance + rolloff * (distance - ref_distance))
            }
            Self::InverseDistanceClamped => {
                let d = distance.clamp(ref_distance, max_distance);
                ref_distance / (ref_distance + rolloff * (d - ref_distance))
            }
            Self::LinearDistance => {
                1.0 - rolloff * (distance - ref_distance) / (max_distance - ref_distance)
            }
            Self::LinearDistanceClamped => {
                let d = distance.clamp(ref_distance, max_distance);
                1.0 - rolloff * (d - ref_distance) / (max_distance - ref_distance)
            }
            Self::ExponentialDistance => {
                (distance / ref_distance).powf(-rolloff)
            }
            Self::ExponentialDistanceClamped => {
                let d = distance.clamp(ref_distance, max_distance);
                (d / ref_distance).powf(-rolloff)
            }
        }
    }
}

/// Audio attenuation settings
#[derive(Clone, Copy, Debug)]
pub struct AttenuationSettings {
    /// Model
    pub model: AttenuationModel,
    /// Reference distance
    pub ref_distance: f32,
    /// Max distance
    pub max_distance: f32,
    /// Rolloff factor
    pub rolloff: f32,
}

impl AttenuationSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            model: AttenuationModel::InverseDistanceClamped,
            ref_distance: 1.0,
            max_distance: 100.0,
            rolloff: 1.0,
        }
    }

    /// Small room
    pub fn small_room() -> Self {
        Self {
            ref_distance: 0.5,
            max_distance: 10.0,
            rolloff: 1.5,
            ..Self::new()
        }
    }

    /// Large room
    pub fn large_room() -> Self {
        Self {
            ref_distance: 2.0,
            max_distance: 50.0,
            rolloff: 1.0,
            ..Self::new()
        }
    }

    /// Outdoor
    pub fn outdoor() -> Self {
        Self {
            ref_distance: 5.0,
            max_distance: 200.0,
            rolloff: 0.5,
            ..Self::new()
        }
    }
}

impl Default for AttenuationSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Audio Listener
// ============================================================================

/// Audio listener create info
#[derive(Clone, Debug)]
pub struct AudioListenerCreateInfo {
    /// Position
    pub position: [f32; 3],
    /// Forward direction
    pub forward: [f32; 3],
    /// Up direction
    pub up: [f32; 3],
    /// Velocity
    pub velocity: [f32; 3],
}

impl AudioListenerCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
        }
    }

    /// At position
    pub fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    /// With orientation
    pub fn with_orientation(mut self, forward: [f32; 3], up: [f32; 3]) -> Self {
        self.forward = forward;
        self.up = up;
        self
    }
}

impl Default for AudioListenerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Audio Visualization
// ============================================================================

/// Audio visualization create info
#[derive(Clone, Debug)]
pub struct AudioVisualizationCreateInfo {
    /// Visualization type
    pub viz_type: AudioVisualizationType,
    /// FFT size (power of 2)
    pub fft_size: u32,
    /// Smoothing (0-1)
    pub smoothing: f32,
    /// Min frequency (Hz)
    pub min_frequency: f32,
    /// Max frequency (Hz)
    pub max_frequency: f32,
}

impl AudioVisualizationCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            viz_type: AudioVisualizationType::Spectrum,
            fft_size: 1024,
            smoothing: 0.8,
            min_frequency: 20.0,
            max_frequency: 20000.0,
        }
    }

    /// Spectrum analyzer
    pub fn spectrum() -> Self {
        Self {
            viz_type: AudioVisualizationType::Spectrum,
            ..Self::new()
        }
    }

    /// Waveform
    pub fn waveform() -> Self {
        Self {
            viz_type: AudioVisualizationType::Waveform,
            ..Self::new()
        }
    }

    /// With FFT size
    pub fn with_fft_size(mut self, size: u32) -> Self {
        self.fft_size = size.next_power_of_two();
        self
    }

    /// With frequency range
    pub fn with_frequency_range(mut self, min: f32, max: f32) -> Self {
        self.min_frequency = min;
        self.max_frequency = max;
        self
    }
}

impl Default for AudioVisualizationCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio visualization type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AudioVisualizationType {
    /// Spectrum (frequency bars)
    #[default]
    Spectrum = 0,
    /// Waveform (time domain)
    Waveform = 1,
    /// Circular spectrum
    CircularSpectrum = 2,
    /// VU meter
    VuMeter = 3,
    /// Oscilloscope
    Oscilloscope = 4,
    /// Spectrogram
    Spectrogram = 5,
}

/// Audio spectrum data
#[derive(Clone, Debug)]
pub struct AudioSpectrumData {
    /// Frequency bins
    pub bins: Vec<f32>,
    /// Peak values
    pub peaks: Vec<f32>,
    /// Sample rate
    pub sample_rate: u32,
    /// FFT size
    pub fft_size: u32,
}

impl AudioSpectrumData {
    /// Creates data
    pub fn new(fft_size: u32, sample_rate: u32) -> Self {
        let bin_count = fft_size / 2;
        Self {
            bins: Vec::from_iter(core::iter::repeat(0.0).take(bin_count as usize)),
            peaks: Vec::from_iter(core::iter::repeat(0.0).take(bin_count as usize)),
            sample_rate,
            fft_size,
        }
    }

    /// Bin count
    pub fn bin_count(&self) -> usize {
        self.bins.len()
    }

    /// Frequency of bin
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.sample_rate as f32 / self.fft_size as f32
    }

    /// Bin for frequency
    pub fn frequency_bin(&self, freq: f32) -> usize {
        (freq * self.fft_size as f32 / self.sample_rate as f32) as usize
    }

    /// Average in frequency range
    pub fn average_range(&self, min_freq: f32, max_freq: f32) -> f32 {
        let min_bin = self.frequency_bin(min_freq);
        let max_bin = self.frequency_bin(max_freq).min(self.bins.len());
        if max_bin <= min_bin {
            return 0.0;
        }

        let sum: f32 = self.bins[min_bin..max_bin].iter().sum();
        sum / (max_bin - min_bin) as f32
    }

    /// Bass (20-250 Hz)
    pub fn bass(&self) -> f32 {
        self.average_range(20.0, 250.0)
    }

    /// Mids (250-4000 Hz)
    pub fn mids(&self) -> f32 {
        self.average_range(250.0, 4000.0)
    }

    /// Highs (4000-20000 Hz)
    pub fn highs(&self) -> f32 {
        self.average_range(4000.0, 20000.0)
    }
}

impl Default for AudioSpectrumData {
    fn default() -> Self {
        Self::new(1024, 44100)
    }
}

// ============================================================================
// Audio-Reactive Graphics
// ============================================================================

/// Audio-reactive settings
#[derive(Clone, Debug)]
pub struct AudioReactiveSettings {
    /// Enable
    pub enabled: bool,
    /// Sensitivity
    pub sensitivity: f32,
    /// Smoothing
    pub smoothing: f32,
    /// Beat detection
    pub beat_detection: BeatDetectionSettings,
    /// Mappings
    pub mappings: Vec<AudioMapping>,
}

impl AudioReactiveSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            sensitivity: 1.0,
            smoothing: 0.8,
            beat_detection: BeatDetectionSettings::default(),
            mappings: Vec::new(),
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::new()
        }
    }

    /// With sensitivity
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    /// With mapping
    pub fn with_mapping(mut self, mapping: AudioMapping) -> Self {
        self.mappings.push(mapping);
        self
    }
}

impl Default for AudioReactiveSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Beat detection settings
#[derive(Clone, Copy, Debug)]
pub struct BeatDetectionSettings {
    /// Enable
    pub enabled: bool,
    /// Threshold
    pub threshold: f32,
    /// Cooldown (seconds)
    pub cooldown: f32,
    /// Frequency band
    pub band: FrequencyBand,
}

impl BeatDetectionSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            threshold: 1.5,
            cooldown: 0.1,
            band: FrequencyBand::Bass,
        }
    }
}

impl Default for BeatDetectionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Frequency band
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FrequencyBand {
    /// Bass (20-250 Hz)
    #[default]
    Bass = 0,
    /// Low mids (250-500 Hz)
    LowMids = 1,
    /// Mids (500-2000 Hz)
    Mids = 2,
    /// High mids (2000-4000 Hz)
    HighMids = 3,
    /// Highs (4000-20000 Hz)
    Highs = 4,
    /// Full range
    FullRange = 5,
}

impl FrequencyBand {
    /// Frequency range (Hz)
    pub const fn range(&self) -> (f32, f32) {
        match self {
            Self::Bass => (20.0, 250.0),
            Self::LowMids => (250.0, 500.0),
            Self::Mids => (500.0, 2000.0),
            Self::HighMids => (2000.0, 4000.0),
            Self::Highs => (4000.0, 20000.0),
            Self::FullRange => (20.0, 20000.0),
        }
    }
}

/// Audio mapping
#[derive(Clone, Debug)]
pub struct AudioMapping {
    /// Source band
    pub band: FrequencyBand,
    /// Target property
    pub target: AudioTarget,
    /// Minimum value
    pub min_value: f32,
    /// Maximum value
    pub max_value: f32,
    /// Smoothing
    pub smoothing: f32,
}

impl AudioMapping {
    /// Creates mapping
    pub fn new(band: FrequencyBand, target: AudioTarget) -> Self {
        Self {
            band,
            target,
            min_value: 0.0,
            max_value: 1.0,
            smoothing: 0.8,
        }
    }

    /// Bass to bloom
    pub fn bass_to_bloom() -> Self {
        Self::new(FrequencyBand::Bass, AudioTarget::BloomIntensity)
    }

    /// Mids to emission
    pub fn mids_to_emission() -> Self {
        Self::new(FrequencyBand::Mids, AudioTarget::EmissionIntensity)
    }

    /// With range
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }
}

/// Audio target property
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AudioTarget {
    /// Bloom intensity
    #[default]
    BloomIntensity = 0,
    /// Emission intensity
    EmissionIntensity = 1,
    /// Scale
    Scale = 2,
    /// Color hue
    ColorHue = 3,
    /// Camera shake
    CameraShake = 4,
    /// Custom float
    CustomFloat = 5,
}

// ============================================================================
// Spatial Audio GPU Data
// ============================================================================

/// Audio source GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AudioSourceGpu {
    /// Position
    pub position: [f32; 3],
    /// Volume
    pub volume: f32,
    /// Direction
    pub direction: [f32; 3],
    /// Range
    pub range: f32,
    /// Cone angles (inner, outer)
    pub cone: [f32; 2],
    /// Outer volume
    pub outer_volume: f32,
    /// Flags
    pub flags: u32,
}

/// Audio listener GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AudioListenerGpu {
    /// Position
    pub position: [f32; 3],
    /// Padding
    pub _padding0: f32,
    /// Forward
    pub forward: [f32; 3],
    /// Padding
    pub _padding1: f32,
    /// Up
    pub up: [f32; 3],
    /// Padding
    pub _padding2: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Audio visualization statistics
#[derive(Clone, Debug, Default)]
pub struct AudioVisualizationStats {
    /// Source count
    pub source_count: u32,
    /// Active sources
    pub active_sources: u32,
    /// Beat detected this frame
    pub beat_detected: bool,
    /// Current bass level
    pub bass_level: f32,
    /// Current mids level
    pub mids_level: f32,
    /// Current highs level
    pub highs_level: f32,
}
