//! # Type Inference Engine
//!
//! Infers and propagates types through code analysis.
//! Implements Hindley-Milner type inference with extensions.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// TYPE REPRESENTATION
// ============================================================================

/// Type variable or concrete type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Type variable (unresolved)
    Var(u64),
    /// Primitive type
    Primitive(PrimitiveType),
    /// Function type
    Function { params: Vec<Type>, ret: Box<Type> },
    /// Tuple type
    Tuple(Vec<Type>),
    /// Array type
    Array(Box<Type>, Option<usize>),
    /// Reference type
    Ref { inner: Box<Type>, mutable: bool },
    /// Option type
    Option(Box<Type>),
    /// Result type
    Result { ok: Box<Type>, err: Box<Type> },
    /// Custom type
    Custom { name: String, params: Vec<Type> },
    /// Never type
    Never,
    /// Unit type
    Unit,
}

/// Primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Char,
    Str,
}

impl Type {
    /// Check if type contains variable
    pub fn contains_var(&self, var: u64) -> bool {
        match self {
            Type::Var(v) => *v == var,
            Type::Function { params, ret } => {
                params.iter().any(|p| p.contains_var(var)) || ret.contains_var(var)
            },
            Type::Tuple(types) => types.iter().any(|t| t.contains_var(var)),
            Type::Array(t, _) => t.contains_var(var),
            Type::Ref { inner, .. } => inner.contains_var(var),
            Type::Option(t) => t.contains_var(var),
            Type::Result { ok, err } => ok.contains_var(var) || err.contains_var(var),
            Type::Custom { params, .. } => params.iter().any(|p| p.contains_var(var)),
            _ => false,
        }
    }

    /// Substitute type variable
    pub fn substitute(&self, var: u64, replacement: &Type) -> Type {
        match self {
            Type::Var(v) if *v == var => replacement.clone(),
            Type::Var(_) => self.clone(),
            Type::Function { params, ret } => Type::Function {
                params: params
                    .iter()
                    .map(|p| p.substitute(var, replacement))
                    .collect(),
                ret: Box::new(ret.substitute(var, replacement)),
            },
            Type::Tuple(types) => Type::Tuple(
                types
                    .iter()
                    .map(|t| t.substitute(var, replacement))
                    .collect(),
            ),
            Type::Array(t, size) => Type::Array(Box::new(t.substitute(var, replacement)), *size),
            Type::Ref { inner, mutable } => Type::Ref {
                inner: Box::new(inner.substitute(var, replacement)),
                mutable: *mutable,
            },
            Type::Option(t) => Type::Option(Box::new(t.substitute(var, replacement))),
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(ok.substitute(var, replacement)),
                err: Box::new(err.substitute(var, replacement)),
            },
            Type::Custom { name, params } => Type::Custom {
                name: name.clone(),
                params: params
                    .iter()
                    .map(|p| p.substitute(var, replacement))
                    .collect(),
            },
            _ => self.clone(),
        }
    }
}

// ============================================================================
// SUBSTITUTION
// ============================================================================

/// Type substitution
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Mappings
    mappings: BTreeMap<u64, Type>,
}

impl Substitution {
    /// Create empty substitution
    pub fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Add mapping
    pub fn insert(&mut self, var: u64, ty: Type) {
        self.mappings.insert(var, ty);
    }

    /// Get mapping
    pub fn get(&self, var: u64) -> Option<&Type> {
        self.mappings.get(&var)
    }

    /// Apply substitution to type
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => self
                .mappings
                .get(v)
                .map(|t| self.apply(t))
                .unwrap_or_else(|| ty.clone()),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
            },
            Type::Tuple(types) => Type::Tuple(types.iter().map(|t| self.apply(t)).collect()),
            Type::Array(t, size) => Type::Array(Box::new(self.apply(t)), *size),
            Type::Ref { inner, mutable } => Type::Ref {
                inner: Box::new(self.apply(inner)),
                mutable: *mutable,
            },
            Type::Option(t) => Type::Option(Box::new(self.apply(t))),
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(self.apply(ok)),
                err: Box::new(self.apply(err)),
            },
            Type::Custom { name, params } => Type::Custom {
                name: name.clone(),
                params: params.iter().map(|p| self.apply(p)).collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Compose substitutions
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();

        for (&var, ty) in &other.mappings {
            result.insert(var, self.apply(ty));
        }

        for (&var, ty) in &self.mappings {
            if !result.mappings.contains_key(&var) {
                result.insert(var, ty.clone());
            }
        }

        result
    }
}

// ============================================================================
// CONSTRAINT
// ============================================================================

