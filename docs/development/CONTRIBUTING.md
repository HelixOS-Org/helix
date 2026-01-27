# Contributing to Helix OS

<div align="center">

🤝 **Welcome Contributors!**

*Your guide to contributing to the Helix OS project*

</div>

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Development Workflow](#2-development-workflow)
3. [Code Standards](#3-code-standards)
4. [Pull Request Process](#4-pull-request-process)
5. [Issue Guidelines](#5-issue-guidelines)
6. [Testing Requirements](#6-testing-requirements)
7. [Documentation](#7-documentation)
8. [Community Guidelines](#8-community-guidelines)
9. [Recognition](#9-recognition)

---

## 1. Getting Started

### 1.1 Prerequisites

Before contributing, ensure you have:

- **Rust Nightly**: Required for `#![no_std]` kernel development
- **QEMU**: For testing the kernel
- **Git**: For version control
- **A text editor**: VS Code recommended with rust-analyzer

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup component add rust-src llvm-tools-preview

# Install QEMU
sudo apt install qemu-system-x86
```

### 1.2 Fork and Clone

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/helix.git
cd helix

# Add upstream remote
git remote add upstream https://github.com/helix-os/helix.git

# Verify remotes
git remote -v
```

### 1.3 Build and Test

```bash
# Build the kernel
./scripts/build.sh

# Run in QEMU
./scripts/run_qemu.sh

# Run tests
./scripts/test.sh
```

### 1.4 Project Structure Overview

```
helix/
├── core/          # Core kernel functionality
├── hal/           # Hardware Abstraction Layer
├── modules/       # Module system
├── fs/            # HelixFS filesystem
├── subsystems/    # Major subsystems (DIS, memory, etc.)
│   ├── dis/       # Differentiated Intent Scheduler
│   ├── execution/ # Execution management
│   ├── memory/    # Memory management
│   └── userspace/ # Userspace support
├── boot/          # Boot code
├── profiles/      # Build profiles
├── scripts/       # Build scripts
├── docs/          # Documentation (you are here!)
└── benchmarks/    # Performance benchmarks
```

---

## 2. Development Workflow

### 2.1 Branch Strategy

```
main
  │
  ├── develop          # Development branch
  │     │
  │     ├── feature/xxx  # Feature branches
  │     ├── bugfix/xxx   # Bug fix branches
  │     └── docs/xxx     # Documentation branches
  │
  └── release/x.y      # Release branches
```

### 2.2 Creating a Feature Branch

```bash
# Update your fork
git checkout main
git fetch upstream
git merge upstream/main

# Create feature branch
git checkout -b feature/my-awesome-feature

# Work on your changes
# ... make changes ...

# Commit with meaningful messages
git add -A
git commit -m "feat(module): add awesome functionality

- Added feature X
- Improved performance of Y
- Fixed edge case in Z"

# Push to your fork
git push origin feature/my-awesome-feature
```

### 2.3 Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code change without fix/feat |
| `perf` | Performance improvement |
| `test` | Adding tests |
| `chore` | Maintenance tasks |

**Scopes:**
| Scope | Area |
|-------|------|
| `core` | Core kernel |
| `hal` | Hardware abstraction |
| `modules` | Module system |
| `fs` | Filesystem |
| `dis` | Scheduler |
| `memory` | Memory management |
| `boot` | Boot process |
| `docs` | Documentation |

**Examples:**

```bash
# Good commit messages
git commit -m "feat(dis): implement EDF scheduling policy"
git commit -m "fix(memory): prevent double-free in buddy allocator"
git commit -m "docs(api): add HAL interrupt documentation"
git commit -m "refactor(core): simplify IPC channel creation"
git commit -m "perf(fs): optimize B-tree node splitting"

# Bad commit messages (avoid these)
git commit -m "fixed stuff"
git commit -m "WIP"
git commit -m "asdf"
```

### 2.4 Keeping Your Branch Updated

```bash
# Fetch upstream changes
git fetch upstream

# Rebase your branch on latest main
git checkout feature/my-feature
git rebase upstream/main

# If conflicts occur:
# 1. Resolve conflicts in each file
# 2. git add <resolved-files>
# 3. git rebase --continue

# Force push after rebase (only to your branch!)
git push origin feature/my-feature --force-with-lease
```

---

## 3. Code Standards

### 3.1 Rust Style Guide

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                           HELIX RUST STYLE GUIDE
// ═══════════════════════════════════════════════════════════════════════════

// ─── FILE STRUCTURE ─────────────────────────────────────────────────────────

// 1. Module documentation
//! This module provides...

// 2. Imports (grouped and ordered)
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::internal::module;
use super::parent_module;

// 3. Constants
const MAX_SIZE: usize = 1024;

// 4. Type definitions
type Result<T> = core::result::Result<T, Error>;

// 5. Traits
pub trait Scheduler { ... }

// 6. Structs and enums
pub struct Task { ... }

// 7. Implementations
impl Task { ... }

// 8. Functions
pub fn create_task() -> Task { ... }

// 9. Tests (at bottom)
#[cfg(test)]
mod tests { ... }

// ─── NAMING CONVENTIONS ─────────────────────────────────────────────────────

// Types: PascalCase
struct TaskManager;
enum TaskState { Ready, Running, Blocked }
trait Scheduler;

// Functions and variables: snake_case
fn create_task() -> Task { ... }
let task_count = 42;

// Constants: SCREAMING_SNAKE_CASE
const MAX_TASKS: usize = 256;
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(0);

// Lifetimes: short lowercase
fn process<'a>(data: &'a [u8]) -> &'a str { ... }

// ─── DOCUMENTATION ──────────────────────────────────────────────────────────

/// Brief description of the function.
///
/// Longer description explaining the purpose, behavior,
/// and any important details.
///
/// # Arguments
///
/// * `name` - Description of the parameter
/// * `config` - Description of another parameter
///
/// # Returns
///
/// Description of what is returned.
///
/// # Errors
///
/// Returns `Error::NotFound` if the task doesn't exist.
///
/// # Panics
///
/// Panics if `name` is empty.
///
/// # Examples
///
/// ```
/// let task = create_task("worker", TaskConfig::default())?;
/// assert_eq!(task.name(), "worker");
/// ```
///
/// # Safety
///
/// (For unsafe functions) Explain invariants that must be upheld.
pub fn create_task(name: &str, config: TaskConfig) -> Result<Task> {
    // Implementation
}

// ─── ERROR HANDLING ─────────────────────────────────────────────────────────

// Use Result for fallible operations
pub fn allocate_page() -> Result<Page, AllocError> {
    // ...
}

// Define specific error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    OutOfMemory,
    InvalidSize,
    Alignment,
}

impl core::fmt::Display for AllocError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::InvalidSize => write!(f, "invalid allocation size"),
            Self::Alignment => write!(f, "alignment error"),
        }
    }
}

// ─── UNSAFE CODE ────────────────────────────────────────────────────────────

// Minimize unsafe blocks
// Document safety requirements
// Provide safe wrappers

/// Reads a value from a memory-mapped I/O register.
///
/// # Safety
///
/// - `addr` must be a valid MMIO address
/// - `addr` must be properly aligned for type `T`
/// - The memory at `addr` must be valid for reading
pub unsafe fn mmio_read<T: Copy>(addr: usize) -> T {
    // SAFETY: Caller guarantees addr is valid MMIO
    core::ptr::read_volatile(addr as *const T)
}

// Safe wrapper
pub fn read_status_register() -> u32 {
    const STATUS_REG: usize = 0xFEE0_0000;
    // SAFETY: STATUS_REG is a known-valid MMIO address
    unsafe { mmio_read(STATUS_REG) }
}

// ─── FORMATTING ─────────────────────────────────────────────────────────────

// Line length: 100 characters max (prefer 80)
// Indentation: 4 spaces (no tabs)
// Braces: same line for functions

fn example_function() {
    // Correct
}

// Match arms alignment
match state {
    TaskState::Ready    => handle_ready(),
    TaskState::Running  => handle_running(),
    TaskState::Blocked  => handle_blocked(),
}

// Chain method calls
let result = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.process())
    .collect::<Vec<_>>();
```

### 3.2 Code Quality Checklist

Before submitting, verify:

- [ ] Code compiles with `cargo build --release`
- [ ] No warnings with `cargo clippy`
- [ ] Formatted with `cargo fmt`
- [ ] All tests pass with `cargo test`
- [ ] Documentation builds with `cargo doc`
- [ ] No unsafe code without safety comments
- [ ] Error handling is appropriate
- [ ] Public APIs are documented

---

## 4. Pull Request Process

### 4.1 Before Opening a PR

1. **Update your branch** with latest upstream
2. **Run all checks locally**:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ./scripts/build.sh
   ```
3. **Write/update tests** for your changes
4. **Update documentation** if needed

### 4.2 PR Template

```markdown
## Description

Brief description of what this PR does.

## Related Issues

Fixes #123
Related to #456

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring

## Changes Made

- Added X functionality
- Fixed Y bug
- Improved Z performance

## Testing

Describe how you tested your changes:

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing in QEMU
- [ ] Benchmarks run (if applicable)

## Screenshots (if applicable)

Add screenshots of QEMU output or relevant visuals.

## Checklist

- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
```

### 4.3 Review Process

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PULL REQUEST LIFECYCLE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────┐     ┌───────────┐     ┌───────────┐     ┌───────────┐       │
│  │   Open    │────▶│  Review   │────▶│  Iterate  │────▶│  Approve  │       │
│  │    PR     │     │           │     │           │     │           │       │
│  └───────────┘     └───────────┘     └───────────┘     └───────────┘       │
│       │                 │                 │                 │               │
│       │                 │                 │                 │               │
│       ▼                 ▼                 ▼                 ▼               │
│  ┌───────────┐     ┌───────────┐     ┌───────────┐     ┌───────────┐       │
│  │  CI Runs  │     │ Comments  │     │  Updates  │     │   Merge   │       │
│  │  Checks   │     │ & Request │     │  & Fixes  │     │           │       │
│  └───────────┘     └───────────┘     └───────────┘     └───────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Review Timeline:**
- Initial review: Within 3 business days
- Follow-up reviews: Within 2 business days
- Simple fixes: Can be merged by any maintainer
- Major changes: Require 2 maintainer approvals

### 4.4 Addressing Review Comments

```bash
# Make requested changes
# ... edit files ...

# Commit with reference to review
git add -A
git commit -m "fix: address review comments

- Renamed variable for clarity
- Added missing error handling
- Improved documentation"

# Push updates
git push origin feature/my-feature
```

**DO:**
- Respond to all comments
- Ask for clarification if needed
- Explain your reasoning

**DON'T:**
- Take feedback personally
- Ignore comments
- Force push without warning

---

## 5. Issue Guidelines

### 5.1 Bug Reports

Use this template for bugs:

```markdown
## Bug Description

A clear and concise description of the bug.

## Environment

- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., nightly-2024-01-15]
- QEMU version: [e.g., 8.1.0]
- Commit hash: [e.g., abc1234]

## Steps to Reproduce

1. Build the kernel with '...'
2. Run in QEMU with '...'
3. Perform action '...'
4. See error

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened.

## Logs/Output

```
Paste relevant serial output or error messages
```

## Additional Context

Any other relevant information (screenshots, related issues, etc.)
```

### 5.2 Feature Requests

```markdown
## Feature Description

A clear and concise description of the feature.

## Motivation

Why is this feature needed? What problem does it solve?

## Proposed Solution

How would you like to see this implemented?

## Alternatives Considered

Other approaches you've thought about.

## Additional Context

Any other relevant information.
```

### 5.3 Issue Labels

| Label | Description |
|-------|-------------|
| `bug` | Something isn't working |
| `enhancement` | New feature or improvement |
| `documentation` | Documentation improvements |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention needed |
| `priority: high` | Urgent issues |
| `priority: low` | Nice to have |
| `wontfix` | Will not be addressed |
| `duplicate` | Already exists |
| `question` | Needs discussion |

---

## 6. Testing Requirements

### 6.1 Test Types

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                              TEST TYPES
// ═══════════════════════════════════════════════════════════════════════════

// ─── UNIT TESTS ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        let task = Task::new("test", Intent::batch());
        assert_eq!(task.name(), "test");
        assert_eq!(task.state(), TaskState::Ready);
    }
    
    #[test]
    fn test_task_state_transition() {
        let mut task = Task::new("test", Intent::batch());
        task.set_state(TaskState::Running);
        assert_eq!(task.state(), TaskState::Running);
    }
    
    #[test]
    #[should_panic(expected = "invalid state")]
    fn test_invalid_transition() {
        let mut task = Task::new("test", Intent::batch());
        task.set_state(TaskState::Running);
        task.set_state(TaskState::Ready); // Not allowed, should panic
    }
}

