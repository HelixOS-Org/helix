#!/bin/bash
# =============================================================================
# Helix OS Framework - Limine QEMU Runner
# =============================================================================
# Launch Helix with Limine bootloader in QEMU graphical window
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELIX_ROOT="${SCRIPT_DIR}/.."

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
BUILD_DIR="${HELIX_ROOT}/build"
ISO_DIR="${BUILD_DIR}/limine-iso"
KERNEL_PATH="${BUILD_DIR}/output/helix-kernel"
ISO_PATH="${BUILD_DIR}/output/helix-limine.iso"

# QEMU options
QEMU_MEMORY="${QEMU_MEMORY:-512M}"
QEMU_CPUS="${QEMU_CPUS:-2}"
QEMU_DISPLAY="${QEMU_DISPLAY:-gtk}"  # gtk, sdl, or cocoa on macOS

# =============================================================================
# Logging
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# =============================================================================
# Check Dependencies
# =============================================================================

check_dependencies() {
    local missing=()

    if ! command -v qemu-system-x86_64 &> /dev/null; then
        missing+=("qemu-system-x86_64")
    fi

    if ! command -v xorriso &> /dev/null; then
        missing+=("xorriso")
    fi

    if [[ ! -f /usr/share/limine/limine-bios-cd.bin ]]; then
        missing+=("limine")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo pacman -S ${missing[*]}"
        exit 1
    fi

    log_success "All dependencies found"
}

# =============================================================================
# Build Kernel (if needed)
# =============================================================================

build_kernel() {
    if [[ ! -f "${KERNEL_PATH}" ]]; then
        log_info "Kernel not found, building..."
        "${SCRIPT_DIR}/build.sh"
    else
        log_info "Using existing kernel: ${KERNEL_PATH}"
    fi
}

# =============================================================================
# Create Limine ISO
# =============================================================================

create_limine_iso() {
    log_info "Creating Limine bootable ISO..."

    # Clean and create ISO directory structure
    rm -rf "${ISO_DIR}"
    mkdir -p "${ISO_DIR}/boot/limine"
    mkdir -p "${ISO_DIR}/EFI/BOOT"

    # Copy kernel
    cp "${KERNEL_PATH}" "${ISO_DIR}/boot/helix-kernel"

    # Create limine.conf
    cat > "${ISO_DIR}/boot/limine/limine.conf" << 'EOF'
# Helix OS Framework - Limine Configuration
# ===========================================

# Timeout in seconds (0 = instant boot)
timeout: 5

# Default entry
default_entry: 1

# Terminal configuration
interface_resolution: 1024x768
# interface_branding: Helix OS Framework

# ===========================================
# Boot Entries
# ===========================================

/Helix OS Framework
    protocol: multiboot2
    kernel_path: boot():/boot/helix-kernel

    # Request linear framebuffer from bootloader
    resolution: 1024x768x32

    # Kernel command line
    cmdline: helix.log_level=debug helix.serial=0x3F8

/Helix OS (Serial Debug)
    protocol: multiboot2
    kernel_path: boot():/boot/helix-kernel
    cmdline: helix.log_level=trace helix.serial=0x3F8 helix.debug=1

/Helix OS (Safe Mode)
    protocol: multiboot2
    kernel_path: boot():/boot/helix-kernel
    cmdline: helix.safe_mode=1 helix.log_level=info
EOF

    # Copy Limine files
    cp /usr/share/limine/limine-bios.sys "${ISO_DIR}/boot/limine/"
    cp /usr/share/limine/limine-bios-cd.bin "${ISO_DIR}/boot/limine/"
    cp /usr/share/limine/limine-uefi-cd.bin "${ISO_DIR}/boot/limine/"

    # Copy UEFI bootloaders
    cp /usr/share/limine/BOOTX64.EFI "${ISO_DIR}/EFI/BOOT/"
    cp /usr/share/limine/BOOTIA32.EFI "${ISO_DIR}/EFI/BOOT/"

    # Create the ISO with xorriso
    log_info "Building ISO with xorriso..."
    xorriso -as mkisofs \
        -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part \
        --efi-boot-image \
        --protective-msdos-label \
        -o "${ISO_PATH}" \
        "${ISO_DIR}" \
        2>&1 | grep -v "^xorriso" || true

    # Install Limine BIOS stages
    log_info "Installing Limine BIOS boot stages..."
    limine bios-install "${ISO_PATH}" 2>/dev/null || {
        log_warn "Could not install BIOS boot stages (UEFI boot still works)"
    }

    log_success "ISO created: ${ISO_PATH}"
    log_info "ISO size: $(du -h "${ISO_PATH}" | cut -f1)"
}

# =============================================================================
# Run QEMU
# =============================================================================

