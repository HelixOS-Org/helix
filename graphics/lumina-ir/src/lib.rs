//! # Lumina IR - Intermediate Representation
//!
//! This crate provides the intermediate representation used by the Lumina shader compiler.
//! The IR is designed to be a portable, optimizable representation of shader code that can
//! be compiled to various backends (SPIR-V, DXIL, Metal, etc.).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     LUMINA IR ARCHITECTURE                      │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Rust Shader Source                                            │
//! │         │                                                       │
//! │         ▼                                                       │
//! │   ┌─────────────┐                                               │
//! │   │   Parser    │ ◄── Parse Rust AST subset                     │
//! │   └──────┬──────┘                                               │
//! │          │                                                      │
//! │          ▼                                                      │
//! │   ┌─────────────┐                                               │
//! │   │  Validator  │ ◄── Check shader constraints                  │
//! │   └──────┬──────┘                                               │
//! │          │                                                      │
//! │          ▼                                                      │
//! │   ┌─────────────┐                                               │
//! │   │  IR Module  │ ◄── Type-safe intermediate representation     │
//! │   └──────┬──────┘                                               │
//! │          │                                                      │
//! │          ▼                                                      │
//! │   ┌─────────────┐                                               │
//! │   │  Optimizer  │ ◄── Dead code, constant folding, etc.         │
//! │   └──────┬──────┘                                               │
//! │          │                                                      │
//! │          ▼                                                      │
//! │   SPIR-V / DXIL / Metal                                         │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`types`] - IR type system
//! - [`instruction`] - IR instruction set
//! - [`module`] - IR module structure
//! - [`function`] - IR function representation
//! - [`block`] - Basic blocks and control flow
//! - [`value`] - Values and constants
//! - [`optimizer`] - Optimization passes
//! - [`builder`] - IR builder utilities
//! - [`analysis`] - Control flow and data flow analysis
//! - [`validation`] - IR validation

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

pub mod types;
pub mod instruction;
pub mod module;
pub mod function;
pub mod block;
pub mod value;
pub mod optimizer;
pub mod builder;
pub mod analysis;
pub mod validation;
pub mod intrinsics;
pub mod passes;
pub mod cfg;
pub mod dominator;
pub mod ssa;

pub use types::*;
pub use instruction::*;
pub use module::*;
pub use function::*;
pub use block::*;
pub use value::*;
pub use builder::*;
