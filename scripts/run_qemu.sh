#!/bin/bash
# =============================================================================
# Helix OS Framework - QEMU Runner
# =============================================================================
# Launch Helix in QEMU with various options
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

# =============================================================================
# Configuration
# =============================================================================

HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"
KERNEL_PATH="${HELIX_ROOT}/build/output/helix-kernel"
ISO_PATH="${HELIX_ROOT}/build/output/helix.iso"

# Default QEMU options
QEMU_ARCH="${QEMU_ARCH:-x86_64}"
QEMU_MEMORY="${QEMU_MEMORY:-256M}"
QEMU_CPUS="${QEMU_CPUS:-1}"
QEMU_DEBUG="${QEMU_DEBUG:-0}"
QEMU_GDB_PORT="${QEMU_GDB_PORT:-1234}"
QEMU_SERIAL="${QEMU_SERIAL:-stdio}"
QEMU_DISPLAY="${QEMU_DISPLAY:-none}"
QEMU_KVM="${QEMU_KVM:-auto}"

# =============================================================================
# QEMU Configuration Builders
# =============================================================================

build_qemu_cmd() {
    local qemu_bin=""
    local qemu_args=()
    
    # Select QEMU binary based on architecture
    case "${QEMU_ARCH}" in
        x86_64)
            qemu_bin="qemu-system-x86_64"
            qemu_args+=("-machine" "q35")
            ;;
        aarch64)
            qemu_bin="qemu-system-aarch64"
            qemu_args+=("-machine" "virt")
            qemu_args+=("-cpu" "cortex-a72")
            ;;
        riscv64)
            qemu_bin="qemu-system-riscv64"
            qemu_args+=("-machine" "virt")
            ;;
        *)
            log_error "Unsupported architecture: ${QEMU_ARCH}"
            exit 1
            ;;
    esac
    
    # Check if QEMU exists
    if ! cmd_exists "${qemu_bin}"; then
        log_error "${qemu_bin} not found. Please install QEMU."
        exit 1
    fi
    
    # Memory and CPU
    qemu_args+=("-m" "${QEMU_MEMORY}")
    qemu_args+=("-smp" "${QEMU_CPUS}")
    
    # KVM acceleration
    if [[ "${QEMU_KVM}" == "auto" ]]; then
        if [[ -e "/dev/kvm" ]] && [[ "${QEMU_ARCH}" == "x86_64" ]]; then
            qemu_args+=("-enable-kvm")
            echo "KVM acceleration enabled" >&2
        fi
    elif [[ "${QEMU_KVM}" == "1" ]]; then
        qemu_args+=("-enable-kvm")
    fi
    
    # Serial console
    case "${QEMU_SERIAL}" in
        stdio)
            qemu_args+=("-serial" "stdio")
            ;;
        file)
            qemu_args+=("-serial" "file:${HELIX_ROOT}/build/logs/serial.log")
            ;;
        none)
            ;;
        *)
            qemu_args+=("-serial" "${QEMU_SERIAL}")
            ;;
    esac
    
    # Display
    case "${QEMU_DISPLAY}" in
        none)
            qemu_args+=("-display" "none")
            ;;
        gtk|sdl|cocoa)
            qemu_args+=("-display" "${QEMU_DISPLAY}")
            ;;
        vnc)
            qemu_args+=("-vnc" ":0")
            ;;
    esac
    
    # Debug mode
    if [[ "${QEMU_DEBUG}" == "1" ]]; then
        qemu_args+=("-s")  # GDB server on port 1234
        qemu_args+=("-S")  # Freeze CPU at startup
        echo "Debug mode: GDB server on port ${QEMU_GDB_PORT}" >&2
        echo "Connect with: gdb -ex 'target remote :${QEMU_GDB_PORT}'" >&2
    fi
    
    # Boot options
    # NOTE: log_info must write to stderr here, not stdout, because
    # this function's stdout is captured by command substitution
    if [[ -f "${ISO_PATH}" ]]; then
        qemu_args+=("-cdrom" "${ISO_PATH}")
        qemu_args+=("-boot" "d")
        echo "Booting from ISO: ${ISO_PATH}" >&2
    elif [[ -f "${KERNEL_PATH}" ]]; then
        qemu_args+=("-kernel" "${KERNEL_PATH}")
        echo "Booting kernel: ${KERNEL_PATH}" >&2
    else
        log_error "No kernel or ISO found!"
        log_error "Run './scripts/build.sh all' first"
        exit 1
    fi
    
    # Additional devices for x86_64
    if [[ "${QEMU_ARCH}" == "x86_64" ]]; then
        # VirtIO devices
        qemu_args+=("-device" "virtio-serial-pci")
        
        # Debug console
        qemu_args+=("-debugcon" "file:${HELIX_ROOT}/build/logs/debug.log")
        qemu_args+=("-global" "isa-debugcon.iobase=0x402")
    fi
    
    # Return command
    echo "${qemu_bin}" "${qemu_args[@]}"
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
    echo -e "${DIM}    QEMU Runner${RESET}"
    echo ""
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 [options]"
    echo ""
    
    echo -e "${BOLD}Options:${RESET}"
    echo "  -a, --arch <arch>     Architecture (x86_64, aarch64, riscv64)"
    echo "  -m, --memory <size>   Memory size (e.g., 256M, 1G)"
    echo "  -c, --cpus <n>        Number of CPUs"
    echo "  -d, --debug           Enable GDB debugging"
    echo "  -g, --graphics        Enable graphics display"
    echo "  -k, --kvm             Force KVM acceleration"
    echo "  --no-kvm              Disable KVM acceleration"
    echo "  -v, --vnc             Enable VNC display"
    echo "  -h, --help            Show this help"
    echo ""
    
    echo -e "${BOLD}Examples:${RESET}"
    echo "  $0                    # Run with defaults"
    echo "  $0 -m 512M -c 2       # 512MB RAM, 2 CPUs"
    echo "  $0 --debug            # With GDB debugging"
    echo "  $0 -g                 # With graphics"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -a|--arch)
                QEMU_ARCH="$2"
                shift 2
                ;;
            -m|--memory)
                QEMU_MEMORY="$2"
                shift 2
                ;;
            -c|--cpus)
                QEMU_CPUS="$2"
                shift 2
                ;;
            -d|--debug)
                QEMU_DEBUG=1
                shift
                ;;
            -g|--graphics)
                QEMU_DISPLAY="gtk"
                shift
                ;;
            -k|--kvm)
                QEMU_KVM=1
                shift
                ;;
            --no-kvm)
                QEMU_KVM=0
                shift
                ;;
            -v|--vnc)
                QEMU_DISPLAY="vnc"
                shift
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    # Ensure log directory exists
    mkdir -p "${HELIX_ROOT}/build/logs"
    
    # Build and run QEMU command
    print_header "Running Helix in QEMU"
    
    print_keyvalue "Architecture" "${QEMU_ARCH}"
    print_keyvalue "Memory" "${QEMU_MEMORY}"
    print_keyvalue "CPUs" "${QEMU_CPUS}"
    print_keyvalue "Display" "${QEMU_DISPLAY}"
    echo ""
    
    local qemu_cmd=$(build_qemu_cmd)
    
    log_info "Starting QEMU..."
    log_debug "Command: ${qemu_cmd}"
    echo ""
    
    print_separator
    
    # Run QEMU
    exec ${qemu_cmd}
}

main "$@"
