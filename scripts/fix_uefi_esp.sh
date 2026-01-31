#!/bin/bash
#
# UEFI ESP Image Creator - Creates proper EFI System Partition
# Fixes BdsDxe boot detection issues

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELIX_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { echo -e "${BLUE}[DEBUG]${NC} $1"; }

# Create proper ESP disk image
create_esp_image() {
    log_info "Creating EFI System Partition Image"
    log_info "===================================="

    local test_dir="$HELIX_ROOT/build/uefi_test"
    local esp_image="$test_dir/esp.img"
    local mount_point="$test_dir/mnt"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    # Cleanup previous
    sudo umount "$mount_point" 2>/dev/null || true
    rm -rf "$test_dir"
    mkdir -p "$test_dir" "$mount_point"

    # Validate EFI kernel exists
    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        log_info "Run: ./scripts/convert_to_efi.sh"
        exit 1
    fi

    log_info "✓ EFI kernel found: $efi_kernel"

    # Create ESP disk image (32MB, FAT32)
    log_info "Creating 32MB FAT32 ESP image..."
    dd if=/dev/zero of="$esp_image" bs=1M count=32 2>/dev/null

    # Format as FAT32 with ESP flags
    mkfs.fat -F32 -n "HELIX_ESP" "$esp_image" >/dev/null

    log_info "✓ ESP image created: $esp_image"

    # Mount ESP image
    log_info "Mounting ESP image..."
    sudo mount -o loop "$esp_image" "$mount_point"

    # Create EFI directory structure
    sudo mkdir -p "$mount_point/EFI/BOOT"
    sudo mkdir -p "$mount_point/EFI/helix"

    # Copy EFI files
    sudo cp "$efi_kernel" "$mount_point/EFI/BOOT/"
    sudo cp "$efi_kernel" "$mount_point/EFI/helix/kernel.efi"

    # Create UEFI startup script
    sudo tee "$mount_point/startup.nsh" >/dev/null << 'EOF'
@echo off
cls
echo ============================================
echo Helix OS - Pure UEFI Boot Test
echo ============================================
echo.
echo Attempting to load kernel...
echo.
\EFI\BOOT\BOOTX64.EFI
EOF

    # Create additional boot scripts
    sudo tee "$mount_point/EFI/BOOT/startup.nsh" >/dev/null << 'EOF'
@echo off
echo Loading Helix OS from BOOT directory...
BOOTX64.EFI
EOF

    sudo tee "$mount_point/EFI/helix/startup.nsh" >/dev/null << 'EOF'
@echo off
echo Loading Helix OS from helix directory...
kernel.efi
EOF

    # Show ESP contents
    log_info "ESP Contents:"
    find "$mount_point" -type f | sort | sed 's|^.*/mnt/|  /|'

    # Unmount ESP
    sudo umount "$mount_point"
    log_info "✓ ESP image ready"

    # Copy OVMF variables
    if [[ -f "$HELIX_ROOT/build/output/OVMF_VARS.4m.fd" ]]; then
        cp "$HELIX_ROOT/build/output/OVMF_VARS.4m.fd" "$test_dir/"
    else
        cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"
    fi

    log_info "✓ OVMF variables ready"
    echo ""

    # Show image info
    log_debug "ESP Image Details:"
    file "$esp_image" | sed 's/^/  /'
    ls -lh "$esp_image" | awk '{print "  Size: " $5}'
    echo ""
}

