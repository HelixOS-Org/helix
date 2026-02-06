//! Audio Support for Helix UEFI Bootloader
//!
//! This module provides audio output support for boot chimes, beep codes,
//! and audio feedback during the UEFI boot process.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Audio System Stack                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Application    │  Boot Chime  │  Beep Codes  │  Error Tones           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Codec Layer    │  WAV Parser  │  Tone Gen  │  Sample Rate Conv        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Protocol       │  HDA  │  AC97  │  USB Audio  │  PC Speaker           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  PCI  │  DMA  │  Interrupts  │  Timer                │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - HD Audio controller support
//! - AC'97 codec support
//! - PC Speaker beep codes
//! - WAV audio file playback
//! - Tone generation
//! - BEEP code patterns
//! - Boot chime sequences

use core::fmt;

// =============================================================================
// AUDIO CONSTANTS
// =============================================================================

/// Standard CD quality sample rate
pub const SAMPLE_RATE_44100: u32 = 44100;

/// DVD quality sample rate
pub const SAMPLE_RATE_48000: u32 = 48000;

/// Standard studio sample rate
pub const SAMPLE_RATE_96000: u32 = 96000;

/// Mono channel
pub const CHANNELS_MONO: u8 = 1;

/// Stereo channels
pub const CHANNELS_STEREO: u8 = 2;

/// 5.1 surround
pub const CHANNELS_5_1: u8 = 6;

/// 7.1 surround
pub const CHANNELS_7_1: u8 = 8;

/// 8-bit samples
pub const BITS_8: u8 = 8;

/// 16-bit samples
pub const BITS_16: u8 = 16;

/// 24-bit samples
pub const BITS_24: u8 = 24;

/// 32-bit samples
pub const BITS_32: u8 = 32;

/// PC Speaker I/O port (timer)
pub const PC_SPEAKER_TIMER_PORT: u16 = 0x42;

/// PC Speaker control port
pub const PC_SPEAKER_CONTROL_PORT: u16 = 0x43;

/// PC Speaker gate port
pub const PC_SPEAKER_GATE_PORT: u16 = 0x61;

/// PC Speaker clock frequency (1.193182 MHz)
pub const PC_SPEAKER_CLOCK_HZ: u32 = 1_193_182;

// =============================================================================
// AUDIO FORMAT
// =============================================================================

/// Audio sample format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// Unsigned 8-bit PCM
    U8,
    /// Signed 16-bit PCM, little-endian
    S16Le,
    /// Signed 16-bit PCM, big-endian
    S16Be,
    /// Signed 24-bit PCM, little-endian
    S24Le,
    /// Signed 24-bit PCM, big-endian
    S24Be,
    /// Signed 32-bit PCM, little-endian
    S32Le,
    /// Signed 32-bit PCM, big-endian
    S32Be,
    /// 32-bit float, little-endian
    F32Le,
    /// 32-bit float, big-endian
    F32Be,
    /// A-law compressed
    ALaw,
    /// μ-law compressed
    MuLaw,
}

impl SampleFormat {
    /// Get bytes per sample
    pub const fn bytes_per_sample(&self) -> usize {
        match self {
            SampleFormat::U8 | SampleFormat::ALaw | SampleFormat::MuLaw => 1,
            SampleFormat::S16Le | SampleFormat::S16Be => 2,
            SampleFormat::S24Le | SampleFormat::S24Be => 3,
            SampleFormat::S32Le
            | SampleFormat::S32Be
            | SampleFormat::F32Le
            | SampleFormat::F32Be => 4,
        }
    }

    /// Get bits per sample
    pub const fn bits_per_sample(&self) -> u8 {
        match self {
            SampleFormat::U8 | SampleFormat::ALaw | SampleFormat::MuLaw => 8,
            SampleFormat::S16Le | SampleFormat::S16Be => 16,
            SampleFormat::S24Le | SampleFormat::S24Be => 24,
            SampleFormat::S32Le
            | SampleFormat::S32Be
            | SampleFormat::F32Le
            | SampleFormat::F32Be => 32,
        }
    }

    /// Check if little-endian
    pub const fn is_little_endian(&self) -> bool {
        matches!(
            self,
            SampleFormat::U8
                | SampleFormat::S16Le
                | SampleFormat::S24Le
                | SampleFormat::S32Le
                | SampleFormat::F32Le
                | SampleFormat::ALaw
                | SampleFormat::MuLaw
        )
    }
}

/// Audio format descriptor
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Sample format
    pub format: SampleFormat,
}

impl AudioFormat {
    /// Create new audio format
    pub const fn new(sample_rate: u32, channels: u8, format: SampleFormat) -> Self {
        Self {
            sample_rate,
            channels,
            format,
        }
    }

