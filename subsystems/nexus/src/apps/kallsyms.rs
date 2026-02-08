// SPDX-License-Identifier: GPL-2.0
//! Apps kallsyms — kernel symbol table lookup proxy for debugging and profiling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Symbol type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Text (code) symbol
    Text,
    /// Data symbol
    Data,
    /// BSS (uninitialized data)
    Bss,
    /// Read-only data
    Rodata,
    /// Weak symbol
    Weak,
    /// Absolute symbol
    Absolute,
    /// Module symbol
    Module,
    /// Undefined / external
    Undefined,
}

/// Symbol binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
}

/// A kernel symbol entry
#[derive(Debug, Clone)]
pub struct KernelSymbol {
    pub address: u64,
    pub size: u64,
    pub name: String,
    pub module: Option<String>,
    pub sym_type: SymbolType,
    pub binding: SymbolBinding,
    pub exported: bool,
    pub gpl_only: bool,
    lookup_count: u64,
}

impl KernelSymbol {
    pub fn new(address: u64, name: String, sym_type: SymbolType) -> Self {
        Self {
            address,
            size: 0,
            name,
            module: None,
            sym_type,
            binding: SymbolBinding::Global,
            exported: false,
            gpl_only: false,
            lookup_count: 0,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_module(mut self, module: String) -> Self {
        self.module = Some(module);
        self
    }

    pub fn contains_addr(&self, addr: u64) -> bool {
        if self.size == 0 {
            return addr == self.address;
        }
        addr >= self.address && addr < self.address + self.size
    }

    pub fn offset_of(&self, addr: u64) -> Option<u64> {
        if self.contains_addr(addr) {
            Some(addr - self.address)
        } else {
            None
        }
    }

    pub fn is_kernel_text(&self) -> bool {
        self.sym_type == SymbolType::Text && self.module.is_none()
    }
}

/// Address lookup result
#[derive(Debug, Clone)]
pub struct SymbolLookup {
    pub symbol: String,
    pub offset: u64,
    pub module: Option<String>,
    pub exact: bool,
}

impl SymbolLookup {
    pub fn format_symbolic(&self) -> String {
        let mut s = self.symbol.clone();
        if self.offset > 0 {
            // Format as "symbol+0xoffset"
            s.push_str("+0x");
            // Simple hex formatting
            let mut hex = alloc::format!("{:x}", self.offset);
            s.push_str(&hex);
        }
        if let Some(ref module) = self.module {
            s.push_str(" [");
            s.push_str(module);
            s.push(']');
        }
        s
    }
}

/// Symbol table section
#[derive(Debug)]
pub struct SymbolSection {
    pub name: String,
    pub start: u64,
    pub end: u64,
    pub symbol_count: usize,
}

impl SymbolSection {
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// Kallsyms stats
#[derive(Debug, Clone)]
pub struct KallsymsStats {
    pub total_symbols: u64,
    pub text_symbols: u64,
    pub data_symbols: u64,
    pub module_symbols: u64,
    pub lookups_by_name: u64,
    pub lookups_by_addr: u64,
    pub lookup_hits: u64,
    pub lookup_misses: u64,
}

/// Main apps kallsyms manager
pub struct AppKallsyms {
    /// Symbols sorted by address for binary search
    symbols_by_addr: Vec<KernelSymbol>,
    /// Name → index mapping for name lookups
    name_index: BTreeMap<String, usize>,
    /// Sections for quick range checking
    sections: Vec<SymbolSection>,
    /// Access restrictions
    restrict_to_exported: bool,
    stats: KallsymsStats,
}

impl AppKallsyms {
    pub fn new() -> Self {
        Self {
            symbols_by_addr: Vec::new(),
            name_index: BTreeMap::new(),
            sections: Vec::new(),
            restrict_to_exported: false,
            stats: KallsymsStats {
                total_symbols: 0,
                text_symbols: 0,
                data_symbols: 0,
                module_symbols: 0,
                lookups_by_name: 0,
                lookups_by_addr: 0,
                lookup_hits: 0,
                lookup_misses: 0,
            },
        }
    }

    pub fn load_symbol(&mut self, symbol: KernelSymbol) {
        self.stats.total_symbols += 1;
        match symbol.sym_type {
            SymbolType::Text => self.stats.text_symbols += 1,
            SymbolType::Data | SymbolType::Bss | SymbolType::Rodata => self.stats.data_symbols += 1,
            _ => {}
        }
        if symbol.module.is_some() {
            self.stats.module_symbols += 1;
        }
        self.name_index.insert(symbol.name.clone(), self.symbols_by_addr.len());
        self.symbols_by_addr.push(symbol);
    }

    pub fn finalize(&mut self) {
        self.symbols_by_addr.sort_by_key(|s| s.address);
        // Rebuild name index after sorting
        self.name_index.clear();
        for (i, sym) in self.symbols_by_addr.iter().enumerate() {
            self.name_index.insert(sym.name.clone(), i);
        }
        // Calculate sizes from gaps
        for i in 0..self.symbols_by_addr.len().saturating_sub(1) {
            if self.symbols_by_addr[i].size == 0 {
                let next_addr = self.symbols_by_addr[i + 1].address;
                let gap = next_addr.saturating_sub(self.symbols_by_addr[i].address);
                if gap < 1024 * 1024 { // Max 1MB per symbol
                    self.symbols_by_addr[i].size = gap;
                }
            }
        }
    }

    pub fn lookup_by_name(&mut self, name: &str) -> Option<&KernelSymbol> {
        self.stats.lookups_by_name += 1;
        if let Some(&idx) = self.name_index.get(name) {
            let sym = &self.symbols_by_addr[idx];
            if self.restrict_to_exported && !sym.exported {
                self.stats.lookup_misses += 1;
                return None;
            }
            self.stats.lookup_hits += 1;
            Some(sym)
        } else {
            self.stats.lookup_misses += 1;
            None
        }
    }

    pub fn lookup_by_addr(&mut self, addr: u64) -> Option<SymbolLookup> {
        self.stats.lookups_by_addr += 1;
        if self.symbols_by_addr.is_empty() {
            self.stats.lookup_misses += 1;
            return None;
        }
        // Binary search for the nearest symbol at or before addr
        let idx = match self.symbols_by_addr.binary_search_by_key(&addr, |s| s.address) {
            Ok(i) => i,
            Err(i) => {
                if i == 0 {
                    self.stats.lookup_misses += 1;
                    return None;
                }
                i - 1
            }
        };
        let sym = &self.symbols_by_addr[idx];
        let offset = addr.saturating_sub(sym.address);
        // Only accept if within symbol bounds (or size unknown with small offset)
        if sym.size > 0 && offset >= sym.size {
            self.stats.lookup_misses += 1;
            return None;
        }
        if sym.size == 0 && offset > 4096 {
            self.stats.lookup_misses += 1;
            return None;
        }
        if self.restrict_to_exported && !sym.exported {
            self.stats.lookup_misses += 1;
            return None;
        }
        self.stats.lookup_hits += 1;
        Some(SymbolLookup {
            symbol: sym.name.clone(),
            offset,
            module: sym.module.clone(),
            exact: offset == 0,
        })
    }

    pub fn symbolize_stack(&mut self, addresses: &[u64]) -> Vec<SymbolLookup> {
        addresses.iter().filter_map(|&addr| self.lookup_by_addr(addr)).collect()
    }

    pub fn add_section(&mut self, name: String, start: u64, end: u64) {
        let count = self.symbols_by_addr.iter()
            .filter(|s| s.address >= start && s.address < end)
            .count();
        self.sections.push(SymbolSection { name, start, end, symbol_count: count });
    }

    pub fn set_restrict_exported(&mut self, restrict: bool) {
        self.restrict_to_exported = restrict;
    }

    pub fn search_prefix(&self, prefix: &str, max: usize) -> Vec<&str> {
        self.name_index.keys()
            .filter(|name| name.starts_with(prefix))
            .take(max)
            .map(|s| s.as_str())
            .collect()
    }

    pub fn stats(&self) -> &KallsymsStats {
        &self.stats
    }
}
