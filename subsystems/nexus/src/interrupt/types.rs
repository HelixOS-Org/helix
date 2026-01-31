//! Core interrupt types and enumerations
//!
//! This module defines fundamental types for interrupt handling including
//! IRQ identifiers, CPU identifiers, interrupt types, priorities, and delivery modes.

#![allow(dead_code)]

/// IRQ number type
pub type Irq = u32;

/// CPU identifier type
pub type CpuId = u32;

/// Type of interrupt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// Timer interrupt
    Timer,
    /// Inter-processor interrupt
    Ipi,
    /// Device interrupt
    Device,
    /// Error interrupt
    Error,
    /// Software interrupt
    Software,
    /// Non-maskable interrupt
    Nmi,
    /// Performance monitoring
    Pmu,
    /// Unknown type
    Unknown,
}

/// Interrupt priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Interrupt delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    /// Fixed delivery to specific CPU
    Fixed,
    /// Lowest priority among targets
    LowestPriority,
    /// System management interrupt
    Smi,
    /// Non-maskable interrupt
    Nmi,
    /// INIT signal
    Init,
    /// Start-up IPI
    Startup,
    /// External interrupt
    ExtInt,
}