    /// Create CD quality format (44.1kHz, stereo, 16-bit)
    pub const fn cd_quality() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_44100,
            channels: CHANNELS_STEREO,
            format: SampleFormat::S16Le,
        }
    }

    /// Create DVD quality format (48kHz, stereo, 16-bit)
    pub const fn dvd_quality() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_48000,
            channels: CHANNELS_STEREO,
            format: SampleFormat::S16Le,
        }
    }

    /// Get bytes per frame (all channels)
    pub const fn bytes_per_frame(&self) -> usize {
        self.format.bytes_per_sample() * (self.channels as usize)
    }

    /// Get bytes per second
    pub const fn bytes_per_second(&self) -> u32 {
        (self.bytes_per_frame() as u32) * self.sample_rate
    }

    /// Calculate buffer size for duration in milliseconds
    pub const fn buffer_size_for_ms(&self, ms: u32) -> usize {
        ((self.bytes_per_second() as u64 * ms as u64) / 1000) as usize
    }
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self::cd_quality()
    }
}

// =============================================================================
// WAV FILE FORMAT
// =============================================================================

/// RIFF chunk header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RiffChunk {
    /// Chunk ID ('RIFF')
    pub id: [u8; 4],
    /// Chunk size
    pub size: u32,
    /// Format ('WAVE')
    pub format: [u8; 4],
}

impl RiffChunk {
    /// Check if valid RIFF/WAVE header
    pub fn is_valid(&self) -> bool {
        &self.id == b"RIFF" && &self.format == b"WAVE"
    }
}

/// Format chunk header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FmtChunk {
    /// Chunk ID ('fmt ')
    pub id: [u8; 4],
    /// Chunk size
    pub size: u32,
    /// Audio format (1 = PCM)
    pub audio_format: u16,
    /// Number of channels
    pub num_channels: u16,
    /// Sample rate
    pub sample_rate: u32,
    /// Byte rate (sample_rate * num_channels * bits_per_sample / 8)
    pub byte_rate: u32,
    /// Block align (num_channels * bits_per_sample / 8)
    pub block_align: u16,
    /// Bits per sample
    pub bits_per_sample: u16,
}

impl FmtChunk {
    /// Check if valid fmt chunk
    pub fn is_valid(&self) -> bool {
        &self.id == b"fmt "
    }

    /// Check if PCM format
    pub fn is_pcm(&self) -> bool {
        self.audio_format == 1
    }

    /// Get sample format
    pub fn sample_format(&self) -> Option<SampleFormat> {
        match (self.bits_per_sample, self.audio_format) {
            (8, 1) => Some(SampleFormat::U8),
            (16, 1) => Some(SampleFormat::S16Le),
            (24, 1) => Some(SampleFormat::S24Le),
            (32, 1) => Some(SampleFormat::S32Le),
            (32, 3) => Some(SampleFormat::F32Le), // IEEE float
            _ => None,
        }
    }

    /// Convert to AudioFormat
    pub fn to_audio_format(&self) -> Option<AudioFormat> {
        self.sample_format().map(|format| AudioFormat {
            sample_rate: self.sample_rate,
            channels: self.num_channels as u8,
            format,
        })
    }
}

/// Data chunk header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DataChunk {
    /// Chunk ID ('data')
    pub id: [u8; 4],
    /// Chunk size
    pub size: u32,
}

impl DataChunk {
    /// Check if valid data chunk
    pub fn is_valid(&self) -> bool {
        &self.id == b"data"
    }
}

/// WAV file parser
pub struct WavParser<'a> {
    /// Raw data
    data: &'a [u8],
    /// Current position
    pos: usize,
}

impl<'a> WavParser<'a> {
    /// Create new parser
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Parse WAV file
    pub fn parse(&mut self) -> Option<WavInfo<'a>> {
        // Parse RIFF header
        if self.data.len() < 12 {
            return None;
        }

        let riff = unsafe { &*(self.data.as_ptr() as *const RiffChunk) };
        if !riff.is_valid() {
            return None;
        }
        self.pos = 12;

        // Find fmt chunk
        let mut format: Option<AudioFormat> = None;
        let mut audio_data: Option<&'a [u8]> = None;

        while self.pos + 8 <= self.data.len() {
            let chunk_id = &self.data[self.pos..self.pos + 4];
            let chunk_size = u32::from_le_bytes([
                self.data[self.pos + 4],
                self.data[self.pos + 5],
                self.data[self.pos + 6],
                self.data[self.pos + 7],
            ]);

            match chunk_id {
                b"fmt " => {
                    if self.pos + 8 + 16 > self.data.len() {
                        return None;
                    }
                    let fmt = unsafe { &*(self.data[self.pos..].as_ptr() as *const FmtChunk) };
                    format = fmt.to_audio_format();
                },
                b"data" => {
                    let data_start = self.pos + 8;
                    let data_end = data_start + chunk_size as usize;
                    if data_end > self.data.len() {
                        return None;
                    }
                    audio_data = Some(&self.data[data_start..data_end]);
                },
                _ => {},
            }

            self.pos += 8 + chunk_size as usize;
            // Align to 2-byte boundary
            if self.pos & 1 != 0 {
                self.pos += 1;
            }
        }

        match (format, audio_data) {
            (Some(f), Some(d)) => Some(WavInfo { format: f, data: d }),
            _ => None,
        }
    }
}

