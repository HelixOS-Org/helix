//! # LUMINA Shader
//!
//! Shader compilation and management for LUMINA graphics API.
//!
//! This crate provides:
//! - Shader compilation from source (GLSL-like syntax)
//! - SPIR-V bytecode handling
//! - Shader reflection for automatic resource binding
//! - Shader variant management for uber-shaders
//! - Runtime shader compilation (with appropriate feature)
//!
//! ## Native Shader Language
//!
//! LUMINA uses its own shader language syntax (similar to GLSL) that
//! compiles to an intermediate representation optimized for the MAGMA driver.

#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// ============================================================================
// Core Shader Types
// ============================================================================

pub mod shader;
pub mod shader_module;
pub mod shader_types;

// ============================================================================
// Compilation
// ============================================================================

pub mod shader_compiler;
pub mod spirv;

// ============================================================================
// Reflection & Variants
// ============================================================================

pub mod shader_reflection;
pub mod shader_variants;

// ============================================================================
// Version
// ============================================================================

/// LUMINA Shader version
pub const LUMINA_SHADER_VERSION: (u32, u32, u32) = (1, 0, 0);
