//! # Type Inference
//!
//! Implements type inference and analysis.
//! Supports Hindley-Milner type inference.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// TYPE TYPES
// ============================================================================

/// Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Primitive type
    Primitive(PrimitiveType),
    /// Type variable
    Variable(u64),
    /// Function type
    Function { params: Vec<Type>, ret: Box<Type> },
    /// Generic type
    Generic { name: String, params: Vec<Type> },
    /// Tuple type
    Tuple(Vec<Type>),
    /// Array type
    Array { element: Box<Type>, size: Option<usize> },
    /// Reference type
    Reference { target: Box<Type>, mutable: bool },
    /// Option type
    Option(Box<Type>),
    /// Result type
    Result { ok: Box<Type>, err: Box<Type> },
    /// Unknown/unresolved
    Unknown,
}

/// Primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Char,
    Str,
    Unit,
}

/// Type constraint
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    /// Constraint ID
    pub id: u64,
    /// Kind
    pub kind: ConstraintKind,
    /// Location
    pub location: String,
}

/// Constraint kind
#[derive(Debug, Clone)]
pub enum ConstraintKind {
    Equal(Type, Type),
    Subtype(Type, Type),
    HasField { base: Type, field: String, field_type: Type },
    Implements { ty: Type, trait_name: String },
    Callable { ty: Type, args: Vec<Type>, ret: Type },
}

/// Type environment
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Bindings
    bindings: BTreeMap<String, Type>,
    /// Parent environment
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: TypeEnv) -> Self {
        Self {
            bindings: BTreeMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn bind(&mut self, name: &str, ty: Type) {
        self.bindings.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<Type> {
        if let Some(ty) = self.bindings.get(name) {
            Some(ty.clone())
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Substitution
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Mappings
    mappings: BTreeMap<u64, Type>,
}

impl Substitution {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Variable(id) => {
                if let Some(t) = self.mappings.get(id) {
                    self.apply(t)
                } else {
                    ty.clone()
                }
            }
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
            },
            Type::Generic { name, params } => Type::Generic {
                name: name.clone(),
                params: params.iter().map(|p| self.apply(p)).collect(),
            },
            Type::Tuple(types) => Type::Tuple(
                types.iter().map(|t| self.apply(t)).collect()
            ),
            Type::Array { element, size } => Type::Array {
                element: Box::new(self.apply(element)),
                size: *size,
            },
            Type::Reference { target, mutable } => Type::Reference {
                target: Box::new(self.apply(target)),
                mutable: *mutable,
            },
            Type::Option(inner) => Type::Option(Box::new(self.apply(inner))),
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(self.apply(ok)),
                err: Box::new(self.apply(err)),
            },
            _ => ty.clone(),
        }
    }

    pub fn extend(&mut self, var: u64, ty: Type) {
        self.mappings.insert(var, ty);
    }

    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();

        for (id, ty) in &self.mappings {
            result.mappings.insert(*id, other.apply(ty));
        }

        for (id, ty) in &other.mappings {
            if !result.mappings.contains_key(id) {
                result.mappings.insert(*id, ty.clone());
            }
        }

        result
    }
}

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Inferred type
    pub inferred_type: Type,
    /// Substitution
    pub substitution: Substitution,
    /// Constraints generated
    pub constraints: Vec<TypeConstraint>,
    /// Errors
    pub errors: Vec<TypeError>,
}

/// Type error
#[derive(Debug, Clone)]
pub struct TypeError {
    /// Error ID
    pub id: u64,
    /// Message
    pub message: String,
    /// Location
    pub location: String,
    /// Expected type
    pub expected: Option<Type>,
    /// Actual type
    pub actual: Option<Type>,
}

// ============================================================================
// TYPE INFERENCE ENGINE
// ============================================================================

