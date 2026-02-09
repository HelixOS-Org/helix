//! # Code Patterns
//!
//! Year 3 EVOLUTION - Common code patterns and templates for generation

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::ast::*;
use super::types::*;

// ============================================================================
// PATTERN IDENTIFICATION
// ============================================================================

/// Pattern ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatternId(pub u64);

static PATTERN_COUNTER: AtomicU64 = AtomicU64::new(1);

impl PatternId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(PATTERN_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

// ============================================================================
// CODE PATTERNS
// ============================================================================

/// Code pattern
#[derive(Debug, Clone)]
pub struct CodePattern {
    /// Pattern ID
    pub id: PatternId,
    /// Name
    pub name: String,
    /// Category
    pub category: PatternCategory,
    /// Description
    pub description: String,
    /// Parameters
    pub params: Vec<PatternParam>,
    /// Template
    pub template: PatternTemplate,
    /// Constraints
    pub constraints: Vec<PatternConstraint>,
    /// Usage count
    pub usage_count: u64,
}

/// Pattern category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCategory {
    /// Creational patterns
    Creational,
    /// Structural patterns
    Structural,
    /// Behavioral patterns
    Behavioral,
    /// Concurrency patterns
    Concurrency,
    /// Functional patterns
    Functional,
    /// Error handling patterns
    ErrorHandling,
    /// Resource management
    ResourceManagement,
    /// Optimization patterns
    Optimization,
    /// Testing patterns
    Testing,
}

/// Pattern parameter
#[derive(Debug, Clone)]
pub struct PatternParam {
    /// Name
    pub name: String,
    /// Kind
    pub kind: ParamKind,
    /// Description
    pub description: String,
    /// Default value
    pub default: Option<PatternValue>,
    /// Required
    pub required: bool,
}

/// Parameter kind
#[derive(Debug, Clone)]
pub enum ParamKind {
    /// Type parameter
    Type,
    /// Name/identifier
    Name,
    /// Expression
    Expr,
    /// Statement list
    Statements,
    /// Integer value
    Integer,
    /// Boolean flag
    Boolean,
    /// String value
    String,
    /// List of items
    List(Box<ParamKind>),
}

/// Pattern value
#[derive(Debug, Clone)]
pub enum PatternValue {
    Type(Type),
    Name(String),
    Expr(AstNode),
    Statements(Vec<AstNode>),
    Integer(i64),
    Boolean(bool),
    String(String),
    List(Vec<PatternValue>),
}

/// Pattern template
#[derive(Debug, Clone)]
pub enum PatternTemplate {
    /// AST template
    Ast(Vec<TemplateNode>),
    /// Code string with placeholders
    Code(String),
    /// Composite (multiple templates)
    Composite(Vec<PatternTemplate>),
}

/// Template node
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// Literal AST node
    Literal(AstNode),
    /// Placeholder for parameter
    Placeholder(String),
    /// Conditional inclusion
    Conditional {
        param: String,
        then_part: Box<TemplateNode>,
        else_part: Option<Box<TemplateNode>>,
    },
    /// Repeated for list parameter
    Repeat {
        param: String,
        template: Box<TemplateNode>,
        separator: Option<String>,
    },
    /// Nested pattern
    Pattern {
        id: PatternId,
        args: Vec<(String, PatternValue)>,
    },
}

/// Pattern constraint
#[derive(Debug, Clone)]
pub enum PatternConstraint {
    /// Type must implement trait
    TypeImplements { param: String, trait_name: String },
    /// Type must be sized
    TypeSized(String),
    /// Type must be Send
    TypeSend(String),
    /// Type must be Sync
    TypeSync(String),
    /// Custom constraint
    Custom { name: String, params: Vec<String> },
}

// ============================================================================
// BUILT-IN PATTERNS
// ============================================================================

/// Pattern library
pub struct PatternLibrary {
    /// Patterns by ID
    patterns: BTreeMap<PatternId, CodePattern>,
    /// Patterns by name
    by_name: BTreeMap<String, PatternId>,
    /// Patterns by category
    by_category: BTreeMap<PatternCategory, Vec<PatternId>>,
}

impl PatternLibrary {
    /// Create new library with built-in patterns
    pub fn new() -> Self {
        let mut lib = Self {
            patterns: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_category: BTreeMap::new(),
        };

        lib.register_builtin_patterns();
        lib
    }

