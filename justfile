# =============================================================================
# Helix OS - Justfile (Modern Makefile Alternative)
# =============================================================================
# https://github.com/casey/just
# Run `just --list` to see available commands
# =============================================================================

# Default recipe - show help
default:
    @just --list --unsorted

# =============================================================================
# Build Recipes
# =============================================================================

# Build the kernel in release mode
build:
    @echo "🔨 Building Helix kernel..."
    ./scripts/build.sh

# Build in debug mode
build-debug:
    @echo "🔨 Building Helix kernel (debug)..."
    ./scripts/build.sh --debug

# Build with all features
build-full:
    @echo "🔨 Building Helix kernel (all features)..."
    cargo build --release --all-features

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean
    rm -rf build/output build/logs build/iso

# Clean and rebuild
rebuild: clean build

# Build for specific target
build-target target:
    @echo "🔨 Building for {{target}}..."
    cargo build --release --target {{target}}

# Build ISO image
iso: build
    @echo "📀 Creating bootable ISO..."
    ./scripts/build.sh --iso

# =============================================================================
# Run Recipes
# =============================================================================

# Run in QEMU
run: build
    @echo "🚀 Starting Helix in QEMU..."
    ./scripts/run_qemu.sh

# Run in debug mode (GDB wait)
run-debug: build-debug
    @echo "🐛 Starting Helix in QEMU (debug mode)..."
    ./scripts/run_qemu.sh --debug

# Run with verbose output
run-verbose: build
    @echo "🚀 Starting Helix in QEMU (verbose)..."
    ./scripts/run_qemu.sh --verbose

# Run with extra QEMU options
run-extra *args: build
    @echo "🚀 Starting Helix in QEMU with extra args..."
    ./scripts/run_qemu.sh {{args}}

# Run with KVM disabled (for WSL/containers)
run-no-kvm: build
    @echo "🚀 Starting Helix in QEMU (no KVM)..."
    ./scripts/run_qemu.sh --no-kvm

# =============================================================================
# Test Recipes
# =============================================================================

# Run all tests
test:
    @echo "🧪 Running tests..."
    ./scripts/test.sh

# Run unit tests only
test-unit:
    @echo "🧪 Running unit tests..."
    cargo test --target x86_64-unknown-linux-gnu --lib

# Run integration tests
test-integration:
    @echo "🧪 Running integration tests..."
    cargo test --target x86_64-unknown-linux-gnu --test '*'

# Run tests with output
test-verbose:
    @echo "🧪 Running tests (verbose)..."
    cargo test --target x86_64-unknown-linux-gnu -- --nocapture

# Run specific test
test-one name:
    @echo "🧪 Running test: {{name}}..."
    cargo test --target x86_64-unknown-linux-gnu {{name}} -- --nocapture

# Run benchmarks
bench:
    @echo "📊 Running benchmarks..."
    cargo bench --target x86_64-unknown-linux-gnu

# =============================================================================
# Lint and Format Recipes
# =============================================================================

# Run clippy
clippy:
    @echo "📎 Running Clippy..."
    cargo clippy --all-targets --all-features -- -D warnings

# Run clippy with fixes
clippy-fix:
    @echo "📎 Running Clippy with auto-fix..."
    cargo clippy --all-targets --all-features --fix --allow-dirty

# Format code
fmt:
    @echo "✨ Formatting code..."
    cargo fmt --all

# Check formatting
fmt-check:
    @echo "✨ Checking code format..."
    cargo fmt --all -- --check

# Run all lints
lint: fmt-check clippy
    @echo "✅ All lints passed!"

# Fix all auto-fixable issues
fix: fmt clippy-fix
    @echo "✅ Applied all auto-fixes!"

# =============================================================================
# Documentation Recipes
# =============================================================================

# Generate documentation
doc:
    @echo "📚 Generating documentation..."
    cargo doc --no-deps --document-private-items

# Generate and open documentation
doc-open:
    @echo "📚 Generating and opening documentation..."
    cargo doc --no-deps --document-private-items --open

# Check documentation
doc-check:
    @echo "📚 Checking documentation..."
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Serve documentation locally
doc-serve: doc
    @echo "📚 Serving documentation at http://localhost:8000..."
    python3 -m http.server 8000 --directory target/doc

# =============================================================================
# Development Recipes
# =============================================================================