/// Parsed WAV file information
#[derive(Debug, Clone)]
pub struct WavInfo<'a> {
    /// Audio format
    pub format: AudioFormat,
    /// Audio sample data
    pub data: &'a [u8],
}

impl WavInfo<'_> {
    /// Get duration in milliseconds
    pub const fn duration_ms(&self) -> u32 {
        if self.format.bytes_per_second() == 0 {
            return 0;
        }
        ((self.data.len() as u64 * 1000) / (self.format.bytes_per_second() as u64)) as u32
    }

    /// Get number of samples
    pub const fn num_samples(&self) -> usize {
        self.data.len() / self.format.bytes_per_frame()
    }
}

// =============================================================================
// TONE GENERATION
// =============================================================================

/// Musical note frequencies (A4 = 440Hz)
pub mod notes {
    /// C0
    pub const C0: u32 = 16;
    /// C1
    pub const C1: u32 = 33;
    /// C2
    pub const C2: u32 = 65;
    /// C3
    pub const C3: u32 = 131;
    /// C4 (Middle C)
    pub const C4: u32 = 262;
    /// C5
    pub const C5: u32 = 523;
    /// D5
    pub const D5: u32 = 587;
    /// E5
    pub const E5: u32 = 659;
    /// C6
    pub const C6: u32 = 1047;
    /// C7
    pub const C7: u32 = 2093;
    /// C8
    pub const C8: u32 = 4186;

    /// D4
    pub const D4: u32 = 294;
    /// E4
    pub const E4: u32 = 330;
    /// F4
    pub const F4: u32 = 349;
    /// G4
    pub const G4: u32 = 392;
    /// A4 (Concert pitch)
    pub const A4: u32 = 440;
    /// B4
    pub const B4: u32 = 494;

    /// C#4
    pub const CS4: u32 = 277;
    /// D#4
    pub const DS4: u32 = 311;
    /// F#4
    pub const FS4: u32 = 370;
    /// G#4
    pub const GS4: u32 = 415;
    /// A#4
    pub const AS4: u32 = 466;

    /// Rest (no sound)
    pub const REST: u32 = 0;
}

/// Waveform type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    /// Sine wave
    Sine,
    /// Square wave
    Square,
    /// Triangle wave
    Triangle,
    /// Sawtooth wave
    Sawtooth,
    /// White noise
    Noise,
}

/// Tone definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tone {
    /// Frequency in Hz
    pub frequency: u32,
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Waveform type
    pub waveform: Waveform,
    /// Volume (0-100)
    pub volume: u8,
}

impl Tone {
    /// Create new tone
    pub const fn new(frequency: u32, duration_ms: u32) -> Self {
        Self {
            frequency,
            duration_ms,
            waveform: Waveform::Sine,
            volume: 80,
        }
    }

    /// Create rest (silence)
    pub const fn rest(duration_ms: u32) -> Self {
        Self {
            frequency: 0,
            duration_ms,
            waveform: Waveform::Sine,
            volume: 0,
        }
    }

    /// Set waveform
    pub const fn with_waveform(mut self, waveform: Waveform) -> Self {
        self.waveform = waveform;
        self
    }

    /// Set volume
    pub const fn with_volume(mut self, volume: u8) -> Self {
        self.volume = if volume > 100 { 100 } else { volume };
        self
    }

    /// Check if this is a rest
    pub const fn is_rest(&self) -> bool {
        self.frequency == 0 || self.volume == 0
    }
}

/// Tone generator
pub struct ToneGenerator {
    /// Sample rate
    sample_rate: u32,
    /// Phase accumulator
    phase: f32,
    /// Random state for noise
    noise_state: u32,
}

