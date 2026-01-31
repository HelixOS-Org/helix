#!/bin/bash
#
# FINAL UEFI ESP SOLUTION
# Creates proper GPT disk with ESP partition type EF00

set -euo pipefail

HELIX_ROOT="$(dirname "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)")"

# Colors
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { echo -e "${BLUE}[DEBUG]${NC} $1"; }

create_real_esp_disk() {
    log_info "🔧 Creating REAL ESP Disk with GPT"
    log_info "=================================="

    local test_dir="$HELIX_ROOT/build/uefi_final"
    local disk_img="$test_dir/helix_esp.img"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    # Cleanup
    sudo losetup -D 2>/dev/null || true
    sudo rm -rf "$test_dir"
    mkdir -p "$test_dir"

    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        exit 1
    fi

    log_info "✓ EFI kernel ready: $(file "$efi_kernel" | cut -d: -f2-)"

    # Create 128MB disk image
    log_info "Creating 128MB disk image..."
    dd if=/dev/zero of="$disk_img" bs=1M count=128 2>/dev/null

    # Setup loop device
    local loop_dev=$(sudo losetup -f --show "$disk_img")
    log_debug "Loop device: $loop_dev"

    # Create GPT with ESP partition
    log_info "Creating GPT partition table with ESP..."
    sudo sgdisk -Z "$loop_dev" >/dev/null 2>&1
    sudo sgdisk -n 1:2048:+64M -t 1:ef00 -c 1:"EFI System" "$loop_dev" >/dev/null 2>&1
    sudo partprobe "$loop_dev" 2>/dev/null || true
    sleep 1

    # Format ESP partition as FAT32
    log_info "Formatting ESP partition as FAT32..."
    sudo mkfs.fat -F32 -n "HELIX_ESP" "${loop_dev}p1" >/dev/null 2>&1

    # Create mount point in /tmp (avoid permission issues)
    local mount_dir="/tmp/helix_esp_mount_$$"
    mkdir -p "$mount_dir"

    # Mount ESP partition
    sudo mount "${loop_dev}p1" "$mount_dir"

    # Create EFI directory structure
    sudo mkdir -p "$mount_dir/EFI/BOOT"
    sudo mkdir -p "$mount_dir/EFI/helix"

    # Copy EFI bootloader
    sudo cp "$efi_kernel" "$mount_dir/EFI/BOOT/BOOTX64.EFI"
    sudo cp "$efi_kernel" "$mount_dir/EFI/helix/kernel.efi"

    # Create startup scripts
    sudo tee "$mount_dir/startup.nsh" >/dev/null << 'EOF'
@echo off
cls
echo ========================================
echo Helix OS - Final UEFI ESP Test
echo ========================================
echo.
echo Loading kernel from ESP...
\EFI\BOOT\BOOTX64.EFI
EOF

    sudo tee "$mount_dir/EFI/BOOT/startup.nsh" >/dev/null << 'EOF'
@echo off
echo Loading BOOTX64.EFI...
BOOTX64.EFI
EOF

    # Show ESP contents
    log_info "ESP Partition Contents:"
    sudo find "$mount_dir" -type f | sort | sed "s|$mount_dir|  |"

    # Sync and unmount
    sudo sync
    sudo umount "$mount_dir"
    rmdir "$mount_dir"

    # Detach loop device
    sudo losetup -d "$loop_dev"

    log_info "✅ ESP disk created: $disk_img"

    # Copy OVMF variables
    cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"

    # Show disk info
    echo ""
    log_debug "Disk Information:"
    echo "  Size: $(ls -lh "$disk_img" | awk '{print $5}')"
    echo "  Type: $(file "$disk_img" | cut -d: -f2-)"

    # Show partition table
    log_debug "Partition Table:"
    sgdisk -p "$disk_img" 2>/dev/null | grep -E "(Number|1)" | sed 's/^/  /' || true
}

test_real_esp_boot() {
    local test_dir="$HELIX_ROOT/build/uefi_final"
    local disk_img="$test_dir/helix_esp.img"

    if [[ ! -f "$disk_img" ]]; then
        log_error "ESP disk not found, creating..."
        create_real_esp_disk
    fi

    log_info "🚀 Testing REAL ESP Boot"
    log_info "========================"

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

        # ESP disk (should be recognized by BdsDxe)
        -drive format=raw,file="$disk_img",if=ide,index=0,media=disk

        # Boot settings
        -boot order=c,menu=on
        -rtc base=utc
        -no-reboot
    )

    echo ""
    log_warn "🎯 FINAL TEST - Expected Results:"
    log_warn "  ✅ BdsDxe detects GPT ESP partition"
    log_warn "  ✅ Auto-boots BOOTX64.EFI from ESP"
    log_warn "  ✅ Helix OS kernel loads without page fault"
    log_warn "  ❌ NO 'Not Found' errors from BdsDxe"
    echo ""
    log_info "Starting QEMU with REAL ESP disk..."
    echo ""

    exec "${qemu_cmd[@]}"
}

# Show final diagnostic
show_solution_summary() {
    log_info "🎯 UEFI ESP BOOT SOLUTION SUMMARY"
    log_info "=================================="
    echo ""
    log_debug "❌ PROBLEM IDENTIFIED:"
    echo "  - QEMU 'fat:rw:' creates virtual FAT, not real ESP"
    echo "  - BdsDxe requires GPT partition with type EF00"
    echo "  - Raw FAT images are not recognized as bootable"
    echo ""
    log_debug "✅ SOLUTION IMPLEMENTED:"
    echo "  - Created real GPT disk image with ESP partition"
    echo "  - Partition type: EF00 (EFI System Partition)"
    echo "  - Proper FAT32 filesystem on ESP partition"
    echo "  - Standard /EFI/BOOT/BOOTX64.EFI structure"
    echo ""
    log_debug "🔧 TECHNICAL DETAILS:"
    echo "  - 128MB GPT disk image"
    echo "  - 64MB ESP partition (type EF00)"
    echo "  - FAT32 filesystem with label HELIX_ESP"
    echo "  - Standard UEFI boot file layout"
    echo ""
    log_debug "📁 ESP STRUCTURE:"
    echo "  /EFI/BOOT/BOOTX64.EFI    ← UEFI boot loader"
    echo "  /EFI/BOOT/startup.nsh    ← Boot script"
    echo "  /EFI/helix/kernel.efi    ← Backup kernel"
    echo "  /startup.nsh             ← Root boot script"
    echo ""
    log_info "This should resolve the 'Not Found' error!"
}

main() {
    case "${1:-help}" in
        "create")
            create_real_esp_disk
            ;;
        "test"|"boot")
            test_real_esp_boot
            ;;
        "all"|"full")
            create_real_esp_disk
            test_real_esp_boot
            ;;
        "summary"|"solution")
            show_solution_summary
            ;;
        "help"|"-h"|"--help")
            echo "UEFI ESP Final Solution v1.0"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  create     Create GPT ESP disk only"
            echo "  test       Test ESP boot with QEMU"
            echo "  all, full  Create and test ESP (recommended)"
            echo "  summary    Show solution summary"
            echo "  help       Show this help"
            echo ""
            echo "This script creates a REAL GPT ESP disk"
            echo "to fix BdsDxe 'Not Found' boot errors."
            ;;
        *)
            log_error "Unknown command: $1"
            log_info "Use '$0 help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
