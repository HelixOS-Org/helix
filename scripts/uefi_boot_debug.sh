#!/bin/bash
#
# UEFI Boot Debugger - Simple Working Version
# Creates bootable ESP image without permission issues

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELIX_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { echo -e "${BLUE}[DEBUG]${NC} $1"; }

# Simple working ESP creator
create_working_esp() {
    log_info "Creating Working ESP via dd + mkfs"
    log_info "=================================="

    local test_dir="$HELIX_ROOT/build/uefi_test"
    local esp_img="$test_dir/esp_fixed.img"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    # Clean
    rm -rf "$test_dir"
    mkdir -p "$test_dir"

    # Check kernel
    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        exit 1
    fi

    log_info "✓ EFI kernel: $(file "$efi_kernel" | cut -d: -f2-)"

    # Create 16MB image (smaller to avoid FAT32 warning)
    log_info "Creating 16MB ESP image..."
    dd if=/dev/zero of="$esp_img" bs=1M count=16 2>/dev/null

    # Format as FAT16 to avoid warnings
    mkfs.fat -F16 -n "HELIX" "$esp_img" >/dev/null 2>&1

    # Create temporary mount dir
    local temp_mount="/tmp/helix_esp_$$"
    mkdir -p "$temp_mount"

    # Mount with user permissions
    if ! sudo mount -o loop,uid=$(id -u),gid=$(id -g),dmask=022,fmask=133 "$esp_img" "$temp_mount"; then
        log_error "Failed to mount ESP image"
        rmdir "$temp_mount"
        exit 1
    fi

    # Create structure
    mkdir -p "$temp_mount/EFI/BOOT"
    mkdir -p "$temp_mount/EFI/helix"

    # Copy files
    cp "$efi_kernel" "$temp_mount/EFI/BOOT/BOOTX64.EFI"
    cp "$efi_kernel" "$temp_mount/EFI/helix/kernel.efi"

    # Create startup scripts
    cat > "$temp_mount/startup.nsh" << 'EOF'
@echo off
cls
echo ===============================
echo Helix OS UEFI Boot Test
echo ===============================
echo.
\EFI\BOOT\BOOTX64.EFI
EOF

    cat > "$temp_mount/EFI/BOOT/startup.nsh" << 'EOF'
@echo off
echo Booting Helix OS...
BOOTX64.EFI
EOF

    # Show contents
    log_info "ESP Contents:"
    find "$temp_mount" -type f | sort | sed "s|$temp_mount|  |"

    # Unmount
    sudo umount "$temp_mount"
    rmdir "$temp_mount"

    log_info "✓ ESP image ready: $esp_img"

    # Copy OVMF vars
    cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"

    # Show image info
    log_debug "Image info: $(file "$esp_img" | cut -d: -f2-)"
    log_debug "Size: $(ls -lh "$esp_img" | awk '{print $5}')"

    return 0
}

# Test the fixed ESP
test_fixed_esp() {
    local test_dir="$HELIX_ROOT/build/uefi_test"
    local esp_img="$test_dir/esp_fixed.img"

    if [[ ! -f "$esp_img" ]]; then
        log_error "ESP image not found, creating..."
        create_working_esp
    fi

    log_info "Testing Fixed ESP Boot"
    log_info "======================"

    local qemu_cmd=(
        qemu-system-x86_64
        -enable-kvm
        -m 512M
        -smp 2
        -display gtk,grab-on-hover=on
        -serial stdio
        -monitor none

        # UEFI firmware
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd
        -drive if=pflash,format=raw,file="$test_dir/OVMF_VARS.4m.fd"

        # ESP disk as IDE (most compatible)
        -drive format=raw,file="$esp_img",if=ide,index=0,media=disk

        # Boot from first disk
        -boot order=c,menu=on,strict=on

        # Other options
        -rtc base=utc
        -no-reboot
    )

    log_info "Command: qemu-system-x86_64 [UEFI with ESP disk]"
    echo ""
    log_warn "🎯 Expected Results:"
    log_warn "  1. BdsDxe detects bootable ESP disk"
    log_warn "  2. Auto-boots BOOTX64.EFI or shows Boot Manager"
    log_warn "  3. No 'Not Found' errors"
    echo ""
    log_info "🚀 Starting QEMU..."

    # Execute
    exec "${qemu_cmd[@]}"
}