impl ToneGenerator {
    /// Create new tone generator
    pub const fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            noise_state: 0xDEAD_BEEF,
        }
    }

    /// Generate samples into buffer
    pub fn generate(&mut self, tone: &Tone, buffer: &mut [i16]) -> usize {
        if tone.is_rest() {
            // Fill with silence
            for sample in buffer.iter_mut() {
                *sample = 0;
            }
            return buffer.len();
        }

        let num_samples = ((self.sample_rate as u64 * tone.duration_ms as u64) / 1000) as usize;
        let samples_to_generate = buffer.len().min(num_samples);
        let phase_increment = (tone.frequency as f32) / (self.sample_rate as f32);
        let volume_scale = (tone.volume as f32) / 100.0;

        for i in 0..samples_to_generate {
            let sample = match tone.waveform {
                Waveform::Sine => self.sine_sample(),
                Waveform::Square => self.square_sample(),
                Waveform::Triangle => self.triangle_sample(),
                Waveform::Sawtooth => self.sawtooth_sample(),
                Waveform::Noise => self.noise_sample(),
            };

            buffer[i] = ((sample * volume_scale) * 32767.0) as i16;
            self.phase += phase_increment;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

        samples_to_generate
    }

    /// Generate sine wave sample (-1.0 to 1.0)
    fn sine_sample(&self) -> f32 {
        // Approximate sine using polynomial
        let x = self.phase * 2.0 - 1.0; // -1 to 1
        let x2 = x * x;
        // Taylor series approximation
        x * (3.141_592_65 - x2 * (5.167_712_78 - x2 * 2.550_164_03))
    }

    /// Generate square wave sample
    fn square_sample(&self) -> f32 {
        if self.phase < 0.5 {
            1.0
        } else {
            -1.0
        }
    }

    /// Generate triangle wave sample
    fn triangle_sample(&self) -> f32 {
        if self.phase < 0.5 {
            4.0 * self.phase - 1.0
        } else {
            3.0 - 4.0 * self.phase
        }
    }

    /// Generate sawtooth wave sample
    fn sawtooth_sample(&self) -> f32 {
        2.0 * self.phase - 1.0
    }

    /// Generate noise sample (LFSR)
    fn noise_sample(&mut self) -> f32 {
        // Linear feedback shift register
        let bit = ((self.noise_state >> 0)
            ^ (self.noise_state >> 2)
            ^ (self.noise_state >> 3)
            ^ (self.noise_state >> 5))
            & 1;
        self.noise_state = (self.noise_state >> 1) | (bit << 15);
        ((self.noise_state & 0xFFFF) as f32 / 32768.0) - 1.0
    }

    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// =============================================================================
// BEEP CODES
// =============================================================================

/// Beep code patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeepCode {
    /// Success (short beep)
    Success,
    /// Warning (two short beeps)
    Warning,
    /// Error (long beep)
    Error,
    /// Fatal error (continuous beep)
    Fatal,
    /// Memory error (3 short)
    MemoryError,
    /// Video error (1 long, 2 short)
    VideoError,
    /// Keyboard error (1 long, 3 short)
    KeyboardError,
    /// Boot device error (1 long, 4 short)
    BootDeviceError,
    /// Custom pattern
    Custom(&'static [Tone]),
}

// Static tone patterns for beep codes
static BEEP_SUCCESS: [Tone; 1] = [Tone::new(notes::C5, 100)];
static BEEP_WARNING: [Tone; 3] = [
    Tone::new(notes::A4, 100),
    Tone::rest(100),
    Tone::new(notes::A4, 100),
];
static BEEP_ERROR: [Tone; 1] = [Tone::new(notes::C4, 500)];
static BEEP_FATAL: [Tone; 1] = [Tone::new(notes::C4, 2000)];
static BEEP_MEMORY_ERROR: [Tone; 5] = [
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
];
static BEEP_VIDEO_ERROR: [Tone; 5] = [
    Tone::new(notes::C4, 400),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
];
static BEEP_KEYBOARD_ERROR: [Tone; 7] = [
    Tone::new(notes::C4, 400),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
];
static BEEP_BOOT_DEVICE_ERROR: [Tone; 9] = [
    Tone::new(notes::C4, 400),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
    Tone::rest(100),
    Tone::new(notes::C5, 100),
];

impl BeepCode {
    /// Get tone sequence for beep code
    pub fn tones(&self) -> &'static [Tone] {
        match self {
            BeepCode::Success => &BEEP_SUCCESS,
            BeepCode::Warning => &BEEP_WARNING,
            BeepCode::Error => &BEEP_ERROR,
            BeepCode::Fatal => &BEEP_FATAL,
            BeepCode::MemoryError => &BEEP_MEMORY_ERROR,
            BeepCode::VideoError => &BEEP_VIDEO_ERROR,
            BeepCode::KeyboardError => &BEEP_KEYBOARD_ERROR,
            BeepCode::BootDeviceError => &BEEP_BOOT_DEVICE_ERROR,
            BeepCode::Custom(tones) => tones,
        }
    }

    /// Get total duration in milliseconds
    pub fn duration_ms(&self) -> u32 {
        self.tones().iter().map(|t| t.duration_ms).sum()
    }
}

/// Boot chime sequences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootChime {
    /// Classic startup chime
    Classic,
    /// Modern startup chime
    Modern,
    /// Minimal beep
    Minimal,
    /// Mac-style chime
    Mac,
    /// Custom sequence
    Custom(&'static [Tone]),
}

/// Static boot chime sequences
mod chime_sequences {
    use super::{notes, Tone, Waveform};

    pub static CLASSIC: &[Tone] = &[
        Tone::new(notes::C4, 100),
        Tone::rest(50),
        Tone::new(notes::E4, 100),
        Tone::rest(50),
        Tone::new(notes::G4, 100),
        Tone::rest(50),
        Tone::new(notes::C5, 200),
    ];

    pub static MODERN: &[Tone] = &[
        Tone {
            frequency: notes::G4,
            duration_ms: 80,
            waveform: Waveform::Triangle,
            volume: 80,
        },
        Tone::rest(30),
        Tone {
            frequency: notes::C5,
            duration_ms: 80,
            waveform: Waveform::Triangle,
            volume: 80,
        },
        Tone::rest(30),
        Tone {
            frequency: notes::E5,
            duration_ms: 150,
            waveform: Waveform::Triangle,
            volume: 80,
        },
    ];

    pub static MINIMAL: &[Tone] = &[Tone::new(notes::C5, 150)];