/// Type constraint
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Types must be equal
    Equal(Type, Type),
    /// Type must be instance of trait
    Instance(Type, String),
    /// Subtype relationship
    Subtype(Type, Type),
}

// ============================================================================
// TYPE ENVIRONMENT
// ============================================================================

/// Type environment (variable -> type scheme)
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// Bindings
    bindings: BTreeMap<String, TypeScheme>,
}

/// Type scheme (forall quantified type)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    /// Quantified variables
    pub vars: Vec<u64>,
    /// Inner type
    pub ty: Type,
}

impl TypeEnv {
    /// Create empty environment
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
    }

    /// Extend environment
    pub fn extend(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    /// Lookup variable
    pub fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        self.bindings.get(name)
    }

    /// Remove binding
    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
    }
}

// ============================================================================
// TYPE INFERENCE ENGINE
// ============================================================================

/// Type inference engine
pub struct TypeInferencer {
    /// Next type variable
    next_var: AtomicU64,
    /// Current substitution
    substitution: Substitution,
    /// Constraints
    constraints: Vec<Constraint>,
    /// Type environment
    env: TypeEnv,
    /// Statistics
    stats: InferenceStats,
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct InferenceStats {
    /// Variables created
    pub vars_created: u64,
    /// Unifications performed
    pub unifications: u64,
    /// Constraints solved
    pub constraints_solved: u64,
    /// Errors
    pub errors: u64,
}

/// Inference error
#[derive(Debug, Clone)]
pub enum InferenceError {
    /// Types cannot be unified
    UnificationFailed(Type, Type),
    /// Occurs check failed (infinite type)
    OccursCheck(u64, Type),
    /// Undefined variable
    UndefinedVariable(String),
    /// Constraint unsatisfied
    ConstraintUnsatisfied(Constraint),
}

impl TypeInferencer {
    /// Create new inferencer
    pub fn new() -> Self {
        Self {
            next_var: AtomicU64::new(1),
            substitution: Substitution::new(),
            constraints: Vec::new(),
            env: TypeEnv::new(),
            stats: InferenceStats::default(),
        }
    }

    /// Create fresh type variable
    pub fn fresh_var(&self) -> Type {
        Type::Var(self.next_var.fetch_add(1, Ordering::Relaxed))
    }

    /// Add constraint
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Unify two types
    pub fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), InferenceError> {
        let t1 = self.substitution.apply(t1);
        let t2 = self.substitution.apply(t2);

        self.stats.unifications += 1;

        match (&t1, &t2) {
            // Same type
            (a, b) if a == b => Ok(()),

            // Type variable on left
            (Type::Var(v), t) => {
                if t.contains_var(*v) {
                    Err(InferenceError::OccursCheck(*v, t.clone()))
                } else {
                    self.substitution.insert(*v, t.clone());
                    Ok(())
                }
            },

            // Type variable on right
            (t, Type::Var(v)) => {
                if t.contains_var(*v) {
                    Err(InferenceError::OccursCheck(*v, t.clone()))
                } else {
                    self.substitution.insert(*v, t.clone());
                    Ok(())
                }
            },

            // Function types
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(InferenceError::UnificationFailed(t1.clone(), t2.clone()));
                }
                for (a, b) in p1.iter().zip(p2.iter()) {
                    self.unify(a, b)?;
                }
                self.unify(r1, r2)
            },

            // Tuple types
            (Type::Tuple(ts1), Type::Tuple(ts2)) => {
                if ts1.len() != ts2.len() {
                    return Err(InferenceError::UnificationFailed(t1.clone(), t2.clone()));
                }
                for (a, b) in ts1.iter().zip(ts2.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            },

            // Array types
            (Type::Array(e1, s1), Type::Array(e2, s2)) => {
                if s1 != s2 {
                    return Err(InferenceError::UnificationFailed(t1.clone(), t2.clone()));
                }
                self.unify(e1, e2)
            },

            // Reference types
            (
                Type::Ref {
                    inner: i1,
                    mutable: m1,
                },
                Type::Ref {
                    inner: i2,
                    mutable: m2,
                },
            ) => {
                if m1 != m2 {
                    return Err(InferenceError::UnificationFailed(t1.clone(), t2.clone()));
                }
                self.unify(i1, i2)
            },

            // Option types
            (Type::Option(t1), Type::Option(t2)) => self.unify(t1, t2),

            // Result types
            (Type::Result { ok: o1, err: e1 }, Type::Result { ok: o2, err: e2 }) => {
                self.unify(o1, o2)?;
                self.unify(e1, e2)
            },

            // Custom types
            (
                Type::Custom {
                    name: n1,
                    params: p1,
                },
                Type::Custom {
                    name: n2,
                    params: p2,
                },
            ) => {
                if n1 != n2 || p1.len() != p2.len() {
                    return Err(InferenceError::UnificationFailed(t1.clone(), t2.clone()));
                }
                for (a, b) in p1.iter().zip(p2.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            },

            // Cannot unify
            _ => Err(InferenceError::UnificationFailed(t1, t2)),
        }
    }

