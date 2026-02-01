//! # Code Templates Engine
//!
//! Year 3 EVOLUTION - Template-based code generation
//! Kernel-specific patterns and idioms for synthesis.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::ir::{IROp, IRType, IRValue};
use super::{Specification, TypeSpec};

// ============================================================================
// TEMPLATE TYPES
// ============================================================================

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateCategory {
    // Data structures
    Array,
    LinkedList,
    Tree,
    HashMap,
    BitSet,
    RingBuffer,

    // Algorithms
    Search,
    Sort,
    Hash,
    Compression,
    Checksum,

    // Control flow
    Loop,
    Conditional,
    StateMachine,

    // Memory
    Allocator,
    Pool,
    Slab,
    Arena,

    // Synchronization
    SpinLock,
    Mutex,
    Semaphore,
    RwLock,
    Atomic,

    // Kernel patterns
    Interrupt,
    Syscall,
    Driver,
    Scheduler,

    // Error handling
    Result,
    Option,
    Panic,
}

/// Code template
#[derive(Debug, Clone)]
pub struct Template {
    /// Template ID
    pub id: u64,
    /// Template name
    pub name: String,
    /// Category
    pub category: TemplateCategory,
    /// Description
    pub description: String,
    /// Template code
    pub code: String,
    /// Parameters
    pub parameters: Vec<TemplateParam>,
    /// Constraints
    pub constraints: Vec<TemplateConstraint>,
    /// Performance characteristics
    pub performance: TemplatePerformance,
}

/// Template parameter
#[derive(Debug, Clone)]
pub struct TemplateParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub typ: ParamType,
    /// Default value
    pub default: Option<String>,
    /// Description
    pub description: String,
}

/// Parameter type
#[derive(Debug, Clone)]
pub enum ParamType {
    Type,
    Expr,
    Ident,
    Const,
    Block,
}

/// Template constraint
#[derive(Debug, Clone)]
pub enum TemplateConstraint {
    TypeIs(String, TypeSpec),
    TypeImplements(String, String),
    ConstRange(String, i128, i128),
    Requires(String),
    NoAlloc,
    NoUnsafe,
}

/// Performance characteristics
#[derive(Debug, Clone)]
pub struct TemplatePerformance {
    /// Time complexity
    pub time_complexity: String,
    /// Space complexity
    pub space_complexity: String,
    /// Cache friendly
    pub cache_friendly: bool,
    /// Branch-free
    pub branch_free: bool,
}

/// Template instantiation
#[derive(Debug, Clone)]
pub struct Instantiation {
    /// Template ID
    pub template_id: u64,
    /// Parameter bindings
    pub bindings: BTreeMap<String, String>,
    /// Generated code
    pub code: String,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Matched template
    pub template: Template,
    /// Match score
    pub score: f64,
    /// Suggested bindings
    pub bindings: BTreeMap<String, String>,
}

// ============================================================================
// TEMPLATE LIBRARY
// ============================================================================

/// Template library
pub struct TemplateLibrary {
    /// All templates
    templates: BTreeMap<u64, Template>,
    /// Index by category
    by_category: BTreeMap<TemplateCategory, Vec<u64>>,
    /// Index by name
    by_name: BTreeMap<String, u64>,
    /// Next ID
    next_id: u64,
}

impl TemplateLibrary {
    /// Create new library
    pub fn new() -> Self {
        let mut lib = Self {
            templates: BTreeMap::new(),
            by_category: BTreeMap::new(),
            by_name: BTreeMap::new(),
            next_id: 1,
        };

        lib.load_builtin_templates();
        lib
    }

    fn load_builtin_templates(&mut self) {
        // Array templates
        self.add_template(Template {
            id: 0,
            name: "array_search_linear".into(),
            category: TemplateCategory::Search,
            description: "Linear search in array".into(),
            code: r#"
pub fn {{name}}<T: PartialEq>(arr: &[T], target: &T) -> Option<usize> {
    for (i, item) in arr.iter().enumerate() {
        if item == target {
            return Some(i);
        }
    }
    None
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("linear_search".into()),
                description: "Function name".into(),
            }],
            constraints: vec![],
            performance: TemplatePerformance {
                time_complexity: "O(n)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        self.add_template(Template {
            id: 0,
            name: "array_search_binary".into(),
            category: TemplateCategory::Search,
            description: "Binary search in sorted array".into(),
            code: r#"
pub fn {{name}}<T: Ord>(arr: &[T], target: &T) -> Option<usize> {
    let mut left = 0;
    let mut right = arr.len();

    while left < right {
        let mid = left + (right - left) / 2;
        match arr[mid].cmp(target) {
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Less => left = mid + 1,
            core::cmp::Ordering::Greater => right = mid,
        }
    }
    None
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("binary_search".into()),
                description: "Function name".into(),
            }],
            constraints: vec![],
            performance: TemplatePerformance {
                time_complexity: "O(log n)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: false,
                branch_free: false,
            },
        });