    pub static MAC: &[Tone] = &[
        Tone {
            frequency: notes::FS4,
            duration_ms: 80,
            waveform: Waveform::Sine,
            volume: 90,
        },
        Tone {
            frequency: notes::A4,
            duration_ms: 80,
            waveform: Waveform::Sine,
            volume: 85,
        },
        Tone {
            frequency: notes::CS4,
            duration_ms: 80,
            waveform: Waveform::Sine,
            volume: 80,
        },
        Tone {
            frequency: notes::E4,
            duration_ms: 80,
            waveform: Waveform::Sine,
            volume: 75,
        },
        Tone {
            frequency: notes::FS4,
            duration_ms: 300,
            waveform: Waveform::Sine,
            volume: 70,
        },
    ];
}

impl BootChime {
    /// Get tone sequence for boot chime
    pub fn tones(&self) -> &'static [Tone] {
        match self {
            BootChime::Classic => chime_sequences::CLASSIC,
            BootChime::Modern => chime_sequences::MODERN,
            BootChime::Minimal => chime_sequences::MINIMAL,
            BootChime::Mac => chime_sequences::MAC,
            BootChime::Custom(tones) => tones,
        }
    }

    /// Get total duration in milliseconds
    pub fn duration_ms(&self) -> u32 {
        self.tones().iter().map(|t| t.duration_ms).sum()
    }
}

// =============================================================================
// HD AUDIO CONTROLLER
// =============================================================================

/// HD Audio controller register offsets
pub mod hda_regs {
    /// Global Capabilities
    pub const GCAP: usize = 0x00;
    /// Minor Version
    pub const VMIN: usize = 0x02;
    /// Major Version
    pub const VMAJ: usize = 0x03;
    /// Output Payload Capability
    pub const OUTPAY: usize = 0x04;
    /// Input Payload Capability
    pub const INPAY: usize = 0x06;
    /// Global Control
    pub const GCTL: usize = 0x08;
    /// Wake Enable
    pub const WAKEEN: usize = 0x0C;
    /// State Change Status
    pub const STATESTS: usize = 0x0E;
    /// Global Status
    pub const GSTS: usize = 0x10;
    /// Output Stream Payload Capability
    pub const OUTSTRMPAY: usize = 0x18;
    /// Input Stream Payload Capability
    pub const INSTRMPAY: usize = 0x1A;
    /// Interrupt Control
    pub const INTCTL: usize = 0x20;
    /// Interrupt Status
    pub const INTSTS: usize = 0x24;
    /// Wall Clock Counter
    pub const WALLCLK: usize = 0x30;
    /// Stream Synchronization
    pub const SSYNC: usize = 0x38;
    /// CORB Lower Base Address
    pub const CORBLBASE: usize = 0x40;
    /// CORB Upper Base Address
    pub const CORBUBASE: usize = 0x44;
    /// CORB Write Pointer
    pub const CORBWP: usize = 0x48;
    /// CORB Read Pointer
    pub const CORBRP: usize = 0x4A;
    /// CORB Control
    pub const CORBCTL: usize = 0x4C;
    /// CORB Status
    pub const CORBSTS: usize = 0x4D;
    /// CORB Size
    pub const CORBSIZE: usize = 0x4E;
    /// RIRB Lower Base Address
    pub const RIRBLBASE: usize = 0x50;
    /// RIRB Upper Base Address
    pub const RIRBUBASE: usize = 0x54;
    /// RIRB Write Pointer
    pub const RIRBWP: usize = 0x58;
    /// RIRB Response Interrupt Count
    pub const RINTCNT: usize = 0x5A;
    /// RIRB Control
    pub const RIRBCTL: usize = 0x5C;
    /// RIRB Status
    pub const RIRBSTS: usize = 0x5D;
    /// RIRB Size
    pub const RIRBSIZE: usize = 0x5E;
    /// Immediate Command Output Interface
    pub const ICOI: usize = 0x60;
    /// Immediate Response Input Interface
    pub const IRII: usize = 0x64;
    /// Immediate Command Status
    pub const ICS: usize = 0x68;
    /// DMA Position Lower Base Address
    pub const DPLBASE: usize = 0x70;
    /// DMA Position Upper Base Address
    pub const DPUBASE: usize = 0x74;
    /// Stream Descriptor registers (base)
    pub const SD_BASE: usize = 0x80;
    /// Stream descriptor size
    pub const SD_SIZE: usize = 0x20;
}

/// Stream descriptor register offsets (relative to stream base)
pub mod sd_regs {
    /// Stream Descriptor Control
    pub const CTL: usize = 0x00;
    /// Stream Descriptor Status
    pub const STS: usize = 0x03;
    /// Link Position in Current Buffer
    pub const LPIB: usize = 0x04;
    /// Cyclic Buffer Length
    pub const CBL: usize = 0x08;
    /// Last Valid Index
    pub const LVI: usize = 0x0C;
    /// FIFO Size
    pub const FIFOS: usize = 0x10;
    /// Stream Format
    pub const FMT: usize = 0x12;
    /// Buffer Descriptor List Pointer - Lower
    pub const BDLPL: usize = 0x18;
    /// Buffer Descriptor List Pointer - Upper
    pub const BDLPU: usize = 0x1C;
}

