#!/bin/bash
#
# Pure UEFI Boot Test Script - Bypasses GRUB completely
# Tests direct UEFI kernel execution

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

# Test pure UEFI boot without GRUB
test_pure_uefi_boot() {
    log_info "Testing Pure UEFI Boot (No GRUB)"
    log_info "=================================="

    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        log_info "Run: ./scripts/convert_to_efi.sh"
        exit 1
    fi

    # Verify EFI format
    local file_info
    file_info=$(file "$efi_kernel")
    log_debug "EFI file: $file_info"

    if ! echo "$file_info" | grep -q "PE32+"; then
        log_error "Not a valid PE32+ EFI file"
        exit 1
    fi

    log_info "✓ Valid PE32+ EFI kernel found"
    echo ""

    # Create minimal UEFI test environment
    local test_dir="$HELIX_ROOT/build/uefi_test"
    rm -rf "$test_dir"
    mkdir -p "$test_dir/EFI/BOOT"

    # Copy EFI kernel directly to ESP location
    cp "$efi_kernel" "$test_dir/EFI/BOOT/"

    # Create minimal startup script
    cat > "$test_dir/startup.nsh" << 'EOF'
@echo off
cls
echo ================================================
echo Helix OS - Pure UEFI Boot Test
echo ================================================
echo Loading kernel directly...
\EFI\BOOT\BOOTX64.EFI
EOF

    # Copy OVMF variables if they exist
    local ovmf_vars="$HELIX_ROOT/build/output/OVMF_VARS.4m.fd"
    if [[ -f "$ovmf_vars" ]]; then
        cp "$ovmf_vars" "$test_dir/"
        log_info "✓ Using existing OVMF variables"
    else
        # Create fresh OVMF variables
        if [[ -f /usr/share/edk2/x64/OVMF_VARS.4m.fd ]]; then
            cp /usr/share/edk2/x64/OVMF_VARS.4m.fd "$test_dir/"
            log_info "✓ Created fresh OVMF variables"
        fi
    fi

    log_info "Test environment created at: $test_dir"
    echo ""

    # Build QEMU command for pure UEFI boot
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

        # FAT filesystem as EFI System Partition
        -drive format=raw,file=fat:rw:"$test_dir"

        # Boot options
        -boot menu=on,strict=on
        -rtc base=utc
        -no-reboot
    )

    log_info "Starting QEMU with pure UEFI boot..."
    log_debug "Command: ${qemu_cmd[*]}"
    echo ""
    log_warn "Expected behavior:"
    log_warn "  1. UEFI shell should start"
    log_warn "  2. Run 'startup.nsh' or manually: \\EFI\\BOOT\\BOOTX64.EFI"
    log_warn "  3. Kernel should load without page fault"
    echo ""
    log_info "Press Ctrl+C to exit QEMU"
    echo ""

    # Execute QEMU
    exec "${qemu_cmd[@]}"
}

# Test EFI file directly
test_efi_file() {
    log_info "EFI File Analysis"
    log_info "=================="

    local efi_kernel="$HELIX_ROOT/build/output/BOOTX64.EFI"

    if [[ ! -f "$efi_kernel" ]]; then
        log_error "EFI kernel not found: $efi_kernel"
        exit 1
    fi

    # Basic file info
    echo "File info:"
    file "$efi_kernel" | sed 's/^/  /'
    echo ""

    # File size
    echo "File size:"
    ls -lh "$efi_kernel" | awk '{print "  Size: " $5}'
    echo ""

    # PE header analysis
    echo "PE header:"
    hexdump -C "$efi_kernel" | head -4 | sed 's/^/  /'
    echo ""

    # Check DOS signature
    local dos_sig
    dos_sig=$(head -c 2 "$efi_kernel" | hexdump -v -e '"%02x"')

    if [[ "$dos_sig" == "4d5a" ]]; then
        log_info "✓ Valid DOS signature (MZ) found"
    else
        log_warn "⚠ Invalid DOS signature: $dos_sig"
    fi
    echo ""

    # Try to extract PE info
    if command -v objdump >/dev/null 2>&1; then
        echo "PE sections:"
        objdump -h "$efi_kernel" 2>/dev/null | grep -E "^\s+[0-9]+" | sed 's/^/  /' || true
        echo ""

        echo "Entry point:"
        objdump -f "$efi_kernel" 2>/dev/null | grep "start address" | sed 's/^/  /' || true
        echo ""
    fi
}

# Main function
main() {
    case "${1:-test}" in
        "test"|"boot")
            test_pure_uefi_boot
            ;;
        "analyze"|"info")
            test_efi_file
            ;;
        "help"|"-h"|"--help")
            echo "Pure UEFI Test Script"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  test, boot    Test pure UEFI boot (default)"
            echo "  analyze, info Analyze EFI file"
            echo "  help          Show this help"
            ;;
        *)
            log_error "Unknown command: $1"
            log_info "Use '$0 help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