// ─── INTEGRATION TESTS ──────────────────────────────────────────────────────

// In tests/integration_test.rs
#[test]
fn test_scheduler_integration() {
    let mut scheduler = Scheduler::new();
    
    // Create multiple tasks
    let t1 = scheduler.spawn("task1", Intent::interactive(1000));
    let t2 = scheduler.spawn("task2", Intent::batch());
    
    // Verify scheduling order
    assert_eq!(scheduler.next(), Some(t1)); // Interactive first
    assert_eq!(scheduler.next(), Some(t2)); // Then batch
}

// ─── PROPERTY-BASED TESTS ───────────────────────────────────────────────────

// Using proptest crate (when available)
#[test]
fn test_allocator_properties() {
    // Any allocation followed by deallocation should be valid
    // Multiple allocations should not overlap
    // etc.
}

// ─── FUZZ TESTS ─────────────────────────────────────────────────────────────

// For critical components like parsers
#[test]
fn fuzz_path_parser() {
    // Test with random inputs
}
```

### 6.2 Test Coverage Requirements

| Component | Minimum Coverage |
|-----------|-----------------|
| Core modules | 80% |
| HAL | 70% |
| Filesystem | 85% |
| Scheduler | 80% |
| IPC | 75% |
| New features | 90% |

### 6.3 Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p helix-core

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_task_creation

# Run ignored tests (slow/require hardware)
cargo test -- --ignored

# Run benchmarks
cargo bench
```

