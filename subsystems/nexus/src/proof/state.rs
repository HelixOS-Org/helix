//! State representation for model checking
//!
//! This module provides State and Value types for representing
//! system states during model checking verification.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Hash a string for state deduplication
#[inline]
pub fn hash_str(s: &str) -> u64 {
    let mut h = 0u64;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u64);
    }
    h
}

/// A value in state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// String
    String(String),
    /// Array
    Array(Vec<Value>),
    /// Null/undefined
    Null,
}

impl Value {
    /// Get as bool
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as int
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as uint
    #[inline]
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Self::Uint(u) => Some(*u),
            _ => None,
        }
    }

    /// Get as string
    #[inline]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Hash for deduplication
    pub fn hash(&self) -> u64 {
        match self {
            Self::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            },
            Self::Int(i) => *i as u64,
            Self::Uint(u) => *u,
            Self::String(s) => hash_str(s),
            Self::Array(arr) => arr
                .iter()
                .fold(0, |acc, v| acc.wrapping_mul(31).wrapping_add(v.hash())),
            Self::Null => 0,
        }
    }
}

/// A system state for model checking
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct State {
    /// State ID
    pub id: u64,
    /// Variable values
    pub variables: BTreeMap<String, Value>,
    /// Parent state (for trace)
    pub parent: Option<u64>,
    /// Transition label
    pub transition: Option<String>,
}

impl State {
    /// Create a new state
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            variables: BTreeMap::new(),
            parent: None,
            transition: None,
        }
    }

    /// Set a variable
    #[inline(always)]
    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.variables.insert(name.into(), value);
    }

    /// Get a variable
    #[inline(always)]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Set parent
    #[inline(always)]
    pub fn with_parent(mut self, parent: u64) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set transition
    #[inline(always)]
    pub fn with_transition(mut self, trans: impl Into<String>) -> Self {
        self.transition = Some(trans.into());
        self
    }

    /// Get hash for deduplication
    #[inline]
    pub fn hash(&self) -> u64 {
        let mut h = 0u64;
        for (k, v) in &self.variables {
            h = h.wrapping_mul(31).wrapping_add(hash_str(k));
            h = h.wrapping_mul(31).wrapping_add(v.hash());
        }
        h
    }
}

/// A counterexample showing property violation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct Counterexample {
    /// Trace of states leading to violation
    pub trace: Vec<State>,
    /// Description
    pub description: String,
    /// Violating state
    pub violating_state: usize,
}

impl Counterexample {
    /// Create a new counterexample
    pub fn new(trace: Vec<State>, violating_state: usize) -> Self {
        Self {
            trace,
            description: String::new(),
            violating_state,
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Get length of trace
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.trace.len()
    }

    /// Check if trace is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.trace.is_empty()
    }
}