/// HD Audio verb commands
pub mod hda_verb {
    /// Get Parameter
    pub const GET_PARAMETER: u32 = 0xF00;
    /// Get Connection Select
    pub const GET_CONN_SELECT: u32 = 0xF01;
    /// Set Connection Select
    pub const SET_CONN_SELECT: u32 = 0x701;
    /// Get Connection List Entry
    pub const GET_CONN_LIST: u32 = 0xF02;
    /// Get Processing State
    pub const GET_PROCESSING: u32 = 0xF03;
    /// Set Processing State
    pub const SET_PROCESSING: u32 = 0x703;
    /// Get Coefficient Index
    pub const GET_COEF_INDEX: u32 = 0xD;
    /// Set Coefficient Index
    pub const SET_COEF_INDEX: u32 = 0x5;
    /// Get Processing Coefficient
    pub const GET_PROC_COEF: u32 = 0xC;
    /// Set Processing Coefficient
    pub const SET_PROC_COEF: u32 = 0x4;
    /// Get Amplifier Gain/Mute
    pub const GET_AMP_GAIN: u32 = 0xB;
    /// Set Amplifier Gain/Mute
    pub const SET_AMP_GAIN: u32 = 0x3;
    /// Get Converter Format
    pub const GET_CONV_FMT: u32 = 0xA;
    /// Set Converter Format
    pub const SET_CONV_FMT: u32 = 0x2;
    /// Get Digital Converter Control 1
    pub const GET_DIGI_CVT_1: u32 = 0xF0D;
    /// Set Digital Converter Control 1
    pub const SET_DIGI_CVT_1: u32 = 0x70D;
    /// Get Digital Converter Control 2
    pub const GET_DIGI_CVT_2: u32 = 0xF0E;
    /// Set Digital Converter Control 2
    pub const SET_DIGI_CVT_2: u32 = 0x70E;
    /// Get Power State
    pub const GET_POWER: u32 = 0xF05;
    /// Set Power State
    pub const SET_POWER: u32 = 0x705;
    /// Get Converter Stream/Channel
    pub const GET_CONV_STREAM: u32 = 0xF06;
    /// Set Converter Stream/Channel
    pub const SET_CONV_STREAM: u32 = 0x706;
    /// Get Pin Sense
    pub const GET_PIN_SENSE: u32 = 0xF09;
    /// Execute Pin Sense
    pub const EXEC_PIN_SENSE: u32 = 0x709;
    /// Get EAPD/BTL Enable
    pub const GET_EAPD: u32 = 0xF0C;
    /// Set EAPD/BTL Enable
    pub const SET_EAPD: u32 = 0x70C;
    /// Get GPI Data
    pub const GET_GPI: u32 = 0xF10;
    /// Set GPI Wake/Unsolicited Enable Mask
    pub const SET_GPI_MASK: u32 = 0x710;
    /// Get GPO Data
    pub const GET_GPO: u32 = 0xF11;
    /// Set GPO Data
    pub const SET_GPO: u32 = 0x711;
    /// Get GPIO Data
    pub const GET_GPIO: u32 = 0xF15;
    /// Set GPIO Data
    pub const SET_GPIO: u32 = 0x715;
    /// Get GPIO Enable Mask
    pub const GET_GPIO_MASK: u32 = 0xF16;
    /// Set GPIO Enable Mask
    pub const SET_GPIO_MASK: u32 = 0x716;
    /// Get GPIO Direction
    pub const GET_GPIO_DIR: u32 = 0xF17;
    /// Set GPIO Direction
    pub const SET_GPIO_DIR: u32 = 0x717;
    /// Get GPIO Wake Enable Mask
    pub const GET_GPIO_WAKE: u32 = 0xF18;
    /// Set GPIO Wake Enable Mask
    pub const SET_GPIO_WAKE: u32 = 0x718;
    /// Get GPIO Unsolicited Enable Mask
    pub const GET_GPIO_UNSOL: u32 = 0xF19;
    /// Set GPIO Unsolicited Enable Mask
    pub const SET_GPIO_UNSOL: u32 = 0x719;
    /// Get GPIO Sticky Mask
    pub const GET_GPIO_STICKY: u32 = 0xF1A;
    /// Set GPIO Sticky Mask
    pub const SET_GPIO_STICKY: u32 = 0x71A;
    /// Get Beep Generation
    pub const GET_BEEP: u32 = 0xF0A;
    /// Set Beep Generation
    pub const SET_BEEP: u32 = 0x70A;
    /// Get Volume Knob
    pub const GET_VOLUME_KNOB: u32 = 0xF0F;
    /// Set Volume Knob
    pub const SET_VOLUME_KNOB: u32 = 0x70F;
    /// Get Pin Widget Control
    pub const GET_PIN_CTL: u32 = 0xF07;
    /// Set Pin Widget Control
    pub const SET_PIN_CTL: u32 = 0x707;
    /// Get Unsolicited Response
    pub const GET_UNSOL_RESP: u32 = 0xF08;
    /// Set Unsolicited Response
    pub const SET_UNSOL_RESP: u32 = 0x708;
    /// Get Pin Configuration Default
    pub const GET_CONFIG_DEFAULT: u32 = 0xF1C;
    /// Set Configuration Default 1
    pub const SET_CONFIG_DEFAULT_1: u32 = 0x71C;
    /// Set Configuration Default 2
    pub const SET_CONFIG_DEFAULT_2: u32 = 0x71D;
    /// Set Configuration Default 3
    pub const SET_CONFIG_DEFAULT_3: u32 = 0x71E;
    /// Set Configuration Default 4
    pub const SET_CONFIG_DEFAULT_4: u32 = 0x71F;
    /// Get Stripe Control
    pub const GET_STRIPE: u32 = 0xF24;
    /// Set Stripe Control
    pub const SET_STRIPE: u32 = 0x724;
    /// Function Reset
    pub const FUNC_RESET: u32 = 0x7FF;
}