# Test ESP boot with proper disk image
test_esp_boot() {
    local test_dir="$HELIX_ROOT/build/uefi_test"
    local esp_image="$test_dir/esp.img"

    if [[ ! -f "$esp_image" ]]; then
        log_error "ESP image not found, creating..."
        create_esp_image
    fi

    log_info "Testing ESP Boot with Real Disk Image"
    log_info "======================================"

    # Build QEMU command with proper ESP disk
    local qemu_cmd=(
        qemu-system-x86_64
        -enable-kvm
        -m 256M
        -smp 1
        -display gtk,grab-on-hover=on
        -serial stdio
        -monitor none

        # UEFI firmware
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd
        -drive if=pflash,format=raw,file="$test_dir/OVMF_VARS.4m.fd"

        # Real ESP disk image (not FAT virtual)
        -drive format=raw,file="$esp_image",if=ide,index=0

        # Boot from disk
        -boot order=c,menu=on
        -rtc base=utc
        -no-reboot
    )

    log_info "Starting QEMU with real ESP disk..."
    log_debug "Command: ${qemu_cmd[*]}"
    echo ""
    log_warn "Expected behavior:"
    log_warn "  1. BdsDxe should detect ESP partition"
    log_warn "  2. Auto-boot BOOTX64.EFI or show Boot Manager"
    log_warn "  3. UEFI shell accessible if needed"
    echo ""
    log_info "Press Ctrl+C to exit QEMU"
    echo ""

    # Execute QEMU
    exec "${qemu_cmd[@]}"
}

# Create ESP with GPT partition table
create_gpt_esp() {
    log_info "Creating GPT ESP Disk Image"
    log_info "============================"

    local test_dir="$HELIX_ROOT/build/uefi_test"
    local disk_image="$test_dir/helix_uefi.img"
    local mount_point="$test_dir/mnt"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    # Cleanup
    sudo umount "$mount_point" 2>/dev/null || true
    rm -rf "$test_dir"
    mkdir -p "$test_dir" "$mount_point"

    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        exit 1
    fi

    # Create 64MB disk image
    log_info "Creating 64MB disk with GPT partition table..."
    dd if=/dev/zero of="$disk_image" bs=1M count=64 2>/dev/null

    # Create GPT partition table with ESP
    log_info "Creating GPT partition table..."
    sgdisk -Z "$disk_image" >/dev/null 2>&1
    sgdisk -n 1:2048:+32M -t 1:ef00 -c 1:"EFI System" "$disk_image" >/dev/null 2>&1

    # Setup loop device for partition
    log_info "Setting up loop device..."
    local loop_device
    loop_device=$(sudo losetup -f --show "$disk_image")
    sudo partprobe "$loop_device"

    # Format ESP partition
    log_info "Formatting ESP partition..."
    sudo mkfs.fat -F32 -n "HELIX_ESP" "${loop_device}p1" >/dev/null

    # Mount ESP partition
    sudo mount "${loop_device}p1" "$mount_point"

    # Create EFI structure
    mkdir -p "$mount_point/EFI/BOOT"
    mkdir -p "$mount_point/EFI/helix"

    # Copy EFI files
    cp "$efi_kernel" "$mount_point/EFI/BOOT/"
    cp "$efi_kernel" "$mount_point/EFI/helix/kernel.efi"

    # Create startup scripts
    cat > "$mount_point/startup.nsh" << 'EOF'
@echo off
cls
echo ==========================================
echo Helix OS - GPT ESP Boot Test
echo ==========================================
echo.
echo Booting from EFI System Partition...
\EFI\BOOT\BOOTX64.EFI
EOF

    # Show contents
    log_info "ESP Contents:"
    find "$mount_point" -type f | sort | sed "s|$mount_point|  |"

    # Cleanup
    sudo umount "$mount_point"
    sudo losetup -d "$loop_device"

    log_info "✓ GPT ESP disk ready: $disk_image"

    # Copy OVMF variables
    if [[ -f "$HELIX_ROOT/build/output/OVMF_VARS.4m.fd" ]]; then
        cp "$HELIX_ROOT/build/output/OVMF_VARS.4m.fd" "$test_dir/"
    else
        cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"
    fi

    echo ""
    log_debug "Disk Image Details:"
    file "$disk_image" | sed 's/^/  /'
    ls -lh "$disk_image" | awk '{print "  Size: " $5}'
    sgdisk -p "$disk_image" 2>/dev/null | grep -E "(Number|1)" | sed 's/^/  /'
}

