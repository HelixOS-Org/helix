//! # Relocation Engine
//!
//! Core relocation engine that applies relocations to kernel images.

use crate::context::{KernelState, RelocationContext, RelocationStrategy};
use crate::elf::Elf64Rela;
use crate::elf::relocations::RelocationInfo;
use crate::{PhysAddr, RelocError, RelocResult, RelocationStats, VirtAddr};

// ============================================================================
// TRAITS
// ============================================================================

/// Trait for anything that can be relocated
pub trait Relocatable {
    /// Get relocation context
    fn relocation_context(&self) -> &RelocationContext;

    /// Get mutable relocation context
    fn relocation_context_mut(&mut self) -> &mut RelocationContext;

    /// Apply all relocations
    fn apply_relocations(&mut self) -> RelocResult<RelocationStats>;

    /// Verify integrity after relocation
    fn verify_integrity(&self) -> RelocResult<()>;

    /// Get current base address
    fn base_address(&self) -> VirtAddr;
}

// ============================================================================
// EARLY RELOCATOR
// ============================================================================

/// Early-stage relocator (pre-MMU, physical addresses only)
///
/// This relocator is used during the earliest boot stages when:
/// - MMU is not yet enabled
/// - Only physical addresses are valid
/// - Minimal dependencies (no heap, no std)
/// - Only R_*_RELATIVE relocations are applied
pub struct EarlyRelocator {
    /// Kernel physical base
    kernel_base: PhysAddr,
    /// Kernel size
    kernel_size: usize,
    /// Calculated slide
    slide: i64,
}

impl EarlyRelocator {
    /// Create new early relocator
    ///
    /// # Arguments
    /// * `kernel_base` - Physical address where kernel is loaded
    /// * `link_base` - Physical address kernel was linked at
    /// * `kernel_size` - Size of kernel in bytes
    pub fn new(kernel_base: PhysAddr, link_base: PhysAddr, kernel_size: usize) -> Self {
        let slide = (kernel_base.as_u64() as i64) - (link_base.as_u64() as i64);
        Self {
            kernel_base,
            kernel_size,
            slide,
        }
    }

    /// Apply early relocations (RELATIVE only)
    ///
    /// # Safety
    /// - Kernel memory must be writable
    /// - RELA entries must be valid
    /// - Called before MMU is enabled
    pub unsafe fn apply_relative_relocations(
        &self,
        rela_base: *const Elf64Rela,
        rela_count: usize,
    ) -> RelocResult<RelocationStats> {
        let mut stats = RelocationStats::new();

        for i in 0..rela_count {
            let rela = unsafe { &*rela_base.add(i) };
            let info = RelocationInfo::from_rela(rela);

            // Only process RELATIVE relocations in early stage
            if !info.is_relative() {
                stats.skipped += 1;
                continue;
            }

            // Check bounds
            if info.offset >= self.kernel_size as u64 {
                return Err(RelocError::OutOfBounds(info.offset));
            }

            // Calculate target address
            let target = (self.kernel_base.as_u64() + info.offset) as *mut u64;

            // Apply relocation: *target += slide
            let current = unsafe { core::ptr::read_unaligned(target) };
            let new_value = (current as i64).wrapping_add(self.slide) as u64;
            unsafe { core::ptr::write_unaligned(target, new_value) };

            stats.relative_count += 1;
            stats.applied += 1;
            stats.total += 1;
        }

        Ok(stats)
    }

    /// Get calculated slide
    pub fn slide(&self) -> i64 {
        self.slide
    }

    /// Check if relocation is needed
    pub fn needs_relocation(&self) -> bool {
        self.slide != 0
    }
}

// ============================================================================
// FULL RELOCATOR
// ============================================================================

/// Full-stage relocator (post-MMU, all relocation types)
///
/// This relocator handles all relocation types after:
/// - MMU is enabled
/// - Virtual addresses are active
/// - Full kernel environment available
pub struct FullRelocator {
    /// Relocation context
    ctx: RelocationContext,
}

impl FullRelocator {
    /// Create new full relocator
    pub fn new(ctx: RelocationContext) -> Self {
        Self { ctx }
    }

