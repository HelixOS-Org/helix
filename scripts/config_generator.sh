#!/bin/bash
# =============================================================================
# Helix OS Framework - Configuration Generator
# =============================================================================
# Generate custom kernel and OS profile configurations
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"
CONFIGS_DIR="${HELIX_ROOT}/configs"
PROFILES_DIR="${HELIX_ROOT}/profiles"

# =============================================================================
# Configuration Templates
# =============================================================================

generate_kernel_config() {
    local name="$1"
    local arch="${2:-x86_64}"
    local config_file="${CONFIGS_DIR}/${name}.toml"
    
    mkdir -p "${CONFIGS_DIR}"
    
    cat > "${config_file}" << EOF
# =============================================================================
# Helix Kernel Configuration: ${name}
# =============================================================================
# Generated on: $(date)
# =============================================================================

[kernel]
name = "${name}"
version = "1.0.0"
arch = "${arch}"

# Build options
[build]
profile = "release"
lto = true
opt_level = 3
debug_info = false
strip = true

# Memory configuration
[memory]
# Minimum required RAM (MB)
min_ram = 64
# Maximum supported RAM (MB, 0 = unlimited)
max_ram = 0
# Kernel heap size (KB)
heap_size = 4096
# Stack size per thread (KB)
stack_size = 32
# Enable virtual memory
virtual_memory = true
# Enable huge pages
huge_pages = false
# Physical allocator: bitmap, buddy
physical_allocator = "buddy"
# Heap allocator: bump, slab
heap_allocator = "slab"

# Scheduler configuration
[scheduler]
# Scheduler module: round_robin, cfs, realtime, cooperative
module = "round_robin"
# Default time slice (ms)
time_slice = 10
# Priority levels
priority_levels = 140
# Enable SMP load balancing
load_balancing = true
# Preemption: none, voluntary, full
preemption = "full"

# Module system
[modules]
# Enable hot reload
hot_reload = true
# Enable userspace modules
userspace_modules = false
# Module verification
verify_modules = true
# Default module load path
module_path = "/lib/modules"

# Security configuration
[security]
# Enable capability system
capabilities = true
# Enable MAC (Mandatory Access Control)
mac = false
# Enable ASLR
aslr = true
# Enable stack canaries
stack_canaries = true
# Enable SMEP/SMAP (x86_64)
smep_smap = true

# Console configuration
[console]
# Console backend: serial, vga, framebuffer
backend = "serial"
# Serial port settings
serial_port = "COM1"
serial_baud = 115200
# Early console for boot messages
early_console = true
# Log level: debug, info, warn, error
log_level = "info"

# Debug configuration
[debug]
# Enable kernel debugger
debugger = false
# Enable stack traces
stack_traces = true
# Enable kernel symbols
symbols = false
# Panic behavior: halt, reboot, dump
panic_behavior = "halt"
# Debug console port
debug_port = 0x402

# Features (can be disabled to reduce size)
[features]
# SMP support
smp = true
# ACPI support
acpi = true
# Power management
power_management = true
# Networking stack
networking = false
# Filesystem support
filesystem = true
# Graphics support
graphics = false
EOF

    log_success "Created kernel config: ${config_file}"
}

generate_profile() {
    local name="$1"
    local target="${2:-desktop}"
    local profile_dir="${PROFILES_DIR}/${name}"
    
    mkdir -p "${profile_dir}/src"
    
    # helix.toml
    cat > "${profile_dir}/helix.toml" << EOF
# =============================================================================
# Helix OS Profile: ${name}
# =============================================================================
# Generated on: $(date)
# Target: ${target}
# =============================================================================

[profile]
name = "${name}"
version = "1.0.0"
description = "Custom ${target} OS profile"
target = "${target}"

[profile.arch]
primary = "x86_64"
supported = ["x86_64", "aarch64"]

[profile.features]
EOF

    # Add features based on target
    case "${target}" in
        minimal|embedded)
            cat >> "${profile_dir}/helix.toml" << 'EOF'
multicore = false
hot_reload = false
userspace = false
networking = false
filesystem = false
graphics = false
EOF
            ;;
        server)
            cat >> "${profile_dir}/helix.toml" << 'EOF'
multicore = true
hot_reload = true
userspace = true
networking = true
filesystem = true
graphics = false
EOF
            ;;
        desktop)
            cat >> "${profile_dir}/helix.toml" << 'EOF'
multicore = true
hot_reload = true
userspace = true
networking = true
filesystem = true
graphics = true
EOF
            ;;
        secure)
            cat >> "${profile_dir}/helix.toml" << 'EOF'
multicore = true
hot_reload = false
userspace = true
networking = false
filesystem = true
graphics = false
EOF
            ;;
    esac
    
    # Continue with common config
    cat >> "${profile_dir}/helix.toml" << 'EOF'

[memory]
min_ram_mb = 64
max_ram_mb = 0
heap_size_kb = 4096
virtual_memory = true