/// HD Audio parameter IDs
pub mod hda_param {
    /// Vendor ID
    pub const VENDOR_ID: u8 = 0x00;
    /// Revision ID
    pub const REVISION_ID: u8 = 0x02;
    /// Subordinate Node Count
    pub const NODE_COUNT: u8 = 0x04;
    /// Function Group Type
    pub const FUNC_TYPE: u8 = 0x05;
    /// Audio Function Group Capabilities
    pub const AFG_CAP: u8 = 0x08;
    /// Audio Widget Capabilities
    pub const WIDGET_CAP: u8 = 0x09;
    /// Sample Size/Rate Capabilities
    pub const SAMPLE_CAP: u8 = 0x0A;
    /// Stream Formats
    pub const STREAM_FMT: u8 = 0x0B;
    /// Pin Capabilities
    pub const PIN_CAP: u8 = 0x0C;
    /// Input Amplifier Capabilities
    pub const IN_AMP_CAP: u8 = 0x0D;
    /// Output Amplifier Capabilities
    pub const OUT_AMP_CAP: u8 = 0x12;
    /// Connection List Length
    pub const CONN_LEN: u8 = 0x0E;
    /// Supported Power States
    pub const POWER_STATES: u8 = 0x0F;
    /// Processing Capabilities
    pub const PROC_CAP: u8 = 0x10;
    /// GPIO Count
    pub const GPIO_COUNT: u8 = 0x11;
    /// Volume Knob Capabilities
    pub const VOLUME_CAP: u8 = 0x13;
}

/// Buffer Descriptor List Entry (16 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HdaBdlEntry {
    /// Address (64-bit)
    pub address: u64,
    /// Length in bytes
    pub length: u32,
    /// Interrupt on Completion flag (bit 0)
    pub ioc: u32,
}

impl HdaBdlEntry {
    /// Create new BDL entry
    pub const fn new(address: u64, length: u32, ioc: bool) -> Self {
        Self {
            address,
            length,
            ioc: if ioc { 1 } else { 0 },
        }
    }
}

// =============================================================================
// AC97 CODEC
// =============================================================================

/// AC'97 codec register offsets
pub mod ac97_regs {
    /// Reset
    pub const RESET: u16 = 0x00;
    /// Master Volume
    pub const MASTER_VOL: u16 = 0x02;
    /// Aux Out Volume
    pub const AUX_OUT_VOL: u16 = 0x04;
    /// Mono Volume
    pub const MONO_VOL: u16 = 0x06;
    /// Master Tone
    pub const MASTER_TONE: u16 = 0x08;
    /// PC Beep Volume
    pub const PC_BEEP: u16 = 0x0A;
    /// Phone Volume
    pub const PHONE_VOL: u16 = 0x0C;
    /// Mic Volume
    pub const MIC_VOL: u16 = 0x0E;
    /// Line In Volume
    pub const LINE_IN_VOL: u16 = 0x10;
    /// CD Volume
    pub const CD_VOL: u16 = 0x12;
    /// Video Volume
    pub const VIDEO_VOL: u16 = 0x14;
    /// Aux In Volume
    pub const AUX_IN_VOL: u16 = 0x16;
    /// PCM Out Volume
    pub const PCM_OUT_VOL: u16 = 0x18;
    /// Record Select
    pub const RECORD_SELECT: u16 = 0x1A;
    /// Record Gain
    pub const RECORD_GAIN: u16 = 0x1C;
    /// Record Gain Mic
    pub const RECORD_GAIN_MIC: u16 = 0x1E;
    /// General Purpose
    pub const GENERAL_PURPOSE: u16 = 0x20;
    /// 3D Control
    pub const CONTROL_3D: u16 = 0x22;
    /// Powerdown Control/Status
    pub const POWERDOWN: u16 = 0x26;
    /// Extended Audio ID
    pub const EXT_AUDIO_ID: u16 = 0x28;
    /// Extended Audio Control/Status
    pub const EXT_AUDIO_CTL: u16 = 0x2A;
    /// PCM Front DAC Rate
    pub const PCM_FRONT_RATE: u16 = 0x2C;
    /// PCM Surround DAC Rate
    pub const PCM_SURROUND_RATE: u16 = 0x2E;
    /// PCM LFE DAC Rate
    pub const PCM_LFE_RATE: u16 = 0x30;
    /// PCM ADC Rate
    pub const PCM_ADC_RATE: u16 = 0x32;
    /// Mic ADC Rate
    pub const MIC_ADC_RATE: u16 = 0x34;
    /// Center/LFE Volume
    pub const CENTER_LFE_VOL: u16 = 0x36;
    /// Surround Volume
    pub const SURROUND_VOL: u16 = 0x38;
    /// S/PDIF Control
    pub const SPDIF_CTL: u16 = 0x3A;
    /// Vendor ID 1
    pub const VENDOR_ID1: u16 = 0x7C;
    /// Vendor ID 2
    pub const VENDOR_ID2: u16 = 0x7E;
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Audio error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    /// No audio device found
    NoDevice,
    /// Device not initialized
    NotInitialized,
    /// Invalid format
    InvalidFormat,
    /// Buffer too small
    BufferTooSmall,
    /// Timeout
    Timeout,
    /// Hardware error
    HardwareError,
    /// Codec error
    CodecError,
    /// DMA error
    DmaError,
    /// Invalid WAV file
    InvalidWavFile,
    /// Unsupported format
    UnsupportedFormat,
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::NoDevice => write!(f, "No audio device found"),
            AudioError::NotInitialized => write!(f, "Audio device not initialized"),
            AudioError::InvalidFormat => write!(f, "Invalid audio format"),
            AudioError::BufferTooSmall => write!(f, "Buffer too small"),
            AudioError::Timeout => write!(f, "Operation timeout"),
            AudioError::HardwareError => write!(f, "Hardware error"),
            AudioError::CodecError => write!(f, "Codec error"),
            AudioError::DmaError => write!(f, "DMA error"),
            AudioError::InvalidWavFile => write!(f, "Invalid WAV file"),
            AudioError::UnsupportedFormat => write!(f, "Unsupported audio format"),
        }
    }
}