    fn register_builtin_patterns(&mut self) {
        // Builder pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("Builder"),
            category: PatternCategory::Creational,
            description: String::from("Builder pattern for complex object construction"),
            params: vec![
                PatternParam {
                    name: String::from("target_type"),
                    kind: ParamKind::Name,
                    description: String::from("Type being built"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("fields"),
                    kind: ParamKind::List(Box::new(ParamKind::Name)),
                    description: String::from("Fields to set via builder"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
pub struct ${target_type}Builder {
    ${fields:field: Option<${field_type}>,}
}

impl ${target_type}Builder {
    pub fn new() -> Self {
        Self {
            ${fields:field: None,}
        }
    }

    ${fields:
    pub fn ${field}(mut self, value: ${field_type}) -> Self {
        self.${field} = Some(value);
        self
    }
    }

    #[inline]
    pub fn build(self) -> Result<${target_type}, &'static str> {
        Ok(${target_type} {
            ${fields:${field}: self.${field}.ok_or("${field} is required")?,}
        })
    }
}
"#,
            )),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // Singleton pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("Singleton"),
            category: PatternCategory::Creational,
            description: String::from("Thread-safe singleton pattern"),
            params: vec![PatternParam {
                name: String::from("type_name"),
                kind: ParamKind::Name,
                description: String::from("Singleton type name"),
                default: None,
                required: true,
            }],
            template: PatternTemplate::Code(String::from(
                r#"
use core::sync::atomic::{AtomicPtr, Ordering};
use alloc::boxed::Box;

static INSTANCE: AtomicPtr<${type_name}> = AtomicPtr::new(core::ptr::null_mut());

impl ${type_name} {
    pub fn instance() -> &'static ${type_name} {
        let ptr = INSTANCE.load(Ordering::Acquire);
        if ptr.is_null() {
            let new = Box::into_raw(Box::new(${type_name}::new()));
            match INSTANCE.compare_exchange(
                core::ptr::null_mut(),
                new,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => unsafe { &*new },
                Err(existing) => {
                    unsafe { drop(Box::from_raw(new)); }
                    unsafe { &*existing }
                }
            }
        } else {
            unsafe { &*ptr }
        }
    }
}
"#,
            )),
            constraints: vec![
                PatternConstraint::TypeSend(String::from("type_name")),
                PatternConstraint::TypeSync(String::from("type_name")),
            ],
            usage_count: 0,
        });

        // Observer pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("Observer"),
            category: PatternCategory::Behavioral,
            description: String::from("Observer pattern for event-driven programming"),
            params: vec![
                PatternParam {
                    name: String::from("subject_name"),
                    kind: ParamKind::Name,
                    description: String::from("Subject type name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("event_type"),
                    kind: ParamKind::Name,
                    description: String::from("Event type"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
pub trait ${subject_name}Observer: Send + Sync {
    fn on_event(&self, event: &${event_type});
}

pub struct ${subject_name} {
    observers: Vec<Box<dyn ${subject_name}Observer>>,
}

impl ${subject_name} {
    pub fn new() -> Self {
        Self { observers: Vec::new() }
    }

    #[inline(always)]
    pub fn subscribe(&mut self, observer: Box<dyn ${subject_name}Observer>) {
        self.observers.push(observer);
    }

    #[inline]
    pub fn notify(&self, event: &${event_type}) {
        for observer in &self.observers {
            observer.on_event(event);
        }
    }
}
"#,
            )),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // State machine pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("StateMachine"),
            category: PatternCategory::Behavioral,
            description: String::from("Type-safe state machine pattern"),
            params: vec![
                PatternParam {
                    name: String::from("machine_name"),
                    kind: ParamKind::Name,
                    description: String::from("State machine name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("states"),
                    kind: ParamKind::List(Box::new(ParamKind::Name)),
                    description: String::from("State names"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("events"),
                    kind: ParamKind::List(Box::new(ParamKind::Name)),
                    description: String::from("Event names"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(r#"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ${machine_name}State {
    ${states:${state},}
}

#[derive(Debug, Clone)]
pub enum ${machine_name}Event {
    ${events:${event},}
}

pub struct ${machine_name} {
    state: ${machine_name}State,
}

impl ${machine_name} {
    pub fn new(initial: ${machine_name}State) -> Self {
        Self { state: initial }
    }

    #[inline(always)]
    pub fn state(&self) -> ${machine_name}State {
        self.state
    }

    #[inline(always)]
    pub fn transition(&mut self, event: ${machine_name}Event) -> Result<(), &'static str> {
        self.state = self.next_state(event)?;
        Ok(())
    }

    fn next_state(&self, event: ${machine_name}Event) -> Result<${machine_name}State, &'static str> {
        match (self.state, event) {
            // Define transitions here
            _ => Err("Invalid transition"),
        }
    }
}
"#)),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // RAII pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("RAII"),
            category: PatternCategory::ResourceManagement,
            description: String::from("Resource Acquisition Is Initialization pattern"),
            params: vec![
                PatternParam {
                    name: String::from("guard_name"),
                    kind: ParamKind::Name,
                    description: String::from("Guard type name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("resource_type"),
                    kind: ParamKind::Name,
                    description: String::from("Resource type"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
pub struct ${guard_name} {
    resource: ${resource_type},
}

impl ${guard_name} {
    #[inline(always)]
    pub fn acquire(resource: ${resource_type}) -> Self {
        // Acquisition logic here
        Self { resource }
    }

    #[inline(always)]
    pub fn get(&self) -> &${resource_type} {
        &self.resource
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut ${resource_type} {
        &mut self.resource
    }
}

impl Drop for ${guard_name} {
    fn drop(&mut self) {
        // Release logic here
    }
}
"#,
            )),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // Result chain pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("ResultChain"),
            category: PatternCategory::ErrorHandling,
            description: String::from("Chainable error handling pattern"),
            params: vec![
                PatternParam {
                    name: String::from("error_type"),
                    kind: ParamKind::Name,
                    description: String::from("Error type name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("variants"),
                    kind: ParamKind::List(Box::new(ParamKind::Name)),
                    description: String::from("Error variants"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
#[derive(Debug)]
pub enum ${error_type} {
    ${variants:${variant},}
}

impl core::fmt::Display for ${error_type} {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ${variants:Self::${variant} => write!(f, "${variant}"),}
        }
    }
}

pub type Result<T> = core::result::Result<T, ${error_type}>;

pub trait ResultExt<T> {
    fn context(self, msg: &str) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, _msg: &str) -> Result<T> {
        self
    }
}
"#,
            )),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // Iterator adapter pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("IteratorAdapter"),
            category: PatternCategory::Functional,
            description: String::from("Custom iterator adapter pattern"),
            params: vec![
                PatternParam {
                    name: String::from("adapter_name"),
                    kind: ParamKind::Name,
                    description: String::from("Adapter type name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("item_type"),
                    kind: ParamKind::Name,
                    description: String::from("Item type"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
pub struct ${adapter_name}<I> {
    iter: I,
}

impl<I> ${adapter_name}<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Iterator for ${adapter_name}<I>
where
    I: Iterator<Item = ${item_type}>,
{
    type Item = ${item_type};

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| {
            // Transform item here
            item
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub trait ${adapter_name}Ext: Iterator<Item = ${item_type}> + Sized {
    fn ${adapter_name_lower}(self) -> ${adapter_name}<Self> {
        ${adapter_name}::new(self)
    }
}

impl<I: Iterator<Item = ${item_type}>> ${adapter_name}Ext for I {}
"#,
            )),
            constraints: Vec::new(),
            usage_count: 0,
        });

        // Lock-free queue pattern
        self.register(CodePattern {
            id: PatternId::generate(),
            name: String::from("LockFreeQueue"),
            category: PatternCategory::Concurrency,
            description: String::from("Lock-free concurrent queue"),
            params: vec![
                PatternParam {
                    name: String::from("queue_name"),
                    kind: ParamKind::Name,
                    description: String::from("Queue type name"),
                    default: None,
                    required: true,
                },
                PatternParam {
                    name: String::from("item_type"),
                    kind: ParamKind::Name,
                    description: String::from("Item type"),
                    default: None,
                    required: true,
                },
            ],
            template: PatternTemplate::Code(String::from(
                r#"
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::boxed::Box;

struct Node<T> {
    value: Option<T>,
    next: AtomicUsize,
}

pub struct ${queue_name} {
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl ${queue_name} {
    pub fn new() -> Self {
        let sentinel = Box::into_raw(Box::new(Node {
            value: None,
            next: AtomicUsize::new(0),
        })) as usize;

        Self {
            head: AtomicUsize::new(sentinel),
            tail: AtomicUsize::new(sentinel),
        }
    }

    pub fn push(&self, value: ${item_type}) {
        let node = Box::into_raw(Box::new(Node {
            value: Some(value),
            next: AtomicUsize::new(0),
        })) as usize;

        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_node = unsafe { &*(tail as *const Node<${item_type}>) };
            let next = tail_node.next.load(Ordering::Acquire);

            if next == 0 {
                if tail_node.next.compare_exchange(
                    0, node, Ordering::Release, Ordering::Relaxed
                ).is_ok() {
                    let _ = self.tail.compare_exchange(
                        tail, node, Ordering::Release, Ordering::Relaxed
                    );
                    return;
                }
            } else {
                let _ = self.tail.compare_exchange(
                    tail, next, Ordering::Release, Ordering::Relaxed
                );
            }
        }
    }

    pub fn pop(&self) -> Option<${item_type}> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let head_node = unsafe { &*(head as *const Node<${item_type}>) };
            let next = head_node.next.load(Ordering::Acquire);

            if head == tail {
                if next == 0 {
                    return None;
                }
                let _ = self.tail.compare_exchange(
                    tail, next, Ordering::Release, Ordering::Relaxed
                );
            } else {
                let next_node = unsafe { &*(next as *const Node<${item_type}>) };
                if self.head.compare_exchange(
                    head, next, Ordering::Release, Ordering::Relaxed
                ).is_ok() {
                    let value = next_node.value.clone();
                    unsafe { drop(Box::from_raw(head as *mut Node<${item_type}>)); }
                    return value;
                }
            }
        }
    }
}
"#,
            )),
            constraints: vec![PatternConstraint::TypeSend(String::from("item_type"))],
            usage_count: 0,
        });
    }

    /// Register pattern
    #[inline]
    pub fn register(&mut self, pattern: CodePattern) {
        let id = pattern.id;
        let name = pattern.name.clone();
        let category = pattern.category;

        self.patterns.insert(id, pattern);
        self.by_name.insert(name, id);
        self.by_category.entry(category).or_default().push(id);
    }

    /// Get pattern by ID
    #[inline(always)]
    pub fn get(&self, id: PatternId) -> Option<&CodePattern> {
        self.patterns.get(&id)
    }

    /// Get pattern by name
    #[inline(always)]
    pub fn get_by_name(&self, name: &str) -> Option<&CodePattern> {
        self.by_name.get(name).and_then(|id| self.patterns.get(id))
    }

    /// Get patterns by category
    #[inline]
    pub fn get_by_category(&self, category: PatternCategory) -> Vec<&CodePattern> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.patterns.get(id)).collect())
            .unwrap_or_default()
    }

    /// Increment usage count
    #[inline]
    pub fn record_usage(&mut self, id: PatternId) {
        if let Some(pattern) = self.patterns.get_mut(&id) {
            pattern.usage_count += 1;
        }
    }

    /// Get most used patterns
    #[inline]
    pub fn most_used(&self, count: usize) -> Vec<&CodePattern> {
        let mut patterns: Vec<_> = self.patterns.values().collect();
        patterns.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        patterns.into_iter().take(count).collect()
    }
}

impl Default for PatternLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PATTERN INSTANTIATION
// ============================================================================

/// Pattern instantiator
pub struct PatternInstantiator<'a> {
    library: &'a PatternLibrary,
}

impl<'a> PatternInstantiator<'a> {
    pub fn new(library: &'a PatternLibrary) -> Self {
        Self { library }
    }

    /// Instantiate pattern
    pub fn instantiate(
        &self,
        pattern_name: &str,
        args: BTreeMap<String, PatternValue>,
    ) -> Result<String, InstantiateError> {
        let pattern = self
            .library
            .get_by_name(pattern_name)
            .ok_or(InstantiateError::PatternNotFound)?;

        // Check required parameters
        for param in &pattern.params {
            if param.required && !args.contains_key(&param.name) {
                return Err(InstantiateError::MissingParameter(param.name.clone()));
            }
        }

        // Expand template
        match &pattern.template {
            PatternTemplate::Code(code) => self.expand_code_template(code, &args),
            PatternTemplate::Ast(_nodes) => {
                // Would generate AST and then pretty-print
                Err(InstantiateError::UnsupportedTemplate)
            },
            PatternTemplate::Composite(_templates) => Err(InstantiateError::UnsupportedTemplate),
        }
    }

    fn expand_code_template(
        &self,
        template: &str,
        args: &BTreeMap<String, PatternValue>,
    ) -> Result<String, InstantiateError> {
        let mut result = template.to_string();

        // Simple placeholder replacement ${name}
        for (name, value) in args {
            let placeholder = format!("${{{}}}", name);
            let replacement = self.value_to_string(value);
            result = result.replace(&placeholder, &replacement);
        }

        Ok(result)
    }

    fn value_to_string(&self, value: &PatternValue) -> String {
        match value {
            PatternValue::Name(s) => s.clone(),
            PatternValue::String(s) => s.clone(),
            PatternValue::Integer(i) => alloc::format!("{}", i),
            PatternValue::Boolean(b) => alloc::format!("{}", b),
            PatternValue::List(items) => items
                .iter()
                .map(|v| self.value_to_string(v))
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        }
    }
}

/// Instantiation error
#[derive(Debug)]
pub enum InstantiateError {
    PatternNotFound,
    MissingParameter(String),
    InvalidParameter(String),
    ConstraintViolation(String),
    UnsupportedTemplate,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_library() {
        let lib = PatternLibrary::new();
        assert!(lib.get_by_name("Builder").is_some());
        assert!(lib.get_by_name("Singleton").is_some());
    }

    #[test]
    fn test_get_by_category() {
        let lib = PatternLibrary::new();
        let creational = lib.get_by_category(PatternCategory::Creational);
        assert!(creational.len() >= 2);
    }

    #[test]
    fn test_usage_tracking() {
        let mut lib = PatternLibrary::new();
        let id = *lib.by_name.get("Builder").unwrap();

        lib.record_usage(id);
        lib.record_usage(id);

        assert_eq!(lib.get(id).unwrap().usage_count, 2);
    }
}