# Alternative: UEFI Shell direct boot
test_shell_direct() {
    log_info "Testing UEFI Shell Direct Access"
    log_info "================================="

    local test_dir="$HELIX_ROOT/build/uefi_test"
    local esp_img="$test_dir/esp_fixed.img"

    if [[ ! -f "$esp_img" ]]; then
        create_working_esp
    fi

    local qemu_cmd=(
        qemu-system-x86_64
        -enable-kvm
        -m 512M
        -smp 2
        -display gtk,grab-on-hover=on
        -serial stdio
        -monitor none

        # UEFI firmware
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd
        -drive if=pflash,format=raw,file="$test_dir/OVMF_VARS.4m.fd"

        # ESP disk
        -drive format=raw,file="$esp_img",if=ide,index=0,media=disk

        # Force UEFI Shell
        -boot menu=on,order=c

        # Debug options
        -rtc base=utc
        -no-reboot
        -d guest_errors
    )

    log_info "Command: qemu-system-x86_64 [Force UEFI Shell]"
    echo ""
    log_warn "🎯 Manual Steps in UEFI Shell:"
    log_warn "  1. Type 'map' to see disk mappings"
    log_warn "  2. Type 'fs0:' to access ESP"
    log_warn "  3. Type 'startup.nsh' to run boot script"
    log_warn "  4. Or 'EFI\\BOOT\\BOOTX64.EFI' directly"
    echo ""

    exec "${qemu_cmd[@]}"
}

# Quick diagnostics
diagnose_issue() {
    log_info "UEFI Boot Issue Diagnosis"
    log_info "=========================="

    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    echo ""
    log_debug "1. EFI Kernel Validation:"
    if [[ -f "$efi_kernel" ]]; then
        echo "  ✓ File exists: $efi_kernel"
        echo "  ✓ Format: $(file "$efi_kernel" | cut -d: -f2-)"
        echo "  ✓ Size: $(ls -lh "$efi_kernel" | awk '{print $5}')"

        # Check PE signature
        local sig=$(head -c 2 "$efi_kernel" | hexdump -v -e '"%02x"')
        if [[ "$sig" == "4d5a" ]]; then
            echo "  ✓ DOS signature: MZ"
        else
            echo "  ✗ Invalid DOS signature: $sig"
        fi
    else
        echo "  ✗ EFI kernel not found!"
    fi

    echo ""
    log_debug "2. OVMF Files:"
    if [[ -f /usr/share/edk2/x64/OVMF_CODE.4m.fd ]]; then
        echo "  ✓ OVMF_CODE available"
    else
        echo "  ✗ OVMF_CODE missing"
    fi

    if [[ -f /usr/share/edk2/x64/OVMF_VARS.4m.fd ]]; then
        echo "  ✓ OVMF_VARS template available"
    else
        echo "  ✗ OVMF_VARS template missing"
    fi

    echo ""
    log_debug "3. System Tools:"
    echo "  mkfs.fat: $(which mkfs.fat || echo "not found")"
    echo "  qemu-system-x86_64: $(which qemu-system-x86_64 || echo "not found")"
    echo "  Loop support: $(lsmod | grep loop >/dev/null && echo "loaded" || echo "not loaded")"

    echo ""
    log_debug "4. Known Issues & Solutions:"
    echo "  Problem: BdsDxe 'Not Found' error"
    echo "  Cause 1: ESP not properly formatted/detected"
    echo "  Solution: Use real disk image instead of QEMU fat:"
    echo ""
    echo "  Problem: Boot Manager appears but no auto-boot"
    echo "  Cause 2: No valid EFI boot entries"
    echo "  Solution: Access UEFI Shell manually"
    echo ""
    echo "  Problem: BOOTX64.EFI not found in /EFI/BOOT/"
    echo "  Cause 3: File permissions or path issues"
    echo "  Solution: Verify ESP structure with working mount"
}

# Main
main() {
    case "${1:-help}" in
        "create"|"make")
            create_working_esp
            ;;
        "test"|"boot")
            test_fixed_esp
            ;;
        "shell")
            test_shell_direct
            ;;
        "both")
            create_working_esp
            test_fixed_esp
            ;;
        "diagnose"|"debug"|"check")
            diagnose_issue
            ;;
        "help"|"-h"|"--help")
            echo "UEFI Boot Debugger v1.0"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  create, make   Create working ESP image"
            echo "  test, boot     Test ESP boot with QEMU"
            echo "  shell          Force UEFI shell access"
            echo "  both           Create and test ESP"
            echo "  diagnose       Run diagnostics"
            echo "  help           Show this help"
            echo ""
            echo "Examples:"
            echo "  $0 both        # Create ESP and test (recommended)"
            echo "  $0 shell       # Access UEFI shell for manual testing"
            echo "  $0 diagnose    # Check for issues"
            ;;
        *)
            log_error "Unknown command: $1"
            log_info "Use '$0 help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