# Watch for changes and rebuild
watch:
    @echo "👀 Watching for changes..."
    cargo watch -x 'build --release'

# Watch and run tests
watch-test:
    @echo "👀 Watching for changes (tests)..."
    cargo watch -x 'test --target x86_64-unknown-linux-gnu'

# Check without building
check:
    @echo "🔍 Checking code..."
    cargo check --all-targets --all-features

# Full pre-commit check
pre-commit: fmt-check clippy test doc-check
    @echo "✅ Pre-commit checks passed!"

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    cargo update

# Audit dependencies for vulnerabilities
audit:
    @echo "🔒 Auditing dependencies..."
    cargo audit

# Show dependency tree
deps:
    cargo tree

# Show outdated dependencies
outdated:
    cargo outdated -R

# =============================================================================
# Debug Recipes
# =============================================================================

# Start GDB session
gdb: build-debug
    @echo "🐛 Starting GDB session..."
    gdb -x .gdbinit build/output/helix-kernel

# Disassemble kernel
disasm:
    @echo "🔬 Disassembling kernel..."
    objdump -d build/output/helix-kernel | less

# Show kernel symbols
symbols:
    @echo "🔬 Showing kernel symbols..."
    nm build/output/helix-kernel | sort

# Show kernel sections
sections:
    @echo "🔬 Showing kernel sections..."
    readelf -S build/output/helix-kernel

# Show kernel size
size:
    @echo "📏 Kernel size:"
    size build/output/helix-kernel

# =============================================================================
# Module Recipes
# =============================================================================

# Add a new module
module-add name:
    @echo "➕ Adding module: {{name}}..."
    ./scripts/module_add.sh {{name}}

# List all modules
module-list:
    @echo "📋 Listing modules..."
    find modules_impl -name "Cargo.toml" -exec dirname {} \;

# =============================================================================
# CI Recipes
# =============================================================================

# Full CI pipeline
ci: lint test doc-check build
    @echo "✅ CI pipeline passed!"

# CI with coverage
ci-coverage:
    @echo "📊 Running CI with coverage..."
    cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html --output-dir build/coverage

# =============================================================================
# Release Recipes
# =============================================================================

# Create release build
release: clean
    @echo "🚀 Building release..."
    ./scripts/build.sh --release

# Create release with all artifacts
release-full: clean build iso doc
    @echo "📦 Creating full release package..."
    mkdir -p build/release
    cp build/output/helix-kernel build/release/
    cp build/output/*.iso build/release/ 2>/dev/null || true
    tar -czvf build/helix-release.tar.gz -C build/release .
    @echo "✅ Release package created: build/helix-release.tar.gz"

# =============================================================================
# Utility Recipes
# =============================================================================

# Show project stats
stats:
    @echo "📊 Project statistics:"
    @echo "  Rust files: $(find . -name '*.rs' -not -path './target/*' | wc -l)"
    @echo "  Lines of Rust: $(find . -name '*.rs' -not -path './target/*' -exec cat {} \; | wc -l)"
    @echo "  Crates: $(find . -name 'Cargo.toml' -not -path './target/*' | wc -l)"

# Show TODO/FIXME comments
todos:
    @echo "📝 TODO/FIXME comments:"
    @grep -rn "TODO\|FIXME\|XXX\|HACK" --include="*.rs" . || echo "None found!"

# Generate flamegraph (requires cargo-flamegraph)
flamegraph:
    @echo "🔥 Generating flamegraph..."
    cargo flamegraph --target x86_64-unknown-linux-gnu

# Install development tools
setup-tools:
    @echo "🔧 Installing development tools..."
    rustup component add rustfmt clippy llvm-tools-preview
    cargo install cargo-watch cargo-audit cargo-outdated cargo-tarpaulin cargo-expand

# Verify toolchain setup
verify:
    @echo "🔧 Verifying toolchain..."
    rustc --version
    cargo --version
    rustfmt --version
    cargo clippy --version
    @echo "✅ Toolchain verified!"

# =============================================================================
# Help
# =============================================================================

# Show detailed help
help:
    @echo "Helix OS Build System"
    @echo "====================="
    @echo ""
    @echo "Usage: just <recipe>"
    @echo ""
    @echo "Run 'just --list' for available recipes."
    @echo "Run 'just --show <recipe>' for recipe details."