        // Spinlock template
        self.add_template(Template {
            id: 0,
            name: "spinlock".into(),
            category: TemplateCategory::SpinLock,
            description: "Simple spinlock implementation".into(),
            code: r#"
use core::sync::atomic::{AtomicBool, Ordering};

pub struct {{name}} {
    locked: AtomicBool,
}

impl {{name}} {
    pub const fn new() -> Self {
        Self { locked: AtomicBool::new(false) }
    }

    pub fn lock(&self) {
        while self.locked.compare_exchange_weak(
            false, true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            core::hint::spin_loop();
        }
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    pub fn try_lock(&self) -> bool {
        self.locked.compare_exchange(
            false, true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_ok()
    }
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("SpinLock".into()),
                description: "Type name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc],
            performance: TemplatePerformance {
                time_complexity: "O(1) amortized".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        // Ring buffer template
        self.add_template(Template {
            id: 0,
            name: "ring_buffer".into(),
            category: TemplateCategory::RingBuffer,
            description: "Fixed-size ring buffer".into(),
            code: r#"
pub struct {{name}}<T, const N: usize> {
    buffer: [core::mem::MaybeUninit<T>; N],
    head: usize,
    tail: usize,
    len: usize,
}

impl<T, const N: usize> {{name}}<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len == N {
            return Err(item);
        }
        self.buffer[self.tail].write(item);
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let item = unsafe { self.buffer[self.head].assume_init_read() };
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(item)
    }

    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn is_full(&self) -> bool { self.len == N }
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("RingBuffer".into()),
                description: "Type name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc],
            performance: TemplatePerformance {
                time_complexity: "O(1)".into(),
                space_complexity: "O(N)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        // Bitset template
        self.add_template(Template {
            id: 0,
            name: "bitset".into(),
            category: TemplateCategory::BitSet,
            description: "Fixed-size bitset".into(),
            code: r#"
pub struct {{name}}<const N: usize> {
    bits: [u64; (N + 63) / 64],
}

impl<const N: usize> {{name}}<N> {
    pub const fn new() -> Self {
        Self { bits: [0; (N + 63) / 64] }
    }

    pub fn set(&mut self, idx: usize) {
        if idx < N {
            self.bits[idx / 64] |= 1 << (idx % 64);
        }
    }

    pub fn clear(&mut self, idx: usize) {
        if idx < N {
            self.bits[idx / 64] &= !(1 << (idx % 64));
        }
    }

    pub fn get(&self, idx: usize) -> bool {
        if idx < N {
            (self.bits[idx / 64] >> (idx % 64)) & 1 == 1
        } else {
            false
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        if idx < N {
            self.bits[idx / 64] ^= 1 << (idx % 64);
        }
    }

    pub fn count_ones(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }

    pub fn first_set(&self) -> Option<usize> {
        for (i, &word) in self.bits.iter().enumerate() {
            if word != 0 {
                return Some(i * 64 + word.trailing_zeros() as usize);
            }
        }
        None
    }
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("BitSet".into()),
                description: "Type name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc],
            performance: TemplatePerformance {
                time_complexity: "O(1)".into(),
                space_complexity: "O(N/64)".into(),
                cache_friendly: true,
                branch_free: true,
            },
        });

        // Hash function template
        self.add_template(Template {
            id: 0,
            name: "fnv1a_hash".into(),
            category: TemplateCategory::Hash,
            description: "FNV-1a hash function".into(),
            code: r#"
pub fn {{name}}(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("fnv1a".into()),
                description: "Function name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc, TemplateConstraint::NoUnsafe],
            performance: TemplatePerformance {
                time_complexity: "O(n)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: true,
            },
        });

        // CRC32 template
        self.add_template(Template {
            id: 0,
            name: "crc32".into(),
            category: TemplateCategory::Checksum,
            description: "CRC32 checksum".into(),
            code: r#"
pub fn {{name}}(data: &[u8]) -> u32 {
    const POLY: u32 = 0xedb88320;
    let mut crc = 0xffffffff;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }

    !crc
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("crc32".into()),
                description: "Function name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc, TemplateConstraint::NoUnsafe],
            performance: TemplatePerformance {
                time_complexity: "O(n)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        // Interrupt handler template
        self.add_template(Template {
            id: 0,
            name: "interrupt_handler".into(),
            category: TemplateCategory::Interrupt,
            description: "Interrupt handler skeleton".into(),
            code: r#"
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

pub extern "x86-interrupt" fn {{name}}(frame: InterruptFrame) {
    // Save state
    {{save_state}}

    // Handle interrupt
    {{handler_body}}

    // Restore state
    {{restore_state}}

    // Send EOI if needed
    {{eoi}}
}
"#
            .into(),
            parameters: vec![
                TemplateParam {
                    name: "name".into(),
                    typ: ParamType::Ident,
                    default: Some("interrupt_handler".into()),
                    description: "Handler name".into(),
                },
                TemplateParam {
                    name: "save_state".into(),
                    typ: ParamType::Block,
                    default: Some("// State saved by CPU".into()),
                    description: "State saving code".into(),
                },
                TemplateParam {
                    name: "handler_body".into(),
                    typ: ParamType::Block,
                    default: Some("// TODO: Handle interrupt".into()),
                    description: "Handler body".into(),
                },
                TemplateParam {
                    name: "restore_state".into(),
                    typ: ParamType::Block,
                    default: Some("// State restored by iret".into()),
                    description: "State restoration code".into(),
                },
                TemplateParam {
                    name: "eoi".into(),
                    typ: ParamType::Block,
                    default: Some("// unsafe { send_eoi(); }".into()),
                    description: "End of interrupt".into(),
                },
            ],
            constraints: vec![TemplateConstraint::NoAlloc],
            performance: TemplatePerformance {
                time_complexity: "O(1)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        // Syscall handler template
        self.add_template(Template {
            id: 0,
            name: "syscall_handler".into(),
            category: TemplateCategory::Syscall,
            description: "System call handler".into(),
            code: r#"
pub fn {{name}}(syscall_num: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    match syscall_num {
        {{syscall_cases}}
        _ => -1, // ENOSYS
    }
}
"#
            .into(),
            parameters: vec![
                TemplateParam {
                    name: "name".into(),
                    typ: ParamType::Ident,
                    default: Some("syscall_dispatch".into()),
                    description: "Handler name".into(),
                },
                TemplateParam {
                    name: "syscall_cases".into(),
                    typ: ParamType::Block,
                    default: Some("0 => { /* read */ 0 }".into()),
                    description: "Syscall match arms".into(),
                },
            ],
            constraints: vec![],
            performance: TemplatePerformance {
                time_complexity: "O(1)".into(),
                space_complexity: "O(1)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });

        // Memory pool template
        self.add_template(Template {
            id: 0,
            name: "memory_pool".into(),
            category: TemplateCategory::Pool,
            description: "Fixed-size memory pool allocator".into(),
            code: r#"
pub struct {{name}}<T, const N: usize> {
    storage: [core::mem::MaybeUninit<T>; N],
    free_list: [u16; N],
    free_head: usize,
    allocated: usize,
}

impl<T, const N: usize> {{name}}<T, N> {
    pub const fn new() -> Self {
        let mut free_list = [0u16; N];
        let mut i = 0;
        while i < N {
            free_list[i] = (i + 1) as u16;
            i += 1;
        }
        Self {
            storage: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            free_list,
            free_head: 0,
            allocated: 0,
        }
    }

    pub fn alloc(&mut self) -> Option<&mut T> {
        if self.free_head >= N {
            return None;
        }
        let idx = self.free_head;
        self.free_head = self.free_list[idx] as usize;
        self.allocated += 1;
        Some(unsafe { &mut *self.storage[idx].as_mut_ptr() })
    }

    pub fn free(&mut self, ptr: &mut T) {
        let addr = ptr as *mut T as usize;
        let base = self.storage.as_ptr() as usize;
        let idx = (addr - base) / core::mem::size_of::<T>();

        if idx < N {
            self.free_list[idx] = self.free_head as u16;
            self.free_head = idx;
            self.allocated -= 1;
        }
    }

    pub fn allocated(&self) -> usize { self.allocated }
    pub fn available(&self) -> usize { N - self.allocated }
}
"#
            .into(),
            parameters: vec![TemplateParam {
                name: "name".into(),
                typ: ParamType::Ident,
                default: Some("MemoryPool".into()),
                description: "Type name".into(),
            }],
            constraints: vec![TemplateConstraint::NoAlloc],
            performance: TemplatePerformance {
                time_complexity: "O(1)".into(),
                space_complexity: "O(N)".into(),
                cache_friendly: true,
                branch_free: false,
            },
        });
    }

    /// Add template to library
    pub fn add_template(&mut self, mut template: Template) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        template.id = id;

        self.by_name.insert(template.name.clone(), id);
        self.by_category
            .entry(template.category)
            .or_default()
            .push(id);
        self.templates.insert(id, template);

        id
    }

    /// Get template by ID
    pub fn get(&self, id: u64) -> Option<&Template> {
        self.templates.get(&id)
    }

    /// Get template by name
    pub fn get_by_name(&self, name: &str) -> Option<&Template> {
        self.by_name.get(name).and_then(|id| self.templates.get(id))
    }

    /// Get templates by category
    pub fn get_by_category(&self, category: TemplateCategory) -> Vec<&Template> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.templates.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find matching templates for spec
    pub fn find_matches(&self, spec: &Specification) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for template in self.templates.values() {
            if let Some(score) = self.match_score(template, spec) {
                if score > 0.0 {
                    matches.push(PatternMatch {
                        template: template.clone(),
                        score,
                        bindings: self.suggest_bindings(template, spec),
                    });
                }
            }
        }

        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        matches
    }

    fn match_score(&self, template: &Template, spec: &Specification) -> Option<f64> {
        let mut score = 0.0;

        // Name similarity
        if spec.name.contains(&template.name) || template.name.contains(&spec.name) {
            score += 0.3;
        }

        // Category hints from spec name
        let name_lower = spec.name.to_lowercase();
        if name_lower.contains("search") && template.category == TemplateCategory::Search {
            score += 0.5;
        }
        if name_lower.contains("hash") && template.category == TemplateCategory::Hash {
            score += 0.5;
        }
        if name_lower.contains("lock")
            && matches!(
                template.category,
                TemplateCategory::SpinLock | TemplateCategory::Mutex
            )
        {
            score += 0.5;
        }

        // Performance requirements
        if spec.performance.no_alloc {
            for constraint in &template.constraints {
                if matches!(constraint, TemplateConstraint::NoAlloc) {
                    score += 0.2;
                }
            }
        }

        if score > 0.0 { Some(score) } else { None }
    }

    fn suggest_bindings(
        &self,
        template: &Template,
        spec: &Specification,
    ) -> BTreeMap<String, String> {
        let mut bindings = BTreeMap::new();

        for param in &template.parameters {
            if param.name == "name" {
                bindings.insert("name".into(), spec.name.clone());
            } else if let Some(default) = &param.default {
                bindings.insert(param.name.clone(), default.clone());
            }
        }

        bindings
    }

    /// Instantiate template with bindings
    pub fn instantiate(
        &self,
        template_id: u64,
        bindings: &BTreeMap<String, String>,
    ) -> Option<Instantiation> {
        let template = self.templates.get(&template_id)?;

        let mut code = template.code.clone();

        for (name, value) in bindings {
            code = code.replace(&format!("{{{{{}}}}}", name), value);
        }

        // Apply defaults for missing bindings
        for param in &template.parameters {
            if !bindings.contains_key(&param.name) {
                if let Some(default) = &param.default {
                    code = code.replace(&format!("{{{{{}}}}}", param.name), default);
                }
            }
        }

        Some(Instantiation {
            template_id,
            bindings: bindings.clone(),
            code,
        })
    }

    /// Get all templates
    pub fn all(&self) -> impl Iterator<Item = &Template> {
        self.templates.values()
    }

    /// Template count
    pub fn count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let lib = TemplateLibrary::new();
        assert!(lib.count() > 0);
    }

    #[test]
    fn test_get_by_name() {
        let lib = TemplateLibrary::new();
        let template = lib.get_by_name("spinlock");
        assert!(template.is_some());
    }

    #[test]
    fn test_get_by_category() {
        let lib = TemplateLibrary::new();
        let templates = lib.get_by_category(TemplateCategory::Search);
        assert!(!templates.is_empty());
    }

    #[test]
    fn test_instantiate() {
        let lib = TemplateLibrary::new();
        let template = lib.get_by_name("fnv1a_hash").unwrap();

        let mut bindings = BTreeMap::new();
        bindings.insert("name".into(), "my_hash".into());

        let result = lib.instantiate(template.id, &bindings);
        assert!(result.is_some());
        assert!(result.unwrap().code.contains("my_hash"));
    }
}