run_qemu() {
    log_info "Starting QEMU with graphical display..."
    log_info "  Memory: ${QEMU_MEMORY}"
    log_info "  CPUs: ${QEMU_CPUS}"
    log_info "  Display: ${QEMU_DISPLAY}"

    # Create log directory
    mkdir -p "${BUILD_DIR}/logs"

    # Build QEMU command
    local qemu_args=(
        # Machine configuration
        "-machine" "q35"
        "-m" "${QEMU_MEMORY}"
        "-smp" "${QEMU_CPUS}"

        # Display - GRAPHICAL WINDOW
        "-display" "${QEMU_DISPLAY}"
        "-vga" "std"

        # Boot from ISO
        "-cdrom" "${ISO_PATH}"
        "-boot" "d"

        # Serial output to file for debugging
        "-serial" "file:${BUILD_DIR}/logs/serial.log"

        # Debug console
        "-debugcon" "file:${BUILD_DIR}/logs/debug.log"
        "-global" "isa-debugcon.iobase=0x402"

        # VirtIO devices
        "-device" "virtio-serial-pci"
    )

    # Enable KVM if available
    if [[ -e "/dev/kvm" ]]; then
        qemu_args+=("-enable-kvm")
        log_info "KVM acceleration enabled"
    fi

    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  ${GREEN}Helix OS Framework${NC} - Starting with Limine bootloader       ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}                                                                ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}  ${YELLOW}Controls:${NC}                                                    ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}    • Ctrl+Alt+G  - Release mouse grab                          ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}    • Ctrl+Alt+F  - Toggle fullscreen                           ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}    • Ctrl+Alt+Q  - Quit QEMU                                   ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}                                                                ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}  ${YELLOW}Logs:${NC}                                                        ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}    • Serial: ${BUILD_DIR}/logs/serial.log                  ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}    • Debug:  ${BUILD_DIR}/logs/debug.log                   ${CYAN}║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Run QEMU
    qemu-system-x86_64 "${qemu_args[@]}"

    local exit_code=$?

    echo ""
    if [[ ${exit_code} -eq 0 ]]; then
        log_success "QEMU exited normally"
    else
        log_warn "QEMU exited with code: ${exit_code}"
    fi

    # Show last lines of serial log
    if [[ -f "${BUILD_DIR}/logs/serial.log" ]]; then
        echo ""
        log_info "Last 20 lines of serial output:"
        echo "─────────────────────────────────────────"
        tail -20 "${BUILD_DIR}/logs/serial.log" 2>/dev/null || true
        echo "─────────────────────────────────────────"
    fi
}

# =============================================================================
# Usage
# =============================================================================

show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Launch Helix OS with Limine bootloader in QEMU graphical window"
    echo ""
    echo "Options:"
    echo "  -m, --memory SIZE    Set memory size (default: 512M)"
    echo "  -c, --cpus COUNT     Set CPU count (default: 2)"
    echo "  -d, --display TYPE   Display type: gtk, sdl, vnc (default: gtk)"
    echo "  --rebuild            Force rebuild kernel before running"
    echo "  --iso-only           Only create ISO, don't run QEMU"
    echo "  -h, --help           Show this help"
    echo ""
    echo "Environment variables:"
    echo "  QEMU_MEMORY          Memory size"
    echo "  QEMU_CPUS            CPU count"
    echo "  QEMU_DISPLAY         Display type"
    echo ""
    echo "Examples:"
    echo "  $0                   # Run with defaults (512M, 2 CPUs, GTK display)"
    echo "  $0 -m 1G -c 4        # Run with 1GB RAM and 4 CPUs"
    echo "  $0 --display sdl     # Use SDL display instead of GTK"
}

# =============================================================================
# Main
# =============================================================================

main() {
    local rebuild=false
    local iso_only=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -m|--memory)
                QEMU_MEMORY="$2"
                shift 2
                ;;
            -c|--cpus)
                QEMU_CPUS="$2"
                shift 2
                ;;
            -d|--display)
                QEMU_DISPLAY="$2"
                shift 2
                ;;
            --rebuild)
                rebuild=true
                shift
                ;;
            --iso-only)
                iso_only=true
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}        ${CYAN}Helix OS Framework${NC} - Limine Boot Runner          ${GREEN}║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Check dependencies
    check_dependencies

    # Build kernel if needed
    if [[ "${rebuild}" == "true" ]] || [[ ! -f "${KERNEL_PATH}" ]]; then
        build_kernel
    fi

    # Create Limine ISO
    create_limine_iso

    # Run QEMU (unless --iso-only)
    if [[ "${iso_only}" != "true" ]]; then
        run_qemu
    else
        log_success "ISO created. Run with: qemu-system-x86_64 -cdrom ${ISO_PATH} -m 512M"
    fi
}

main "$@"