---

## 7. Documentation

### 7.1 Documentation Requirements

All public APIs must have:

1. **Brief description**: One-line summary
2. **Detailed description**: Full explanation
3. **Arguments**: Document all parameters
4. **Returns**: What the function returns
5. **Errors**: When and why it fails
6. **Examples**: Working code examples
7. **Safety**: For unsafe functions

### 7.2 Documentation Types

```
docs/
├── README.md              # Main index
├── guide/                 # User guides
│   ├── INTRODUCTION.md
│   └── GETTING_STARTED.md
├── architecture/          # Architecture docs
│   ├── OVERVIEW.md
│   ├── BOOT_PROCESS.md
│   └── MEMORY_MODEL.md
├── api/                   # API reference
│   ├── CORE.md
│   ├── HAL.md
│   ├── MODULES.md
│   ├── FILESYSTEM.md
│   └── SCHEDULER.md
├── development/           # Developer docs
│   ├── CONTRIBUTING.md    # (this file)
│   ├── CODING_STANDARDS.md
│   └── DEBUGGING.md
└── reference/             # Reference docs
    ├── CHANGELOG.md
    ├── GLOSSARY.md
    └── FAQ.md
```

### 7.3 Building Documentation

```bash
# Build Rust docs
cargo doc --no-deps --document-private-items

# Open in browser
cargo doc --open

# Check doc links
cargo doc --no-deps 2>&1 | grep -i "warning"
```

