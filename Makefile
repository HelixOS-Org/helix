# =============================================================================
# Helix OS Framework - Makefile
# =============================================================================
# Modern build system with beautiful output
# =============================================================================

.PHONY: all build clean test run help \
        step-0 step-1 step-2 step-3 step-4 step-5 step-6 \
        step-7 step-8 step-9 step-10 step-11 step-12 \
        qemu debug flash modules config docs

# =============================================================================
# Configuration
# =============================================================================

# Directories
ROOT_DIR := $(shell pwd)
SCRIPTS_DIR := $(ROOT_DIR)/scripts
BUILD_DIR := $(ROOT_DIR)/build
OUTPUT_DIR := $(BUILD_DIR)/output

# Default target
ARCH ?= x86_64
PROFILE ?= release
TARGET := $(ARCH)-unknown-none

# Colors (for direct make rules)
RESET := \033[0m
BOLD := \033[1m
GREEN := \033[32m
YELLOW := \033[33m
BLUE := \033[34m
CYAN := \033[36m

# =============================================================================
# Main Targets
# =============================================================================

all: build ## Build everything
	@echo ""
	@echo -e "$(GREEN)$(BOLD)✓ Build complete!$(RESET)"

build: ## Full build using the LFS-style build system
	@$(SCRIPTS_DIR)/build.sh all

quick: ## Quick build (skip environment checks)
	@$(SCRIPTS_DIR)/build.sh from 1_build_bootloader

clean: ## Clean all build artifacts
	@$(SCRIPTS_DIR)/build.sh clean
	@echo -e "$(GREEN)✓ Cleaned$(RESET)"

distclean: clean ## Deep clean including Cargo.lock
	@rm -rf target Cargo.lock
	@echo -e "$(GREEN)✓ Deep cleaned$(RESET)"

# =============================================================================
# Individual Build Steps
# =============================================================================

step-0: ## Prepare build environment
	@$(SCRIPTS_DIR)/build.sh step 0_prepare_env

step-1: ## Build bootloader
	@$(SCRIPTS_DIR)/build.sh step 1_build_bootloader

step-2: ## Build core kernel
	@$(SCRIPTS_DIR)/build.sh step 2_build_core_kernel

step-3: ## Build memory subsystem
	@$(SCRIPTS_DIR)/build.sh step 3_build_memory_subsystem

step-4: ## Build scheduler
	@$(SCRIPTS_DIR)/build.sh step 4_build_scheduler

step-5: ## Build I/O subsystem
	@$(SCRIPTS_DIR)/build.sh step 5_build_io_subsystem

step-6: ## Build module system
	@$(SCRIPTS_DIR)/build.sh step 6_build_module_system

step-7: ## Build communication layer
	@$(SCRIPTS_DIR)/build.sh step 7_build_communication_layer

step-8: ## Build system interface
	@$(SCRIPTS_DIR)/build.sh step 8_build_sys_interface

step-9: ## Build userland framework
	@$(SCRIPTS_DIR)/build.sh step 9_build_userland_framework

step-10: ## Run all tests
	@$(SCRIPTS_DIR)/build.sh step 10_test_all

step-11: ## Package kernel image
	@$(SCRIPTS_DIR)/build.sh step 11_package_kernel

step-12: clean ## Alias for clean

# =============================================================================
# Run Targets
# =============================================================================

run: qemu ## Run in QEMU (alias)

qemu: ## Run in QEMU
	@$(SCRIPTS_DIR)/run_qemu.sh

qemu-debug: ## Run in QEMU with GDB debugging
	@$(SCRIPTS_DIR)/run_qemu.sh --debug

qemu-graphics: ## Run in QEMU with graphics
	@$(SCRIPTS_DIR)/run_qemu.sh --graphics

qemu-vnc: ## Run in QEMU with VNC display
	@$(SCRIPTS_DIR)/run_qemu.sh --vnc

# =============================================================================
# Test Targets
# =============================================================================

test: ## Run all tests
	@echo -e "$(CYAN)Running tests...$(RESET)"
	@cargo +nightly test --workspace --lib
	@echo -e "$(GREEN)✓ Tests passed$(RESET)"

test-unit: ## Run unit tests only
	@cargo +nightly test --workspace --lib -- --skip integration

test-integration: ## Run integration tests
	@cargo +nightly test --workspace --test '*'

test-doc: ## Test documentation examples
	@cargo +nightly test --workspace --doc

# =============================================================================
# Module Management
# =============================================================================

modules: ## List all modules
	@$(SCRIPTS_DIR)/module_add.sh list

module-add: ## Add a new module (interactive)
	@echo "Usage: make module-add-TYPE-NAME"
	@echo "  Example: make module-add-scheduler-my_sched"
	@echo ""
	@$(SCRIPTS_DIR)/module_add.sh list

module-add-scheduler-%: ## Add a new scheduler module
	@$(SCRIPTS_DIR)/module_add.sh add scheduler $*

module-add-allocator-%: ## Add a new allocator module
	@$(SCRIPTS_DIR)/module_add.sh add allocator $*

module-add-driver-%: ## Add a new driver module
	@$(SCRIPTS_DIR)/module_add.sh add driver $*

# =============================================================================
# Configuration
# =============================================================================

config: ## Interactive configuration generator
	@$(SCRIPTS_DIR)/config_generator.sh interactive

config-kernel: ## Generate kernel configuration
	@$(SCRIPTS_DIR)/config_generator.sh kernel

config-profile: ## Generate OS profile
	@$(SCRIPTS_DIR)/config_generator.sh profile

# =============================================================================
# Flash Targets
# =============================================================================

flash: ## Flash utility help
	@$(SCRIPTS_DIR)/flash.sh help

image: ## Create disk image
	@$(SCRIPTS_DIR)/flash.sh image $(OUTPUT_DIR)/helix.img 256M

usb: ## Create bootable USB (requires device path)
	@echo "Usage: make usb DEVICE=/dev/sdX"
	@echo ""
	@echo "Available devices:"
	@lsblk -d -o NAME,SIZE,MODEL | grep -v loop

usb-%: ## Create bootable USB on specific device
	@$(SCRIPTS_DIR)/flash.sh usb /dev/$*

# =============================================================================
# Documentation
# =============================================================================

docs: ## Generate documentation
	@echo -e "$(CYAN)Generating documentation...$(RESET)"
	@cargo +nightly doc --workspace --no-deps
	@echo -e "$(GREEN)✓ Documentation generated at target/doc/$(RESET)"

docs-open: docs ## Generate and open documentation
	@xdg-open target/doc/helix_core/index.html 2>/dev/null || open target/doc/helix_core/index.html

# =============================================================================
# Development Tools
# =============================================================================

fmt: ## Format all code
	@echo -e "$(CYAN)Formatting code...$(RESET)"
	@cargo +nightly fmt --all
	@echo -e "$(GREEN)✓ Formatted$(RESET)"

fmt-check: ## Check code formatting
	@cargo +nightly fmt --all -- --check

clippy: ## Run clippy lints
	@echo -e "$(CYAN)Running clippy...$(RESET)"
	@cargo +nightly clippy --workspace --all-targets
	@echo -e "$(GREEN)✓ No warnings$(RESET)"

check: ## Quick syntax check
	@cargo +nightly check --workspace

# =============================================================================
# Architecture-specific builds
# =============================================================================

x86_64: ## Build for x86_64
	@HELIX_ARCH=x86_64 $(SCRIPTS_DIR)/build.sh all

aarch64: ## Build for AArch64
	@HELIX_ARCH=aarch64 $(SCRIPTS_DIR)/build.sh all

riscv64: ## Build for RISC-V 64
	@HELIX_ARCH=riscv64 $(SCRIPTS_DIR)/build.sh all

# =============================================================================
# Profile-specific builds
# =============================================================================

debug: ## Build in debug mode
	@HELIX_PROFILE=debug $(SCRIPTS_DIR)/build.sh all

release: ## Build in release mode (default)
	@HELIX_PROFILE=release $(SCRIPTS_DIR)/build.sh all

# =============================================================================
# Utility Targets
# =============================================================================

info: ## Show build configuration
	@echo ""
	@echo -e "$(BOLD)Helix OS Framework$(RESET)"
	@echo "─────────────────────────────────"
	@echo -e "  Architecture: $(CYAN)$(ARCH)$(RESET)"
	@echo -e "  Target:       $(CYAN)$(TARGET)$(RESET)"
	@echo -e "  Profile:      $(CYAN)$(PROFILE)$(RESET)"
	@echo -e "  Build Dir:    $(CYAN)$(BUILD_DIR)$(RESET)"
	@echo ""
	@echo -e "$(BOLD)Rust Toolchain$(RESET)"
	@echo "─────────────────────────────────"
	@rustc --version
	@cargo --version
	@echo ""

tree: ## Show project structure
	@command -v tree >/dev/null 2>&1 && tree -L 3 -I 'target|.git' || find . -maxdepth 3 -type d ! -path './target/*' ! -path './.git/*'

loc: ## Count lines of code
	@echo -e "$(BOLD)Lines of Code$(RESET)"
	@echo "─────────────────────────────────"
	@find . -name "*.rs" -not -path "./target/*" | xargs wc -l | tail -1 | awk '{print "  Rust: " $$1}'
	@find . -name "*.sh" -not -path "./target/*" | xargs wc -l | tail -1 | awk '{print "  Bash: " $$1}'
	@find . -name "*.toml" -not -path "./target/*" | xargs wc -l | tail -1 | awk '{print "  TOML: " $$1}'
	@echo ""

update: ## Update dependencies
	@echo -e "$(CYAN)Updating dependencies...$(RESET)"
	@cargo +nightly update
	@echo -e "$(GREEN)✓ Dependencies updated$(RESET)"

# =============================================================================
# Help
# =============================================================================

help: ## Show this help
	@echo ""
	@echo -e "$(BOLD)$(CYAN)    ██╗  ██╗███████╗██╗     ██╗██╗  ██╗$(RESET)"
	@echo -e "$(BOLD)$(CYAN)    ██║  ██║██╔════╝██║     ██║╚██╗██╔╝$(RESET)"
	@echo -e "$(BOLD)$(CYAN)    ███████║█████╗  ██║     ██║ ╚███╔╝ $(RESET)"
	@echo -e "$(BOLD)$(CYAN)    ██╔══██║██╔══╝  ██║     ██║ ██╔██╗ $(RESET)"
	@echo -e "$(BOLD)$(CYAN)    ██║  ██║███████╗███████╗██║██╔╝ ██╗$(RESET)"
	@echo -e "$(BOLD)$(CYAN)    ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═╝$(RESET)"
	@echo ""
	@echo -e "$(BOLD)    OS Framework Build System$(RESET)"
	@echo ""
	@echo -e "$(BOLD)Available targets:$(RESET)"
	@echo ""
	@grep -E '^[a-zA-Z0-9_%-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}' | \
		sort
	@echo ""
	@echo -e "$(BOLD)Examples:$(RESET)"
	@echo "  make all                    # Full build"
	@echo "  make step-2                 # Build only kernel core"
	@echo "  make qemu                   # Run in QEMU"
	@echo "  make module-add-scheduler-cfs  # Add CFS scheduler"
	@echo ""

# Default target when none specified
.DEFAULT_GOAL := help
