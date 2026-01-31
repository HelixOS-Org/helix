//! Base Address Register (BAR) types.

// ============================================================================
// BAR (BASE ADDRESS REGISTER)
// ============================================================================

/// BAR type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarType {
    /// Memory 32-bit
    Memory32,
    /// Memory 64-bit
    Memory64,
    /// I/O
    Io,
    /// Not used
    None,
}

impl BarType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Memory32 => "mem32",
            Self::Memory64 => "mem64",
            Self::Io => "io",
            Self::None => "none",
        }
    }

    /// Is memory BAR
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory32 | Self::Memory64)
    }
}

/// BAR flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarFlags {
    /// Prefetchable
    pub prefetchable: bool,
    /// Is 64-bit (uses next BAR too)
    pub is_64bit: bool,
    /// Is I/O
    pub is_io: bool,
}

/// Base Address Register
#[derive(Debug, Clone)]
pub struct Bar {
    /// BAR index (0-5)
    pub index: u8,
    /// BAR type
    pub bar_type: BarType,
    /// Flags
    pub flags: BarFlags,
    /// Base address
    pub base: u64,
    /// Size in bytes
    pub size: u64,
    /// Is assigned
    pub assigned: bool,
}

impl Bar {
    /// Create new BAR
    pub fn new(index: u8, bar_type: BarType, base: u64, size: u64) -> Self {
        Self {
            index,
            bar_type,
            flags: BarFlags::default(),
            base,
            size,
            assigned: base != 0,
        }
    }

    /// End address
    pub fn end(&self) -> u64 {
        self.base + self.size
    }

    /// Is valid
    pub fn is_valid(&self) -> bool {
        self.size > 0
    }
}