---

## 8. Community Guidelines

### 8.1 Code of Conduct

We are committed to providing a welcoming and inclusive environment.

**Our Standards:**

✅ **DO:**
- Use welcoming and inclusive language
- Be respectful of differing viewpoints
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

❌ **DON'T:**
- Use sexualized language or imagery
- Engage in trolling, insults, or personal attacks
- Harass anyone publicly or privately
- Publish others' private information
- Conduct yourself unprofessionally

### 8.2 Communication Channels

| Channel | Purpose |
|---------|---------|
| GitHub Issues | Bug reports, feature requests |
| GitHub Discussions | Questions, ideas, showcase |
| Discord | Real-time chat, help |
| Mailing List | Announcements, design discussions |

### 8.3 Getting Help

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         GETTING HELP FLOWCHART                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ I have a        │                                                        │
│  │ question...     │                                                        │
│  └────────┬────────┘                                                        │
│           │                                                                 │
│           ▼                                                                 │
│  ┌─────────────────┐     Yes    ┌─────────────────┐                        │
│  │ Is it in the    │───────────▶│ Read the docs!  │                        │
│  │ documentation?  │            └─────────────────┘                        │
│  └────────┬────────┘                                                        │
│           │ No                                                              │
│           ▼                                                                 │
│  ┌─────────────────┐     Yes    ┌─────────────────┐                        │
│  │ Has it been     │───────────▶│ Read the issue  │                        │
│  │ asked before?   │            │ and comments    │                        │
│  └────────┬────────┘            └─────────────────┘                        │
│           │ No                                                              │
│           ▼                                                                 │
│  ┌─────────────────┐     Quick  ┌─────────────────┐                        │
│  │ What type of    │───────────▶│ Ask on Discord  │                        │
│  │ question?       │            └─────────────────┘                        │
│  └────────┬────────┘                                                        │
│           │ Detailed                                                        │
│           ▼                                                                 │
│  ┌─────────────────┐                                                        │
│  │ Open a GitHub   │                                                        │
│  │ Discussion      │                                                        │
│  └─────────────────┘                                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Recognition