# Test GPT ESP boot
test_gpt_boot() {
    local test_dir="$HELIX_ROOT/build/uefi_test"
    local disk_image="$test_dir/helix_uefi.img"

    if [[ ! -f "$disk_image" ]]; then
        log_error "GPT disk image not found, creating..."
        create_gpt_esp
    fi

    log_info "Testing GPT ESP Boot"
    log_info "===================="

    local qemu_cmd=(
        qemu-system-x86_64
        -enable-kvm
        -m 256M
        -smp 1
        -display gtk,grab-on-hover=on
        -serial stdio
        -monitor none

        # UEFI firmware
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd
        -drive if=pflash,format=raw,file="$test_dir/OVMF_VARS.4m.fd"

        # GPT disk with ESP partition
        -drive format=raw,file="$disk_image",if=ide,index=0

        # Boot configuration
        -boot order=c,menu=on
        -rtc base=utc
        -no-reboot
    )

    log_info "Starting QEMU with GPT ESP disk..."
    log_debug "Command: ${qemu_cmd[*]}"
    echo ""
    log_info "Expected: BdsDxe should detect GPT ESP and auto-boot"
    echo ""

    exec "${qemu_cmd[@]}"
}

# Analyze existing test environment
analyze_test_env() {
    log_info "Analyzing Test Environment"
    log_info "========================="

    local test_dir="$HELIX_ROOT/build/uefi_test"
    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    echo ""
    log_debug "EFI Kernel Analysis:"
    if [[ -f "$efi_kernel" ]]; then
        file "$efi_kernel" | sed 's/^/  /'
        ls -lh "$efi_kernel" | awk '{print "  Size: " $5}'
        echo "  PE Header:"
        hexdump -C "$efi_kernel" | head -2 | sed 's/^/    /'
    else
        log_error "  EFI kernel not found!"
    fi

    echo ""
    log_debug "Test Directory Structure:"
    if [[ -d "$test_dir" ]]; then
        find "$test_dir" -type f | sort | sed 's/^/  /'
    else
        echo "  Test directory does not exist"
    fi

    echo ""
    log_debug "OVMF Files Available:"
    find /usr/share/edk2 -name "*.fd" 2>/dev/null | head -5 | sed 's/^/  /'

    echo ""
    log_debug "System Requirements:"
    echo "  ✓ Loop device support: $(lsmod | grep loop >/dev/null && echo "Yes" || echo "No")"
    echo "  ✓ FAT filesystem: $(modinfo vfat >/dev/null 2>&1 && echo "Yes" || echo "No")"
    echo "  ✓ GPT tools (sgdisk): $(which sgdisk >/dev/null && echo "Yes" || echo "No")"
    echo "  ✓ Sudo access: $(sudo -n true 2>/dev/null && echo "Yes" || echo "Needs password")"
}

# Main function
main() {
    case "${1:-help}" in
        "esp"|"simple")
            create_esp_image
            test_esp_boot
            ;;
        "gpt"|"full")
            create_gpt_esp
            test_gpt_boot
            ;;
        "create-esp")
            create_esp_image
            ;;
        "create-gpt")
            create_gpt_esp
            ;;
        "test-esp")
            test_esp_boot
            ;;
        "test-gpt")
            test_gpt_boot
            ;;
        "analyze"|"info")
            analyze_test_env
            ;;
        "help"|"-h"|"--help")
            echo "UEFI ESP Boot Fixer v2.0"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  esp, simple    Create simple ESP image and test boot"
            echo "  gpt, full      Create GPT disk with ESP partition and test"
            echo "  create-esp     Create ESP image only"
            echo "  create-gpt     Create GPT disk only"
            echo "  test-esp       Test existing ESP image"
            echo "  test-gpt       Test existing GPT disk"
            echo "  analyze, info  Analyze test environment"
            echo "  help           Show this help"
            echo ""
            echo "Examples:"
            echo "  $0 esp         # Quick ESP test (recommended)"
            echo "  $0 gpt         # Full GPT ESP test"
            echo "  $0 analyze     # Check environment"
            ;;
        *)
            log_error "Unknown command: $1"
            log_info "Use '$0 help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