    /// Solve all constraints
    pub fn solve_constraints(&mut self) -> Result<(), InferenceError> {
        while let Some(constraint) = self.constraints.pop() {
            self.stats.constraints_solved += 1;

            match constraint {
                Constraint::Equal(t1, t2) => {
                    self.unify(&t1, &t2)?;
                },
                Constraint::Instance(_, _) => {
                    // Trait constraint - simplified for now
                },
                Constraint::Subtype(t1, t2) => {
                    // Subtype - simplified as equality
                    self.unify(&t1, &t2)?;
                },
            }
        }
        Ok(())
    }

    /// Generalize type in environment
    pub fn generalize(&self, ty: &Type) -> TypeScheme {
        // Find free type variables
        let free_vars = self.free_vars(ty);

        TypeScheme {
            vars: free_vars,
            ty: ty.clone(),
        }
    }

    fn free_vars(&self, ty: &Type) -> Vec<u64> {
        let mut vars = Vec::new();
        self.collect_vars(ty, &mut vars);
        vars
    }

    fn collect_vars(&self, ty: &Type, vars: &mut Vec<u64>) {
        match ty {
            Type::Var(v) => {
                if !vars.contains(v) {
                    vars.push(*v);
                }
            },
            Type::Function { params, ret } => {
                for p in params {
                    self.collect_vars(p, vars);
                }
                self.collect_vars(ret, vars);
            },
            Type::Tuple(types) => {
                for t in types {
                    self.collect_vars(t, vars);
                }
            },
            Type::Array(t, _) => self.collect_vars(t, vars),
            Type::Ref { inner, .. } => self.collect_vars(inner, vars),
            Type::Option(t) => self.collect_vars(t, vars),
            Type::Result { ok, err } => {
                self.collect_vars(ok, vars);
                self.collect_vars(err, vars);
            },
            Type::Custom { params, .. } => {
                for p in params {
                    self.collect_vars(p, vars);
                }
            },
            _ => {},
        }
    }

    /// Instantiate type scheme
    pub fn instantiate(&self, scheme: &TypeScheme) -> Type {
        let mut ty = scheme.ty.clone();
        for &var in &scheme.vars {
            let fresh = self.fresh_var();
            ty = ty.substitute(var, &fresh);
        }
        ty
    }

    /// Get resolved type
    pub fn resolve(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }

    /// Get statistics
    pub fn stats(&self) -> &InferenceStats {
        &self.stats
    }
}

impl Default for TypeInferencer {
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
    fn test_unify_primitives() {
        let mut inf = TypeInferencer::new();

        let t1 = Type::Primitive(PrimitiveType::I32);
        let t2 = Type::Primitive(PrimitiveType::I32);

        assert!(inf.unify(&t1, &t2).is_ok());
    }

    #[test]
    fn test_unify_var() {
        let mut inf = TypeInferencer::new();

        let var = inf.fresh_var();
        let t = Type::Primitive(PrimitiveType::Bool);

        inf.unify(&var, &t).unwrap();

        let resolved = inf.resolve(&var);
        assert_eq!(resolved, t);
    }

    #[test]
    fn test_unify_function() {
        let mut inf = TypeInferencer::new();

        let var = inf.fresh_var();
        let fn1 = Type::Function {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            ret: Box::new(var.clone()),
        };
        let fn2 = Type::Function {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        };

        inf.unify(&fn1, &fn2).unwrap();

        let resolved = inf.resolve(&var);
        assert_eq!(resolved, Type::Primitive(PrimitiveType::Bool));
    }

    #[test]
    fn test_occurs_check() {
        let mut inf = TypeInferencer::new();

        let var = inf.fresh_var();
        let ty = Type::Option(Box::new(var.clone()));

        let result = inf.unify(&var, &ty);
        assert!(matches!(result, Err(InferenceError::OccursCheck(_, _))));
    }

    #[test]
    fn test_generalize_instantiate() {
        let inf = TypeInferencer::new();

        let var = inf.fresh_var();
        let scheme = inf.generalize(&Type::Function {
            params: vec![var.clone()],
            ret: Box::new(var),
        });

        let inst1 = inf.instantiate(&scheme);
        let inst2 = inf.instantiate(&scheme);

        // Different instantiations should have different variables
        assert_ne!(inst1, inst2);
    }
}
