# =============================================================================
# Helix OS - Docker Development Environment
# =============================================================================
# Containerized build environment for consistent builds
# =============================================================================

FROM rust:1.75-bookworm AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    # Build tools
    build-essential \
    binutils \
    lld \
    nasm \
    # QEMU for testing
    qemu-system-x86 \
    qemu-system-arm \
    # Bootloader tools
    grub-pc-bin \
    grub-common \
    xorriso \
    mtools \
    # Debugging
    gdb-multiarch \
    # Utilities
    curl \
    git \
    xxd \
    file \
    # Cleanup
    && rm -rf /var/lib/apt/lists/*

# Install Rust nightly and targets
RUN rustup default nightly \
    && rustup component add rust-src rustfmt clippy llvm-tools-preview \
    && rustup target add x86_64-unknown-none \
    && rustup target add aarch64-unknown-none \
    && rustup target add riscv64gc-unknown-none-elf

# Install cargo tools
RUN cargo install cargo-watch cargo-audit cargo-expand

# Create working directory
WORKDIR /helix

# Copy project files
COPY . .

# Set environment variables
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:${PATH}"
ENV RUST_BACKTRACE=1

# Default command
CMD ["./scripts/build.sh"]

# =============================================================================
# Multi-stage build for smaller runtime image
# =============================================================================

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    grub-pc-bin \
    xorriso \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /helix

# Copy built artifacts from builder
COPY --from=builder /helix/build/output /helix/build/output
COPY --from=builder /helix/scripts /helix/scripts

CMD ["./scripts/run_qemu.sh", "--no-kvm"]
