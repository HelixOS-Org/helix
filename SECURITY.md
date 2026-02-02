# Security Policy

## About Helix OS Security

Helix OS is an experimental operating system kernel written in Rust. Security is a core priority,
and we take all security vulnerabilities seriously. This document outlines our security policies
and procedures for reporting vulnerabilities.

## Supported Versions

As Helix OS is currently in early development (pre-1.0), only the latest version on the `main`
branch receives security updates.

| Version | Supported          |
| ------- | ------------------ |
| main    | :white_check_mark: |
| develop | :white_check_mark: |
| < 0.1   | :x:                |

## Security Features

Helix OS implements several security mechanisms:

### Memory Safety
- **Rust Language**: The kernel is written in Rust, providing memory safety guarantees
- **No unsafe code in userspace**: Strict separation between kernel and user space
- **Stack protection**: Guard pages and stack canaries where applicable

### Kernel Hardening
- **KASLR**: Kernel Address Space Layout Randomization
- **W^X Policy**: Memory regions are either writable or executable, never both
- **Secure boot support**: UEFI Secure Boot compatibility

### Runtime Protection
- **Nexus AI Subsystem**: Anomaly detection and self-healing capabilities
- **Quarantine System**: Automatic isolation of misbehaving components
- **Intrusion Detection**: Behavioral analysis and threat detection

## Reporting a Vulnerability

### Where to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

1. **Email**: Send details to the maintainers (check repository for contact info)
2. **GitHub Security Advisories**: Use GitHub's private vulnerability reporting feature

### What to Include

When reporting a vulnerability, please include:

- **Description**: A clear description of the vulnerability
- **Impact**: What an attacker could achieve by exploiting this
- **Reproduction Steps**: Step-by-step instructions to reproduce the issue
- **Affected Components**: Which parts of Helix OS are affected
- **Suggested Fix**: If you have ideas on how to fix it (optional)

### Response Timeline

- **Acknowledgment**: Within 48 hours of receiving your report
- **Initial Assessment**: Within 7 days
- **Status Updates**: Every 14 days until resolution
- **Resolution**: Depends on severity and complexity

### Severity Classification

| Severity | Description | Target Resolution |
|----------|-------------|-------------------|
| Critical | Remote code execution, kernel panic | 24-72 hours |
| High | Privilege escalation, memory corruption | 7 days |
| Medium | Information disclosure, DoS | 30 days |
| Low | Minor issues, hardening improvements | 90 days |

## Security Best Practices for Contributors

When contributing to Helix OS, please follow these guidelines:

1. **Minimize `unsafe` blocks**: Only use when absolutely necessary, with clear safety comments
2. **Validate all inputs**: Especially from userspace or external sources
3. **Use checked arithmetic**: Prefer `checked_*` or `saturating_*` operations
4. **No hardcoded secrets**: Never commit credentials, keys, or sensitive data
5. **Review dependencies**: Use `cargo audit` before adding new dependencies
6. **Test edge cases**: Include tests for boundary conditions and malformed inputs

## Acknowledgments

We appreciate the security research community's efforts in responsibly disclosing vulnerabilities.
Contributors who report valid security issues will be acknowledged in our release notes
(unless they prefer to remain anonymous).

## Contact

For security-related inquiries that don't involve vulnerability reports,
please open a discussion on the GitHub repository.

---

*This security policy is subject to change. Last updated: February 2026*