    /// Apply all relocations
    ///
    /// # Safety
    /// - Kernel memory must be writable
    /// - ELF info must be valid
    pub unsafe fn apply_all(&mut self) -> RelocResult<RelocationStats> {
        let elf = self.ctx.elf.as_ref().ok_or(RelocError::NotInitialized)?;

        let mut stats = RelocationStats::new();

        // Get RELA info
        let rela_addr = elf.rela_addr.ok_or(RelocError::NoRelocations)?;
        let rela_count = elf.rela_count;

        // Convert virtual address to pointer
        let rela_ptr = self.vaddr_to_ptr(rela_addr)? as *const Elf64Rela;

        // Apply relocations based on strategy
        match self.ctx.strategy {
            RelocationStrategy::FullPie => {
                unsafe { self.apply_rela_entries(rela_ptr, rela_count, &mut stats)? };

                // Apply GOT relocations
                if let Some(got_addr) = elf.got_addr {
                    unsafe { self.apply_got(got_addr, &mut stats)? };
                }

                // Apply PLT relocations
                if elf.plt_rela_addr.is_some() {
                    let plt_addr = elf.plt_rela_addr.unwrap();
                    let plt_count = elf.plt_rela_size / core::mem::size_of::<Elf64Rela>();
                    let plt_ptr = self.vaddr_to_ptr(plt_addr)? as *const Elf64Rela;
                    unsafe { self.apply_rela_entries(plt_ptr, plt_count, &mut stats)? };
                }
            },

            RelocationStrategy::StaticMinimal => {
                // Only apply RELATIVE relocations
                for i in 0..rela_count {
                    let rela = unsafe { &*rela_ptr.add(i) };
                    let info = RelocationInfo::from_rela(rela);

                    if info.is_relative() {
                        unsafe { self.apply_single_relocation(&info, &mut stats)? };
                    } else {
                        stats.skipped += 1;
                    }
                    stats.total += 1;
                }
            },

            RelocationStrategy::Hybrid => {
                // Relocate everything except TLS
                for i in 0..rela_count {
                    let rela = unsafe { &*rela_ptr.add(i) };
                    let info = RelocationInfo::from_rela(rela);

                    unsafe { self.apply_single_relocation(&info, &mut stats)? };
                    stats.total += 1;
                }
            },

            RelocationStrategy::EarlyBoot | RelocationStrategy::None => {
                // Should not use FullRelocator for these
                return Err(RelocError::NotInitialized);
            },
        }

        // Update state
        self.ctx.state = KernelState::FullyRelocated;

        Ok(stats)
    }

    /// Apply RELA entries
    unsafe fn apply_rela_entries(
        &self,
        rela_ptr: *const Elf64Rela,
        count: usize,
        stats: &mut RelocationStats,
    ) -> RelocResult<()> {
        for i in 0..count {
            let rela = unsafe { &*rela_ptr.add(i) };
            let info = RelocationInfo::from_rela(rela);

            unsafe { self.apply_single_relocation(&info, stats)? };
            stats.total += 1;
        }
        Ok(())
    }

    /// Apply a single relocation
    unsafe fn apply_single_relocation(
        &self,
        info: &RelocationInfo,
        stats: &mut RelocationStats,
    ) -> RelocResult<()> {
        // Check bounds
        if info.offset >= self.ctx.kernel_size as u64 {
            return Err(RelocError::OutOfBounds(info.offset));
        }

        // Calculate target address
        let target = (self.ctx.virt_base.as_u64() + info.offset) as *mut u8;

        // Resolve symbol if needed
        let symbol_value = if info.needs_symbol() {
            self.resolve_symbol(info.sym_index)?
        } else {
            0
        };

        // Apply relocation based on architecture
        #[cfg(target_arch = "x86_64")]
        {
            unsafe {
                crate::elf::relocations::apply_x86_64_relocation(
                    target,
                    info.r_type,
                    info.addend,
                    self.ctx.slide,
                    symbol_value,
                )?;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            unsafe {
                crate::elf::relocations::apply_aarch64_relocation(
                    target,
                    info.r_type,
                    info.addend,
                    self.ctx.slide,
                    symbol_value,
                )?;
            }
        }

        // Update stats
        if info.is_relative() {
            stats.relative_count += 1;
        } else if info.r_type == crate::elf::relocations::x86_64::R_X86_64_64 {
            stats.absolute_count += 1;
        } else if info.r_type == crate::elf::relocations::x86_64::R_X86_64_PC32 {
            stats.pc32_count += 1;
        }
        stats.applied += 1;

        Ok(())
    }

    /// Apply GOT relocations
    unsafe fn apply_got(&self, _got_addr: u64, stats: &mut RelocationStats) -> RelocResult<()> {
        // GOT entries are typically handled by GLOB_DAT relocations
        // This is a placeholder for additional GOT processing if needed
        stats.got_entries += 1;
        Ok(())
    }

    /// Resolve symbol index to value
    fn resolve_symbol(&self, _sym_index: u32) -> RelocResult<u64> {
        // For kernel relocation, most symbols are internal
        // External symbol resolution would go here
        // For now, return error for unresolved symbols
        Err(RelocError::SymbolNotFound(_sym_index))
    }

    /// Convert virtual address to pointer
    fn vaddr_to_ptr(&self, vaddr: u64) -> RelocResult<*mut u8> {
        // Calculate offset from kernel base
        let base = self.ctx.virt_base.as_u64();

        if vaddr >= base && vaddr < base + self.ctx.kernel_size as u64 {
            Ok(vaddr as *mut u8)
        } else {
            Err(RelocError::OutOfBounds(vaddr))
        }
    }

    /// Get context reference
    pub fn context(&self) -> &RelocationContext {
        &self.ctx
    }

    /// Consume and return context
    pub fn into_context(self) -> RelocationContext {
        self.ctx
    }
}

// ============================================================================
// RELOCATION ENGINE
// ============================================================================

/// High-level relocation engine
///
/// Coordinates between early and full relocation stages.
pub struct RelocationEngine {
    /// Current context
    ctx: Option<RelocationContext>,
    /// Accumulated stats
    stats: RelocationStats,
}

impl RelocationEngine {
    /// Create new engine
    pub fn new() -> Self {
        Self {
            ctx: None,
            stats: RelocationStats::new(),
        }
    }

