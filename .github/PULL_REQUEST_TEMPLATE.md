## Description

<!-- Provide a brief description of the changes in this PR -->

## Type of Change

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ New feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to change)
- [ ] 🔒 Security fix (addresses a vulnerability or security concern)
- [ ] 📚 Documentation update
- [ ] 🔧 Refactoring (no functional changes)
- [ ] 🧪 Test addition/modification

## Security Checklist

**All items must be checked for PRs touching kernel-critical code (core/, hal/, boot/, subsystems/memory/, subsystems/execution/)**

- [ ] ❌ **No new `#[allow(...)]` attributes added** (fix root causes instead)
- [ ] ❌ **No new `static mut` declarations** (use thread-safe alternatives)
- [ ] ✅ **All new `unsafe` blocks have `// SAFETY:` comments** explaining soundness
- [ ] ✅ **No new `unwrap()` or `expect()` in kernel code** (use proper error handling)
- [ ] ✅ **All user/hardware inputs are validated** before use
- [ ] ✅ **No hardcoded secrets, keys, or credentials**
- [ ] ✅ **Bounds checking added for array/slice accesses**

## Testing

<!-- Describe the testing you've done -->

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Tested on target architecture(s): <!-- x86_64, aarch64, riscv64 -->
- [ ] QEMU testing completed

## Code Quality

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo doc` builds without warnings
- [ ] No new compiler warnings introduced

## Review Guidance

<!-- Help reviewers understand your changes -->

### Areas requiring careful review:
<!-- List specific files or areas that need extra attention -->

### Potential risks:
<!-- Describe any risks or concerns with these changes -->

## Related Issues

<!-- Link to related issues using "Fixes #123" or "Related to #456" -->

---

**By submitting this PR, I confirm that:**
- [ ] I have read and followed the [Contributing Guidelines](CONTRIBUTING.md)
- [ ] I have read and understood the [Security Policy](SECURITY.md)
- [ ] My code follows the project's coding standards
- [ ] I have not introduced any known security vulnerabilities