[scheduler]
module = "round_robin"
time_slice_ms = 10
load_balancing = true

[modules.static]
modules = []

[modules.dynamic]
modules = []

[boot]
cmdline = "console=serial loglevel=4"

[debug]
level = "normal"
symbols = true
stack_traces = true
EOF

    # Cargo.toml
    cat > "${profile_dir}/Cargo.toml" << EOF
[package]
name = "helix-${name}-os"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "${name} OS built with Helix Framework"

[dependencies]
helix-hal = { workspace = true }
helix-core = { workspace = true }
helix-execution = { workspace = true }
helix-memory = { workspace = true }
helix-modules = { workspace = true }

[features]
default = []

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
lto = true
opt-level = "s"
EOF

    # main.rs
    cat > "${profile_dir}/src/main.rs" << 'EOF'
//! Helix OS - ${NAME}
//!
//! Custom OS built with Helix Framework.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Initialize kernel
    init();
    
    // Main loop
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn init() {
    // TODO: Initialize your OS here
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe { core::arch::asm!("hlt"); }
    }
}
EOF

    sed -i "s/\${NAME}/${name}/g" "${profile_dir}/src/main.rs"
    
    log_success "Created profile: ${profile_dir}"
    echo ""
    log_info "Files created:"
    print_bullet "${profile_dir}/helix.toml"
    print_bullet "${profile_dir}/Cargo.toml"
    print_bullet "${profile_dir}/src/main.rs"
}

# =============================================================================
# Interactive Configuration
# =============================================================================

interactive_kernel_config() {
    print_header "Helix Kernel Configuration Generator"
    
    # Name
    read -p "Configuration name [default]: " name
    name="${name:-default}"
    
    # Architecture
    echo ""
    echo "Target architecture:"
    echo "  1) x86_64"
    echo "  2) aarch64"
    echo "  3) riscv64"
    read -p "Select [1]: " arch_choice
    
    case "${arch_choice}" in
        2) arch="aarch64" ;;
        3) arch="riscv64" ;;
        *) arch="x86_64" ;;
    esac
    
    generate_kernel_config "${name}" "${arch}"
}

interactive_profile() {
    print_header "Helix OS Profile Generator"
    
    # Name
    read -p "Profile name: " name
    if [[ -z "${name}" ]]; then
        log_error "Name is required"
        exit 1
    fi
    
    # Target
    echo ""
    echo "Target type:"
    echo "  1) Desktop (full-featured)"
    echo "  2) Server (networking, no graphics)"
    echo "  3) Embedded (minimal)"
    echo "  4) Secure (hardened)"
    read -p "Select [1]: " target_choice
    
    case "${target_choice}" in
        2) target="server" ;;
        3) target="embedded" ;;
        4) target="secure" ;;
        *) target="desktop" ;;
    esac
    
    generate_profile "${name}" "${target}"
}

# =============================================================================
# Usage
# =============================================================================

print_usage() {
    echo -e "${HELIX_PRIMARY}"
    cat << 'EOF'
    ██╗  ██╗███████╗██╗     ██╗██╗  ██╗
    ██║  ██║██╔════╝██║     ██║╚██╗██╔╝
    ███████║█████╗  ██║     ██║ ╚███╔╝ 
    ██╔══██║██╔══╝  ██║     ██║ ██╔██╗ 
    ██║  ██║███████╗███████╗██║██╔╝ ██╗
    ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═╝
EOF
    echo -e "${RESET}"
    echo -e "${DIM}    Configuration Generator${RESET}"
    echo ""
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 <command> [args]"
    echo ""
    
    echo -e "${BOLD}Commands:${RESET}"
    echo "  kernel [name] [arch]    Generate kernel configuration"
    echo "  profile [name] [target] Generate OS profile"
    echo "  interactive             Interactive configuration wizard"
    echo "  help                    Show this help"
    echo ""
    
    echo -e "${BOLD}Examples:${RESET}"
    echo "  $0 kernel myconfig x86_64"
    echo "  $0 profile myos desktop"
    echo "  $0 interactive"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    local command="${1:-help}"
    shift || true
    
    case "${command}" in
        kernel)
            local name="${1:-default}"
            local arch="${2:-x86_64}"
            generate_kernel_config "${name}" "${arch}"
            ;;
        profile)
            local name="${1:-}"
            local target="${2:-desktop}"
            if [[ -z "${name}" ]]; then
                log_error "Profile name required"
                exit 1
            fi
            generate_profile "${name}" "${target}"
            ;;
        interactive)
            echo ""
            echo "What would you like to generate?"
            echo "  1) Kernel configuration"
            echo "  2) OS profile"
            read -p "Select: " choice
            
            case "${choice}" in
                1) interactive_kernel_config ;;
                2) interactive_profile ;;
                *) log_error "Invalid choice" ;;
            esac
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            log_error "Unknown command: ${command}"
            print_usage
            exit 1
            ;;
    esac
}

main "$@"