    /// Initialize with context
    pub fn init(&mut self, ctx: RelocationContext) {
        self.ctx = Some(ctx);
    }

    /// Get context
    pub fn context(&self) -> Option<&RelocationContext> {
        self.ctx.as_ref()
    }

    /// Get mutable context
    pub fn context_mut(&mut self) -> Option<&mut RelocationContext> {
        self.ctx.as_mut()
    }

    /// Get accumulated stats
    pub fn stats(&self) -> &RelocationStats {
        &self.stats
    }

    /// Perform early relocation
    ///
    /// # Safety
    /// Must be called before MMU is enabled
    pub unsafe fn early_relocate(
        &mut self,
        rela_base: *const Elf64Rela,
        rela_count: usize,
    ) -> RelocResult<RelocationStats> {
        let ctx = self.ctx.as_mut().ok_or(RelocError::NotInitialized)?;

        let early = EarlyRelocator::new(
            ctx.phys_base,
            PhysAddr::new(ctx.link_base.as_u64()),
            ctx.kernel_size,
        );

        let stats = unsafe { early.apply_relative_relocations(rela_base, rela_count)? };

        ctx.state = KernelState::EarlyRelocated;
        self.stats.merge(&stats);

        Ok(stats)
    }

    /// Perform full relocation
    ///
    /// # Safety
    /// Must be called after MMU is enabled
    pub unsafe fn full_relocate(&mut self) -> RelocResult<RelocationStats> {
        let ctx = self.ctx.take().ok_or(RelocError::NotInitialized)?;

        let mut full = FullRelocator::new(ctx);
        let stats = unsafe { full.apply_all()? };

        self.ctx = Some(full.into_context());
        self.stats.merge(&stats);

        Ok(stats)
    }

    /// Finalize relocation
    pub fn finalize(&mut self) -> RelocResult<()> {
        let ctx = self.ctx.as_mut().ok_or(RelocError::NotInitialized)?;
        ctx.state = KernelState::Finalized;
        Ok(())
    }
}

impl Default for RelocationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RELOCATABLE KERNEL
// ============================================================================

/// Kernel-specific relocatable implementation
pub struct RelocatableKernel {
    /// Engine
    engine: RelocationEngine,
}

impl RelocatableKernel {
    /// Create from context
    pub fn new(ctx: RelocationContext) -> Self {
        let mut engine = RelocationEngine::new();
        engine.init(ctx);
        Self { engine }
    }

    /// Create from boot context
    pub fn from_boot_context(boot: &crate::boot::BootContext) -> RelocResult<Self> {
        let ctx = RelocationContext::builder()
            .phys_base(boot.kernel_phys_base.as_u64())
            .virt_base(boot.kernel_virt_base)
            .kernel_size(boot.kernel_size)
            .boot_protocol(boot.protocol)
            .build()?;
        Ok(Self::new(ctx))
    }

    /// Apply early relocations (pre-MMU)
    ///
    /// # Safety
    /// Must be called before MMU is enabled
    pub unsafe fn apply_early(
        &mut self,
        rela_base: *const Elf64Rela,
        rela_count: usize,
    ) -> RelocResult<RelocationStats> {
        unsafe { self.engine.early_relocate(rela_base, rela_count) }
    }

    /// Apply full relocations (post-MMU)
    ///
    /// # Safety
    /// Must be called after MMU is enabled
    pub unsafe fn apply_full(&mut self) -> RelocResult<RelocationStats> {
        unsafe { self.engine.full_relocate() }
    }

    /// Verify integrity
    pub fn verify_integrity(&self) -> RelocResult<()> {
        #[cfg(feature = "validation")]
        {
            crate::validation::verify_kernel(self.engine.context().unwrap())?;
        }
        Ok(())
    }

    /// Finalize and return stats
    pub fn finalize(mut self) -> RelocResult<(RelocationContext, RelocationStats)> {
        self.engine.finalize()?;
        let ctx = self.engine.ctx.take().unwrap();
        Ok((ctx, self.engine.stats))
    }

    /// Get stats
    pub fn stats(&self) -> &RelocationStats {
        self.engine.stats()
    }
}

impl Relocatable for RelocatableKernel {
    fn relocation_context(&self) -> &RelocationContext {
        self.engine.context().unwrap()
    }

    fn relocation_context_mut(&mut self) -> &mut RelocationContext {
        self.engine.context_mut().unwrap()
    }

    fn apply_relocations(&mut self) -> RelocResult<RelocationStats> {
        unsafe { self.apply_full() }
    }

    fn verify_integrity(&self) -> RelocResult<()> {
        self.verify_integrity()
    }

    fn base_address(&self) -> VirtAddr {
        self.engine.context().unwrap().virt_base
    }
}