// =============================================================================
// PC SPEAKER SUPPORT
// =============================================================================

/// PC Speaker frequency to timer divisor
pub const fn frequency_to_divisor(frequency: u32) -> u16 {
    if frequency == 0 {
        return 0;
    }
    (PC_SPEAKER_CLOCK_HZ / frequency) as u16
}

/// PC Speaker beep parameters
#[derive(Debug, Clone, Copy)]
pub struct PcSpeakerBeep {
    /// Frequency in Hz
    pub frequency: u32,
    /// Duration in milliseconds
    pub duration_ms: u32,
}

impl PcSpeakerBeep {
    /// Create new beep
    pub const fn new(frequency: u32, duration_ms: u32) -> Self {
        Self {
            frequency,
            duration_ms,
        }
    }

    /// Get timer divisor
    pub const fn divisor(&self) -> u16 {
        frequency_to_divisor(self.frequency)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format() {
        let format = AudioFormat::cd_quality();
        assert_eq!(format.sample_rate, 44100);
        assert_eq!(format.channels, 2);
        assert_eq!(format.bytes_per_frame(), 4);
        assert_eq!(format.bytes_per_second(), 176400);
    }

    #[test]
    fn test_sample_format() {
        assert_eq!(SampleFormat::U8.bytes_per_sample(), 1);
        assert_eq!(SampleFormat::S16Le.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::S24Le.bytes_per_sample(), 3);
        assert_eq!(SampleFormat::S32Le.bytes_per_sample(), 4);
    }

    #[test]
    fn test_tone() {
        let tone = Tone::new(440, 1000);
        assert_eq!(tone.frequency, 440);
        assert_eq!(tone.duration_ms, 1000);
        assert!(!tone.is_rest());

        let rest = Tone::rest(500);
        assert!(rest.is_rest());
    }

    #[test]
    fn test_beep_code_duration() {
        let success = BeepCode::Success;
        assert_eq!(success.duration_ms(), 100);

        let warning = BeepCode::Warning;
        assert_eq!(warning.duration_ms(), 300); // 100 + 100 + 100
    }

    #[test]
    fn test_frequency_to_divisor() {
        // 440 Hz should give approximately 2712
        let divisor = frequency_to_divisor(440);
        assert!(divisor > 2700 && divisor < 2720);

        // 1000 Hz should give approximately 1193
        let divisor = frequency_to_divisor(1000);
        assert!(divisor > 1190 && divisor < 1200);
    }

    #[test]
    fn test_hda_bdl_entry() {
        let entry = HdaBdlEntry::new(0x1000, 4096, true);
        assert_eq!(entry.address, 0x1000);
        assert_eq!(entry.length, 4096);
        assert_eq!(entry.ioc, 1);
    }

    #[test]
    fn test_wav_chunks() {
        let riff = RiffChunk {
            id: *b"RIFF",
            size: 1000,
            format: *b"WAVE",
        };
        assert!(riff.is_valid());

        let bad_riff = RiffChunk {
            id: *b"RIFF",
            size: 1000,
            format: *b"AVI ",
        };
        assert!(!bad_riff.is_valid());
    }
}
