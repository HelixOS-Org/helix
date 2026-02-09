// SPDX-License-Identifier: GPL-2.0
//! Apps tls_v2 — thread-local storage management v2.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TLS model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsModel {
    LocalExec,
    InitialExec,
    LocalDynamic,
    GeneralDynamic,
}

/// TLS variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVariant {
    Variant1,
    Variant2,
}

/// TLS block state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsBlockState {
    Uninitialized,
    Initializing,
    Ready,
    Freed,
}

/// TLS module descriptor
#[derive(Debug, Clone)]
pub struct TlsModule {
    pub id: u64,
    pub size: u64,
    pub align: u64,
    pub init_size: u64,
    pub offset: u64,
    pub model: TlsModel,
    pub generation: u32,
}

impl TlsModule {
    pub fn new(id: u64, size: u64, align: u64, model: TlsModel) -> Self {
        Self { id, size, align, init_size: size, offset: 0, model, generation: 0 }
    }

    #[inline(always)]
    pub fn aligned_size(&self) -> u64 {
        if self.align == 0 { return self.size; }
        (self.size + self.align - 1) & !(self.align - 1)
    }
}

/// Thread TLS block
#[derive(Debug, Clone)]
pub struct TlsBlock {
    pub thread_id: u64,
    pub module_id: u64,
    pub base_addr: u64,
    pub state: TlsBlockState,
    pub access_count: u64,
}

impl TlsBlock {
    pub fn new(thread_id: u64, module_id: u64, base: u64) -> Self {
        Self { thread_id, module_id, base_addr: base, state: TlsBlockState::Uninitialized, access_count: 0 }
    }

    #[inline(always)]
    pub fn initialize(&mut self) { self.state = TlsBlockState::Initializing; }
    #[inline(always)]
    pub fn ready(&mut self) { self.state = TlsBlockState::Ready; }
    #[inline(always)]
    pub fn free(&mut self) { self.state = TlsBlockState::Freed; }
    #[inline(always)]
    pub fn access(&mut self) { self.access_count += 1; }
}

/// DTV (Dynamic Thread Vector) entry
#[derive(Debug, Clone)]
pub struct DtvEntry {
    pub module_id: u64,
    pub block_addr: u64,
    pub generation: u32,
    pub allocated: bool,
}

/// Per-thread TLS state
#[derive(Debug)]
#[repr(align(64))]
pub struct ThreadTlsState {
    pub thread_id: u64,
    pub dtv: Vec<DtvEntry>,
    pub tp_addr: u64,
    pub variant: TlsVariant,
    pub blocks: BTreeMap<u64, TlsBlock>,
    pub total_accesses: u64,
}

impl ThreadTlsState {
    pub fn new(thread_id: u64, tp_addr: u64, variant: TlsVariant) -> Self {
        Self {
            thread_id, dtv: Vec::new(), tp_addr, variant,
            blocks: BTreeMap::new(), total_accesses: 0,
        }
    }

    #[inline]
    pub fn allocate_block(&mut self, module_id: u64, base: u64) {
        let block = TlsBlock::new(self.thread_id, module_id, base);
        self.blocks.insert(module_id, block);
        self.dtv.push(DtvEntry { module_id, block_addr: base, generation: 0, allocated: true });
    }

    #[inline]
    pub fn access(&mut self, module_id: u64) -> Option<u64> {
        if let Some(block) = self.blocks.get_mut(&module_id) {
            block.access();
            self.total_accesses += 1;
            Some(block.base_addr)
        } else { None }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TlsV2Stats {
    pub total_modules: u32,
    pub total_threads: u32,
    pub total_blocks: u32,
    pub total_tls_bytes: u64,
    pub total_accesses: u64,
}

/// Main TLS v2 manager
pub struct AppTlsV2 {
    modules: BTreeMap<u64, TlsModule>,
    threads: BTreeMap<u64, ThreadTlsState>,
    next_module_id: u64,
    next_offset: u64,
}

impl AppTlsV2 {
    pub fn new() -> Self {
        Self { modules: BTreeMap::new(), threads: BTreeMap::new(), next_module_id: 1, next_offset: 0 }
    }

    #[inline]
    pub fn register_module(&mut self, size: u64, align: u64, model: TlsModel) -> u64 {
        let id = self.next_module_id;
        self.next_module_id += 1;
        let mut module = TlsModule::new(id, size, align, model);
        module.offset = self.next_offset;
        self.next_offset += module.aligned_size();
        self.modules.insert(id, module);
        id
    }

    #[inline(always)]
    pub fn create_thread(&mut self, thread_id: u64, tp_addr: u64, variant: TlsVariant) {
        self.threads.insert(thread_id, ThreadTlsState::new(thread_id, tp_addr, variant));
    }

    #[inline]
    pub fn allocate_block(&mut self, thread_id: u64, module_id: u64) -> Option<u64> {
        let module = self.modules.get(&module_id)?;
        let base = module.offset; // simplified
        self.threads.get_mut(&thread_id)?.allocate_block(module_id, base);
        Some(base)
    }

    #[inline]
    pub fn stats(&self) -> TlsV2Stats {
        let blocks: u32 = self.threads.values().map(|t| t.blocks.len() as u32).sum();
        let bytes: u64 = self.modules.values().map(|m| m.aligned_size()).sum();
        let accesses: u64 = self.threads.values().map(|t| t.total_accesses).sum();
        TlsV2Stats {
            total_modules: self.modules.len() as u32, total_threads: self.threads.len() as u32,
            total_blocks: blocks, total_tls_bytes: bytes, total_accesses: accesses,
        }
    }
}
