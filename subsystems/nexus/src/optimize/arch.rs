//! Architecture detection and CPU features

#![allow(dead_code)]

// ============================================================================
// ARCHITECTURE
// ============================================================================

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// x86-64
    X86_64,
    /// ARM64 / AArch64
    Aarch64,
    /// RISC-V 64-bit
    Riscv64,
    /// Unknown architecture
    Unknown,
}

impl Architecture {
    /// Detect current architecture
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }
        #[cfg(target_arch = "riscv64")]
        {
            Self::Riscv64
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            Self::Unknown
        }
    }

    /// Get architecture name
    pub fn name(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::Riscv64 => "riscv64",
            Self::Unknown => "unknown",
        }
    }

    /// Get cache line size
    pub fn cache_line_size(&self) -> usize {
        match self {
            Self::X86_64 => 64,
            Self::Aarch64 => 64, // Can be 128 on some implementations
            Self::Riscv64 => 64,
            Self::Unknown => 64,
        }
    }

    /// Get typical page size
    pub fn page_size(&self) -> usize {
        4096 // 4KB on most architectures
    }
}

// ============================================================================
// CPU FEATURES
// ============================================================================

/// CPU feature flags
#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    /// x86: SSE support
    pub sse: bool,
    /// x86: SSE2 support
    pub sse2: bool,
    /// x86: SSE4.1 support
    pub sse4_1: bool,
    /// x86: AVX support
    pub avx: bool,
    /// x86: AVX2 support
    pub avx2: bool,
    /// x86: AVX-512 support
    pub avx512: bool,
    /// ARM: NEON support
    pub neon: bool,
    /// ARM: SVE support
    pub sve: bool,
    /// RISC-V: Vector extension
    pub rvv: bool,
    /// Atomic operations support
    pub atomics: bool,
    /// Hardware transactional memory
    pub htm: bool,
    /// Number of cores
    pub cores: u32,
    /// Number of threads per core
    pub threads_per_core: u32,
    /// L1 data cache size (bytes)
    pub l1d_cache: u32,
    /// L1 instruction cache size (bytes)
    pub l1i_cache: u32,
    /// L2 cache size (bytes)
    pub l2_cache: u32,
    /// L3 cache size (bytes)
    pub l3_cache: u32,
}

impl CpuFeatures {
    /// Detect CPU features
    pub fn detect() -> Self {
        let mut features = Self::default();
        features.atomics = true; // Assume atomics on all 64-bit platforms

        // In a real kernel, we'd use CPUID (x86), feature registers (ARM), etc.
        // Here we just set reasonable defaults

        #[cfg(target_arch = "x86_64")]
        {
            features.sse = true;
            features.sse2 = true;
            // Other features would be detected via CPUID
        }

        #[cfg(target_arch = "aarch64")]
        {
            features.neon = true; // NEON is mandatory on AArch64
        }

        features
    }

    /// Has vector extensions?
    pub fn has_vectors(&self) -> bool {
        self.avx || self.avx2 || self.avx512 || self.neon || self.sve || self.rvv
    }

    /// Best vector width (in bytes)
    pub fn best_vector_width(&self) -> usize {
        if self.avx512 {
            64
        } else if self.avx2 || self.avx {
            32
        } else if self.sve {
            64 // Variable, but often 64
        } else if self.neon || self.sse2 {
            16
        } else {
            8
        }
    }
}