/// Type inference engine
pub struct TypeInferenceEngine {
    /// Next type variable
    next_var: AtomicU64,
    /// Constraints
    constraints: Vec<TypeConstraint>,
    /// Substitution
    substitution: Substitution,
    /// Errors
    errors: Vec<TypeError>,
    /// Configuration
    config: InferenceConfig,
    /// Statistics
    stats: InferenceStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Maximum iterations for unification
    pub max_iterations: usize,
    /// Enable implicit coercion
    pub implicit_coercion: bool,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            implicit_coercion: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct InferenceStats {
    /// Variables created
    pub variables_created: u64,
    /// Constraints solved
    pub constraints_solved: u64,
    /// Unifications performed
    pub unifications_performed: u64,
}

impl TypeInferenceEngine {
    /// Create new engine
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            next_var: AtomicU64::new(1),
            constraints: Vec::new(),
            substitution: Substitution::new(),
            errors: Vec::new(),
            config,
            stats: InferenceStats::default(),
        }
    }

    /// Create fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let id = self.next_var.fetch_add(1, Ordering::Relaxed);
        self.stats.variables_created += 1;
        Type::Variable(id)
    }

    /// Add constraint
    pub fn constrain(&mut self, kind: ConstraintKind, location: &str) {
        let id = self.next_var.fetch_add(1, Ordering::Relaxed);

        self.constraints.push(TypeConstraint {
            id,
            kind,
            location: location.into(),
        });
    }

    /// Unify two types
    pub fn unify(&mut self, t1: &Type, t2: &Type) -> Result<Substitution, TypeError> {
        self.stats.unifications_performed += 1;

        let t1 = self.substitution.apply(t1);
        let t2 = self.substitution.apply(t2);

        match (&t1, &t2) {
            // Same types
            _ if t1 == t2 => Ok(Substitution::new()),

            // Variable unification
            (Type::Variable(id), _) => {
                if self.occurs_in(*id, &t2) {
                    Err(TypeError {
                        id: 0,
                        message: "Infinite type detected".into(),
                        location: String::new(),
                        expected: Some(t1),
                        actual: Some(t2),
                    })
                } else {
                    let mut s = Substitution::new();
                    s.extend(*id, t2.clone());
                    Ok(s)
                }
            }
            (_, Type::Variable(id)) => {
                if self.occurs_in(*id, &t1) {
                    Err(TypeError {
                        id: 0,
                        message: "Infinite type detected".into(),
                        location: String::new(),
                        expected: Some(t2),
                        actual: Some(t1),
                    })
                } else {
                    let mut s = Substitution::new();
                    s.extend(*id, t1.clone());
                    Ok(s)
                }
            }

            // Function types
            (Type::Function { params: p1, ret: r1 }, Type::Function { params: p2, ret: r2 }) => {
                if p1.len() != p2.len() {
                    return Err(TypeError {
                        id: 0,
                        message: "Function arity mismatch".into(),
                        location: String::new(),
                        expected: Some(t1),
                        actual: Some(t2),
                    });
                }

                let mut s = Substitution::new();

                for (pt1, pt2) in p1.iter().zip(p2.iter()) {
                    let s2 = self.unify(&s.apply(pt1), &s.apply(pt2))?;
                    s = s.compose(&s2);
                }

                let s2 = self.unify(&s.apply(r1), &s.apply(r2))?;
                Ok(s.compose(&s2))
            }

            // Generic types
            (Type::Generic { name: n1, params: p1 }, Type::Generic { name: n2, params: p2 }) => {
                if n1 != n2 || p1.len() != p2.len() {
                    return Err(TypeError {
                        id: 0,
                        message: format!("Type mismatch: {} vs {}", n1, n2),
                        location: String::new(),
                        expected: Some(t1),
                        actual: Some(t2),
                    });
                }

                let mut s = Substitution::new();
                for (pt1, pt2) in p1.iter().zip(p2.iter()) {
                    let s2 = self.unify(&s.apply(pt1), &s.apply(pt2))?;
                    s = s.compose(&s2);
                }

                Ok(s)
            }

            // Tuple types
            (Type::Tuple(t1), Type::Tuple(t2)) => {
                if t1.len() != t2.len() {
                    return Err(TypeError {
                        id: 0,
                        message: "Tuple length mismatch".into(),
                        location: String::new(),
                        expected: None,
                        actual: None,
                    });
                }

                let mut s = Substitution::new();
                for (e1, e2) in t1.iter().zip(t2.iter()) {
                    let s2 = self.unify(&s.apply(e1), &s.apply(e2))?;
                    s = s.compose(&s2);
                }

                Ok(s)
            }

            // Array types
            (Type::Array { element: e1, size: s1 }, Type::Array { element: e2, size: s2 }) => {
                if s1 != s2 {
                    return Err(TypeError {
                        id: 0,
                        message: "Array size mismatch".into(),
                        location: String::new(),
                        expected: None,
                        actual: None,
                    });
                }

                self.unify(e1, e2)
            }

            // Option types
            (Type::Option(inner1), Type::Option(inner2)) => {
                self.unify(inner1, inner2)
            }

            // Result types
            (Type::Result { ok: o1, err: e1 }, Type::Result { ok: o2, err: e2 }) => {
                let s1 = self.unify(o1, o2)?;
                let s2 = self.unify(&s1.apply(e1), &s1.apply(e2))?;
                Ok(s1.compose(&s2))
            }

            // Unknown can unify with anything
            (Type::Unknown, _) | (_, Type::Unknown) => Ok(Substitution::new()),

            // No match
            _ => Err(TypeError {
                id: 0,
                message: "Type mismatch".into(),
                location: String::new(),
                expected: Some(t1),
                actual: Some(t2),
            }),
        }
    }

    fn occurs_in(&self, var: u64, ty: &Type) -> bool {
        match ty {
            Type::Variable(id) => *id == var,
            Type::Function { params, ret } => {
                params.iter().any(|p| self.occurs_in(var, p)) || self.occurs_in(var, ret)
            }
            Type::Generic { params, .. } => params.iter().any(|p| self.occurs_in(var, p)),
            Type::Tuple(types) => types.iter().any(|t| self.occurs_in(var, t)),
            Type::Array { element, .. } => self.occurs_in(var, element),
            Type::Reference { target, .. } => self.occurs_in(var, target),
            Type::Option(inner) => self.occurs_in(var, inner),
            Type::Result { ok, err } => self.occurs_in(var, ok) || self.occurs_in(var, err),
            _ => false,
        }
    }

    /// Solve constraints
    pub fn solve(&mut self) -> Result<Substitution, Vec<TypeError>> {
        let constraints = core::mem::take(&mut self.constraints);
        let mut errors = Vec::new();

        for constraint in constraints {
            match &constraint.kind {
                ConstraintKind::Equal(t1, t2) => {
                    match self.unify(t1, t2) {
                        Ok(s) => {
                            self.substitution = self.substitution.compose(&s);
                            self.stats.constraints_solved += 1;
                        }
                        Err(mut e) => {
                            e.location = constraint.location.clone();
                            errors.push(e);
                        }
                    }
                }
                ConstraintKind::Subtype(_, _) => {
                    // Subtyping not fully implemented
                    self.stats.constraints_solved += 1;
                }
                ConstraintKind::HasField { base, field, field_type } => {
                    // Field access type inference
                    if let Type::Generic { name: _, params: _ } = base {
                        // Would look up field in type definition
                    }
                    self.stats.constraints_solved += 1;
                }
                ConstraintKind::Implements { ty: _, trait_name: _ } => {
                    // Trait bound checking
                    self.stats.constraints_solved += 1;
                }
                ConstraintKind::Callable { ty, args, ret } => {
                    let func_type = Type::Function {
                        params: args.clone(),
                        ret: Box::new(ret.clone()),
                    };

                    match self.unify(ty, &func_type) {
                        Ok(s) => {
                            self.substitution = self.substitution.compose(&s);
                            self.stats.constraints_solved += 1;
                        }
                        Err(mut e) => {
                            e.location = constraint.location.clone();
                            errors.push(e);
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(self.substitution.clone())
        } else {
            Err(errors)
        }
    }

    /// Infer type of literal
    pub fn infer_literal(&mut self, value: &str) -> Type {
        if value == "true" || value == "false" {
            Type::Primitive(PrimitiveType::Bool)
        } else if value.contains('.') {
            Type::Primitive(PrimitiveType::F64)
        } else if value.starts_with('"') {
            Type::Primitive(PrimitiveType::Str)
        } else if value.starts_with('\'') {
            Type::Primitive(PrimitiveType::Char)
        } else if value.parse::<i64>().is_ok() {
            Type::Primitive(PrimitiveType::I32)
        } else {
            self.fresh_var()
        }
    }

    /// Get substitution
    pub fn substitution(&self) -> &Substitution {
        &self.substitution
    }

    /// Get statistics
    pub fn stats(&self) -> &InferenceStats {
        &self.stats
    }

    /// Reset engine
    pub fn reset(&mut self) {
        self.constraints.clear();
        self.substitution = Substitution::new();
        self.errors.clear();
    }
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new(InferenceConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_var() {
        let mut engine = TypeInferenceEngine::default();

        let v1 = engine.fresh_var();
        let v2 = engine.fresh_var();

        assert_ne!(v1, v2);
    }

    #[test]
    fn test_unify_same() {
        let mut engine = TypeInferenceEngine::default();

        let t1 = Type::Primitive(PrimitiveType::I32);
        let t2 = Type::Primitive(PrimitiveType::I32);

        let result = engine.unify(&t1, &t2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_variable() {
        let mut engine = TypeInferenceEngine::default();

        let var = engine.fresh_var();
        let concrete = Type::Primitive(PrimitiveType::Bool);

        let result = engine.unify(&var, &concrete);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert_eq!(subst.apply(&var), concrete);
    }

    #[test]
    fn test_unify_function() {
        let mut engine = TypeInferenceEngine::default();

        let f1 = Type::Function {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        };

        let f2 = Type::Function {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        };

        let result = engine.unify(&f1, &f2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_mismatch() {
        let mut engine = TypeInferenceEngine::default();

        let t1 = Type::Primitive(PrimitiveType::I32);
        let t2 = Type::Primitive(PrimitiveType::Bool);

        let result = engine.unify(&t1, &t2);
        assert!(result.is_err());
    }

    #[test]
    fn test_solve_constraints() {
        let mut engine = TypeInferenceEngine::default();

        let var = engine.fresh_var();
        engine.constrain(
            ConstraintKind::Equal(var.clone(), Type::Primitive(PrimitiveType::I64)),
            "test",
        );

        let result = engine.solve();
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert_eq!(subst.apply(&var), Type::Primitive(PrimitiveType::I64));
    }

    #[test]
    fn test_type_env() {
        let mut env = TypeEnv::new();

        env.bind("x", Type::Primitive(PrimitiveType::I32));

        assert_eq!(env.lookup("x"), Some(Type::Primitive(PrimitiveType::I32)));
        assert_eq!(env.lookup("y"), None);
    }

    #[test]
    fn test_infer_literal() {
        let mut engine = TypeInferenceEngine::default();

        assert_eq!(engine.infer_literal("true"), Type::Primitive(PrimitiveType::Bool));
        assert_eq!(engine.infer_literal("42"), Type::Primitive(PrimitiveType::I32));
        assert_eq!(engine.infer_literal("3.14"), Type::Primitive(PrimitiveType::F64));
    }
}
