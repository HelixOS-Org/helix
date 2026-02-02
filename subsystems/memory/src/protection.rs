//! # Memory Protection

use bitflags::bitflags;
use helix_hal::VirtAddr;

bitflags! {
    /// Memory protection flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ProtectionFlags: u32 {
        /// No access
        const NONE = 0;
        /// Read access
        const READ = 1 << 0;
        /// Write access
        const WRITE = 1 << 1;
        /// Execute access
        const EXECUTE = 1 << 2;
        /// Read + Write
        const RW = Self::READ.bits() | Self::WRITE.bits();
        /// Read + Execute
        const RX = Self::READ.bits() | Self::EXECUTE.bits();
        /// Read + Write + Execute (dangerous!)
        const RWX = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

/// Memory protection domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionDomain {
    /// Kernel mode
    Kernel,
    /// User mode
    User,
    /// Supervisor mode
    Supervisor,
}

/// Memory protection key (for PKU on x86)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectionKey(u8);

impl ProtectionKey {
    /// Create a new protection key
    pub fn new(key: u8) -> Self {
        Self(key & 0xF) // Only 16 keys available
    }

    /// Get the key value
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Guard page configuration
pub struct GuardPage {
    /// Address of guard page
    pub address: VirtAddr,
    /// Size of guard region
    pub size: usize,
}

impl GuardPage {
    /// Create a guard page
    pub fn new(address: VirtAddr, size: usize) -> Self {
        Self { address, size }
    }

    /// Check if address hits the guard page
    pub fn check(&self, addr: VirtAddr) -> bool {
        let start = self.address.as_u64();
        let end = start + self.size as u64;
        let a = addr.as_u64();
        a >= start && a < end
    }
}
