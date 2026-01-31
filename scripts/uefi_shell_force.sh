#!/bin/bash
#
# Force UEFI Shell Boot - Garantit l'accès au Shell UEFI

set -euo pipefail

HELIX_ROOT="$(dirname "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)")"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

force_uefi_shell() {
    log_info "🐚 Forcing UEFI Shell Boot"
    log_info "=========================="

    local test_dir="$HELIX_ROOT/build/uefi_shell"
    local disk_img="$test_dir/shell_disk.img"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    # Cleanup
    sudo losetup -D 2>/dev/null || true
    rm -rf "$test_dir"
    mkdir -p "$test_dir"

    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found"
        exit 1
    fi

    # Locate UEFI Shell
    local shell_efi=""
    for path in \
        /usr/share/edk2/x64/Shell.efi \
        /usr/share/edk2-shell/x64/Shell.efi \
        /usr/share/OVMF/Shell.efi \
        /usr/share/ovmf/Shell.efi; do
        if [[ -f "$path" ]]; then
            shell_efi="$path"
            break
        fi
    done

    if [[ -z "$shell_efi" ]]; then
        log_warn "UEFI Shell not found, downloading..."
        curl -sL "https://github.com/tianocore/edk2/raw/edk2-stable202311/ShellBinPkg/UefiShell/X64/Shell.efi" \
            -o "$test_dir/Shell.efi" || true
        shell_efi="$test_dir/Shell.efi"
    fi

    log_info "✓ UEFI Shell: $shell_efi"

    # Create 64MB disk
    log_info "Creating ESP disk with Shell..."
    dd if=/dev/zero of="$disk_img" bs=1M count=64 2>/dev/null

    # Create GPT + ESP
    local loop_dev=$(sudo losetup -f --show "$disk_img")
    sudo sgdisk -Z "$loop_dev" >/dev/null 2>&1
    sudo sgdisk -n 1:2048:+56M -t 1:ef00 -c 1:"ESP" "$loop_dev" >/dev/null 2>&1
    sudo partprobe "$loop_dev" 2>/dev/null || true
    sleep 1

    # Format as FAT32
    sudo mkfs.fat -F32 -n "HELIX" "${loop_dev}p1" >/dev/null 2>&1

    # Mount
    local mnt="/tmp/helix_shell_$$"
    mkdir -p "$mnt"
    sudo mount "${loop_dev}p1" "$mnt"

    # Create structure - Shell.efi as default boot
    sudo mkdir -p "$mnt/EFI/BOOT"
    sudo mkdir -p "$mnt/EFI/helix"

    # CRITICAL: Copy Shell.efi as BOOTX64.EFI for auto-boot
    if [[ -f "$shell_efi" ]]; then
        sudo cp "$shell_efi" "$mnt/EFI/BOOT/BOOTX64.EFI"
        log_info "✓ Shell.efi copied as BOOTX64.EFI (auto-boot)"
    fi

    # Copy Helix kernel separately
    sudo cp "$efi_kernel" "$mnt/EFI/helix/helix.efi"

    # Create startup.nsh that runs automatically in Shell
    sudo tee "$mnt/startup.nsh" >/dev/null << 'EOF'
@echo off
cls
echo =============================================
echo HELIX OS - UEFI SHELL ACTIVE
echo =============================================
echo.
echo Available commands:
echo   helix      - Boot Helix OS kernel
echo   map        - Show disk mappings
echo   ls         - List files
echo   exit       - Exit shell
echo.
echo Type 'helix' to boot Helix OS
echo =============================================
EOF

    # Create helix boot script
    sudo tee "$mnt/helix.nsh" >/dev/null << 'EOF'
@echo off
echo Loading Helix OS kernel...
\EFI\helix\helix.efi
EOF

    # Sync and unmount
    sudo sync
    sudo umount "$mnt"
    rmdir "$mnt"
    sudo losetup -d "$loop_dev"

    # Copy OVMF vars
    cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"

    log_info "✅ Shell ESP ready"
    echo ""

    # Launch QEMU
    log_info "🚀 Starting QEMU with UEFI Shell..."
    echo ""
    log_warn "📝 COMMANDS IN SHELL:"
    log_warn "   fs0:              - Switch to ESP"
    log_warn "   ls                - List files"
    log_warn "   helix.nsh         - Boot Helix kernel"
    log_warn "   \\EFI\\helix\\helix.efi - Direct boot"
    echo ""

    exec qemu-system-x86_64 \
        -enable-kvm \
        -m 512M \
        -smp 2 \
        -display gtk,grab-on-hover=on \
        -serial stdio \
        -monitor none \
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd \
        -drive if=pflash,format=raw,file="$test_dir/OVMF_VARS.4m.fd" \
        -drive format=raw,file="$disk_img",if=ide,index=0,media=disk \
        -boot order=c \
        -rtc base=utc \
        -no-reboot
}

force_uefi_shell
