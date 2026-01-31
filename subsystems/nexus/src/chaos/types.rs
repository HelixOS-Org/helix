//! Fault type definitions

#![allow(dead_code)]

// ============================================================================
// FAULT TYPE
// ============================================================================

/// Type of fault to inject
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultType {
    /// Memory pressure / allocation failure
    Memory,
    /// CPU spike / starvation
    Cpu,
    /// I/O latency / failure
    Io,
    /// Network latency / failure
    Network,
    /// Latency injection
    Latency,
    /// Panic injection
    Panic,
    /// Hang / deadlock simulation
    Hang,
    /// Data corruption
    Corruption,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Clock skew
    ClockSkew,
}

impl FaultType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Memory => "Memory Fault",
            Self::Cpu => "CPU Fault",
            Self::Io => "I/O Fault",
            Self::Network => "Network Fault",
            Self::Latency => "Latency Injection",
            Self::Panic => "Panic Injection",
            Self::Hang => "Hang/Deadlock",
            Self::Corruption => "Data Corruption",
            Self::ResourceExhaustion => "Resource Exhaustion",
            Self::ClockSkew => "Clock Skew",
        }
    }

    /// Get severity (1-10)
    pub fn severity(&self) -> u8 {
        match self {
            Self::Latency => 3,
            Self::ClockSkew => 4,
            Self::Memory => 6,
            Self::Cpu => 5,
            Self::Io => 6,
            Self::Network => 5,
            Self::ResourceExhaustion => 7,
            Self::Corruption => 9,
            Self::Panic => 10,
            Self::Hang => 8,
        }
    }

    /// Is this fault potentially destructive?
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::Panic | Self::Corruption | Self::Hang)
    }
}
