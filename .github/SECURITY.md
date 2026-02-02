# =============================================================================
# Helix OS - Security Policy
# =============================================================================

## 🔒 Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| main    | :white_check_mark: |
| develop | :white_check_mark: |
| < 1.0   | :x:                |

## 🚨 Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue,
please report it responsibly.

### How to Report

1. **DO NOT** create a public GitHub issue for security vulnerabilities
2. Email security@helix-os.org (or create a private security advisory)
3. Include as much detail as possible:
   - Type of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

| Timeline | Action |
|----------|--------|
| 24 hours | Initial acknowledgment |
| 72 hours | Preliminary assessment |
| 7 days   | Detailed response with plan |
| 30 days  | Fix development (for valid issues) |
| 90 days  | Public disclosure (coordinated) |

## 🛡️ Security Measures

### CI/CD Pipeline Security

- **Hardened Runners**: All CI jobs use `step-security/harden-runner`
- **Minimal Permissions**: Jobs run with least-privilege permissions
- **Dependency Scanning**: Automated `cargo-audit` and `cargo-deny` checks
- **Secret Scanning**: TruffleHog and Gitleaks integration
- **SLSA Provenance**: Build attestation for releases

### Code Security

- **Signed Commits**: Required for main and release branches
- **Code Review**: 2 approvals required for production changes
- **CODEOWNERS**: Security team review for sensitive files
- **Supply Chain**: Dependency review on all PRs

### Branch Protection

- **main**: Strictest protection, release managers only
- **develop**: Standard protection, 1 approval
- **release/***: Enhanced protection, 2 approvals
- **hotfix/***: Expedited protection, security team

## 🔐 Security Best Practices for Contributors

### Code

```rust
// ✅ DO: Use safe abstractions
pub fn safe_operation() -> Result<(), Error> { ... }

// ❌ DON'T: Unnecessary unsafe without justification
unsafe fn risky_operation() { ... }

// ✅ DO: Document safety requirements
/// # Safety
/// - Pointer must be valid and aligned
/// - Memory must be initialized
pub unsafe fn documented_unsafe() { ... }
```

### Commits

```bash
# ✅ DO: Sign your commits
git config --global commit.gpgsign true

# ✅ DO: Use conventional commits
git commit -m "fix(security): validate input bounds"
```

### Dependencies

- Minimize external dependencies
- Prefer well-audited crates
- Pin exact versions in Cargo.lock
- Review dependency changes carefully

## 📋 Security Checklist for PRs

- [ ] No hardcoded secrets or credentials
- [ ] All `unsafe` blocks are documented and justified
- [ ] Input validation for external data
- [ ] Bounds checking for array/slice access
- [ ] No use of deprecated or insecure APIs
- [ ] Dependencies are reviewed and trusted

## 🏆 Security Hall of Fame

We appreciate responsible disclosure. Contributors who report valid
security issues will be acknowledged here (with permission).

---

*Last updated: 2026-02-02*
