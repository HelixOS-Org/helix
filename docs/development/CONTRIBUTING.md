# Contributing to Helix OS

We're building a kernel. Quality matters more than velocity.

Every patch that lands in this tree runs on bare metal with no safety net —
no standard library, no OS underneath, no second chance. We hold contributions
to a high standard because the code demands it. That said, the codebase is
modular: you can work on a single crate (a scheduler, a driver, a filesystem
module) without understanding the full kernel. We welcome contributions of all
sizes, from typo fixes to new subsystems.

Read this document before opening your first PR.

---

## Table of Contents

1. [Development Environment](#1-development-environment)
2. [Code Standards (Non-Negotiable)](#2-code-standards-non-negotiable)
3. [Contribution Workflow](#3-contribution-workflow)
4. [Commit Conventions](#4-commit-conventions)
5. [Testing & CI](#5-testing--ci)
6. [Reporting Bugs](#6-reporting-bugs)
7. [Where to Start](#7-where-to-start)

---

## 1. Development Environment

### 1.1 Toolchain

The project pins a **specific Rust nightly** in `rust-toolchain.toml`
(`nightly-2025-01-15`). Rustup will install it automatically when you enter
the repository. Do **not** override it with a different nightly — the kernel
uses unstable features that may break across nightly versions.

Required components (also pinned in the toolchain file):

| Component | Purpose |
|:----------|:--------|
| `rust-src` | Required for `-Zbuild-std` (we rebuild `core`/`alloc` for the bare-metal target) |
| `rustfmt` | Code formatting — enforced in CI |
| `clippy` | Linting — enforced in CI |
| `llvm-tools-preview` | `objcopy`, `objdump`, binary inspection |

### 1.2 System Dependencies

```bash
# Debian / Ubuntu
sudo apt install qemu-system-x86 lld make git

# Fedora
sudo dnf install qemu-system-x86 lld make git

# Arch
sudo pacman -S qemu-full lld make git
```

**QEMU** is the primary testing platform. We target `x86_64-unknown-none`.

### 1.3 Setup

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/helix.git
cd helix
git remote add upstream https://github.com/helix-os/helix.git

# Verify the toolchain installs automatically
rustc --version   # should show nightly-2025-01-15

# Build
./scripts/build.sh

# Boot in QEMU
./scripts/run_qemu.sh
```

If `./scripts/build.sh` completes and QEMU boots, your environment is ready.

---

## 2. Code Standards (Non-Negotiable)

These checks run in CI. A PR that fails any of them will not be reviewed.

### 2.1 Formatting — `rustfmt`

The project ships a `rustfmt.toml` with specific rules (100-column width,
module-level import granularity, Unix line endings). Before committing:

```bash
cargo fmt --all -- --check
```

If it reports diffs, run `cargo fmt --all` and amend your commit.

### 2.2 Linting — `clippy`

The workspace configures Clippy at the `[workspace.lints.clippy]` level in
`Cargo.toml`. The rules are strict:

- **`correctness`** — denied (build fails)
- **`suspicious`**, **`complexity`**, **`perf`**, **`style`** — warned
- **`unsafe_op_in_unsafe_fn`** — warned (every `unsafe` block inside an
  `unsafe fn` must have its own `unsafe {}`)
- **`missing_safety_doc`** — warned (every `unsafe fn` must document its
  safety contract)
- **`unwrap_used`**, **`todo`**, **`unimplemented`** — warned

Run before pushing:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Yes, `-D warnings` promotes all warnings to errors. If Clippy complains,
fix it. If you believe a lint is a false positive, add an `#[allow(...)]`
with a comment explaining why — not a blanket suppression.

### 2.3 Documentation

Every public item (`pub fn`, `pub struct`, `pub trait`, `pub enum`) must
have a `///` doc comment. The comment must explain:

- **What** the item does (one sentence).
- **Safety** (if `unsafe`) — what invariants the caller must uphold.
- **Panics** (if applicable) — under what conditions.

```rust
/// Allocate a contiguous range of physical frames.
///
/// Returns `Err(MemError::OutOfMemory)` if the allocator cannot
/// satisfy the request.
///
/// # Safety
///
/// The caller must ensure `count > 0` and that the returned frames
/// are not aliased.
pub unsafe fn alloc_frames(count: usize) -> MemResult<Frame> { ... }
```

### 2.4 `#![no_std]` — No Standard Library

Every crate in this workspace compiles with `#![no_std]`. This means:

- **No `std::` imports.** Use `core::` and `alloc::` only.
- **No hidden allocations.** `format!()`, `vec![]`, `String::from()` all
  allocate. Be deliberate about every allocation. Prefer fixed-size types
  (`heapless`, `arrayvec`) or stack buffers.
- **No `println!()`.** Use the kernel's debug console (`core/src/debug/`).
- **No `unwrap()` in production paths.** Handle errors explicitly. `unwrap()`
  in kernel code is a panic on bare metal — there is no one to catch it.

### 2.5 `unsafe` Code

`unsafe` is allowed — this is a kernel. But it is never casual:

- Minimize the scope of each `unsafe` block.
- Annotate every `unsafe fn` with a `# Safety` doc section.
- Prefer safe abstractions. If you write a raw pointer dance, wrap it in a
  safe API with documented invariants.
- Every `unsafe` block should have a `// SAFETY: ...` comment on the line
  above it explaining why the operation is sound.

```rust
// SAFETY: `ptr` is guaranteed non-null and aligned by the frame allocator.
// The frame was allocated in `alloc_frames` and has not been freed.
unsafe { core::ptr::write(ptr, value) };
```

---

## 3. Contribution Workflow

### 3.1 Branching

```bash
git checkout main
git pull upstream main
git checkout -b feat/my-feature     # or fix/issue-42, refactor/hal-cleanup
```

Branch naming: `feat/`, `fix/`, `refactor/`, `docs/`, `test/`, `bench/`.

### 3.2 One PR, One Concern

Each PR should address **one** logical change. A scheduler refactor and a
typo fix are two separate PRs. This makes review faster and reverts cleaner.

### 3.3 Keep Up to Date

Rebase on `main` before requesting review. We do not accept merge commits.

```bash
git fetch upstream
git rebase upstream/main
# resolve conflicts if any
git push --force-with-lease origin feat/my-feature
```

### 3.4 Code Review

- Every PR requires at least one approving review.
- Respond to feedback within a reasonable time.
- Mark review comments as resolved when addressed.
- Do not force-push after a review has started unless asked to rebase.

---

## 4. Commit Conventions

### 4.1 Conventional Commits

All commits must follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification:

```
<type>(<scope>): <short description>

[optional body]

[optional footer(s)]
```

**Types:**

| Type | Usage |
|:-----|:------|
| `feat` | New feature or module |
| `fix` | Bug fix |
| `refactor` | Code restructuring (no behavior change) |
| `docs` | Documentation only |
| `test` | Tests only |
| `perf` | Performance improvement |
| `ci` | CI/CD pipeline changes |
| `chore` | Build scripts, tooling, dependencies |

**Scope** is the crate or subsystem affected: `hal`, `core`, `execution`,
`memory`, `fs`, `nexus`, `modules`, `boot/limine`, `boot/uefi`, etc.

**Examples:**

```
feat(execution): implement CFS scheduler module
fix(memory): prevent double-free in slab allocator
refactor(hal): extract APIC logic into dedicated submodule
docs(modules): add hot-reload protocol specification
test(fs): add CoW snapshot round-trip tests
```

### 4.2 Developer Certificate of Origin (DCO)

By submitting a patch, you certify that you have the right to do so under
the project's license. We enforce this via the
[DCO](https://developercertificate.org/).

**Sign every commit** with the `-s` flag:

```bash
git commit -s -m "feat(hal): add x2APIC MSR-based IPI support"
```

This adds a `Signed-off-by: Your Name <your@email.com>` trailer. CI will
reject unsigned commits.

If you forgot to sign, amend:

```bash
# Last commit
git commit --amend -s --no-edit

# Multiple commits
git rebase --signoff HEAD~N
```

### 4.3 Commit Hygiene

- One logical change per commit. If a commit message needs "and", split it.
- Write the subject in imperative mood: "add", not "added" or "adds".
- Keep the subject line under 72 characters.
- Use the body to explain **why**, not **what** (the diff shows what).

---

## 5. Testing & CI

### 5.1 Local Checks (Run Before Every Push)

```bash
# The full pre-commit gate — mirrors what CI runs
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --target x86_64-unknown-linux-gnu --lib
./scripts/build.sh
```

If all four pass locally, CI will pass.

### 5.2 What CI Checks

| Check | Command | Failure = blocked |
|:------|:--------|:------------------|
| Format | `cargo fmt --all -- --check` | ✅ |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ |
| Unit tests | `cargo test --target x86_64-unknown-linux-gnu --lib` | ✅ |
| Kernel build | `./scripts/build.sh` | ✅ |
| DCO | Signed-off-by trailer present | ✅ |

### 5.3 Writing Tests

- Unit tests go in the same file as the code, inside a `#[cfg(test)]` module.
- Tests run on the **host** (`x86_64-unknown-linux-gnu`), not on bare metal.
  This means `std` is available in test modules — but keep test logic
  portable.
- If your change touches a subsystem framework (e.g., `execution/`,
  `memory/`), add or update tests for the trait contracts.
- Integration tests and boot tests use `./scripts/test.sh`.

---

## 6. Reporting Bugs

A kernel bug report must be actionable. Include:

1. **Summary** — One sentence describing the observed behavior.
2. **Expected behavior** — What should have happened.
3. **Reproduction steps** — Exact commands, starting from `git clone`.
4. **Environment:**
   - Output of `rustc --version`
   - Output of `qemu-system-x86_64 --version`
   - Host OS and architecture
   - Boot protocol used (Limine / UEFI / Multiboot2)
5. **Logs** — Serial output, QEMU console output, or panic messages.
   Attach the full log, not a screenshot.
6. **Bisect** (if possible) — `git bisect` to the first bad commit.

**Template:**

```markdown
## Bug: [short description]

**Environment:**
- rustc: nightly-2025-01-15
- QEMU: 8.2.0
- Host: Ubuntu 24.04 x86_64
- Boot: Limine

**Steps to reproduce:**
1. `git checkout main`
2. `./scripts/build.sh`
3. `./scripts/run_qemu.sh`
4. Observe: [what happens]

**Expected:** [what should happen]

**Serial log:**
[paste full output]
```

---

## 7. Where to Start

The codebase is a Cargo workspace with ~20 crates. You don't need to
understand all of them. Pick an area that interests you:

| Area | Crate(s) | What to do |
|:-----|:---------|:-----------|
| **New scheduler** | `modules_impl/schedulers/` | Implement the `Scheduler` trait. Use `round_robin/` as reference. |
| **Drivers** | (new crate) | VirtIO block, VirtIO net, PS/2 keyboard. The HAL provides interrupt and MMIO abstractions. |
| **Filesystem** | `fs/` | Implement a ramfs, extend VFS coverage, add tests for CoW snapshots. |
| **HAL** | `hal/` | Improve x86_64 timer calibration, add HPET support, extend ACPI parsing. |
| **Benchmarks** | `benchmarks/` | Add benchmarks for IPC latency, context switch time, allocation throughput. |
| **Documentation** | `docs/` | Improve module guide, add architecture diagrams, write tutorials. |
| **Tests** | Any crate | Increase test coverage. Every subsystem needs more unit tests. |

Start small. A clean, well-tested 50-line patch is worth more than a
sprawling 500-line patch with no tests.

---

## License

By contributing to Helix OS, you agree that your contributions will be
licensed under the project's dual license: **MIT OR Apache-2.0**.

See [LICENSE](../../LICENSE) for details.