### 9.1 Contributor Recognition

We appreciate all contributions! Contributors are recognized in:

- **CONTRIBUTORS.md**: Listed by name/handle
- **Release notes**: Mentioned for significant contributions
- **Project website**: Hall of fame for major contributors

### 9.2 Types of Contributions

All contributions are valued:

| Contribution | Recognition |
|--------------|-------------|
| Code | Listed in CONTRIBUTORS.md |
| Documentation | Listed in CONTRIBUTORS.md |
| Bug reports | Acknowledged in issue |
| Design work | Credited in relevant docs |
| Testing | Acknowledged in release notes |
| Community help | Community champion badge |

### 9.3 Becoming a Maintainer

Active contributors may be invited to become maintainers:

**Criteria:**
- Multiple quality contributions
- Good code review participation
- Helpful in community
- Understanding of project direction

**Responsibilities:**
- Review PRs
- Triage issues
- Help new contributors
- Participate in design discussions

---

## Summary

Contributing to Helix OS is straightforward:

1. **Fork & Clone** the repository
2. **Create a branch** for your changes
3. **Make changes** following our guidelines
4. **Test thoroughly** before submitting
5. **Open a PR** with clear description
6. **Respond to review** comments
7. **Celebrate** when merged! 🎉

Thank you for contributing to Helix OS!

---

<div align="center">

🤝 *Together we build better software* 🤝

**Questions?** Open a [GitHub Discussion](https://github.com/helix-os/helix/discussions)

</div>
