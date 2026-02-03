# CodeQL Security Alert Suppressions

This document explains the security alert suppressions in the Helix OS codebase.

## Overview

Helix OS is a bare-metal operating system kernel that interfaces directly with hardware
and firmware. This requires certain patterns that static analysis tools may flag as
security issues, even when they are intentional and safe.

## Suppressed Alerts

### 1. Hard-coded Cryptographic Values (CWE-798)

**Location**: `fs/src/crypto/*.rs` (test modules)

**Reason**: Test code intentionally uses zero-filled keys and nonces for:
- Deterministic testing behavior
- Reproducible test results
- Unit test verification

**Why it's safe**:
- These values are only used in `#[cfg(test)]` modules
- Production code uses keys derived from secure KDF (Key Derivation Functions)
- Test values never appear in release builds

**Production key handling**:
```rust
// Production keys come from:
// 1. User-provided passphrase via secure input
// 2. Argon2id KDF with high memory/iteration cost
// 3. Hardware RNG for salt generation
```

### 2. Access of Invalid Pointer (CWE-825)

**Location**: `boot/uefi/src/protocols/*.rs`, `boot/uefi/src/raw/protocols/*.rs`

**Reason**: UEFI FFI code requires raw pointer manipulation to interface with firmware.

**Why it's safe**:
1. **UEFI Specification Guarantees**: Protocol pointers obtained via `LocateProtocol`
   or `HandleProtocol` are guaranteed valid by the UEFI specification
2. **Firmware Validation**: The UEFI firmware validates all protocol handles
3. **Lifetime Guarantees**: Pointers remain valid for the duration of boot services
4. **Debug Assertions**: We add `debug_assert!(!ptr.is_null())` for development builds

**Safety Documentation**:
All `unsafe fn from_raw()` functions document their safety requirements:
```rust
/// # Safety
/// Protocol pointer must be valid and obtained from UEFI LocateProtocol
/// or HandleProtocol calls. The pointer must remain valid for the
/// lifetime of this instance.
```

## How to Handle New Alerts

### Legitimate Security Issues
1. Fix the issue immediately
2. Add a regression test
3. Document the fix in the PR

### False Positives
1. Add a `// codeql[rule-id]` suppression comment
2. Document the suppression in this file
3. Explain why the code is safe in the source file

## CodeQL Configuration

The CodeQL configuration file (`.github/codeql/codeql-config.yml`) contains:
- Path exclusions for test code
- Query filters for kernel-specific patterns
- Rule exclusions for documented false positives

## References

- [UEFI Specification](https://uefi.org/specifications)
- [CodeQL for Rust](https://codeql.github.com/docs/codeql-language-guides/codeql-for-rust/)
- [CWE-798: Hard-coded Credentials](https://cwe.mitre.org/data/definitions/798.html)
- [CWE-825: Expired Pointer Dereference](https://cwe.mitre.org/data/definitions/825.html)
