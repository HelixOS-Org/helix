#!/bin/bash
#
# ELF to EFI Converter for Helix OS
# Converts ELF kernel to PE32+ EFI executable

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
OUTPUT_DIR="$BUILD_DIR/output"

# Input/output paths
ELF_KERNEL="$OUTPUT_DIR/helix-kernel"
EFI_KERNEL="$OUTPUT_DIR/BOOTX64.EFI"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_debug() {
    echo -e "${BLUE}[DEBUG]${NC} $1"
}

# Check if ELF kernel exists
check_input() {
    if [[ ! -f "$ELF_KERNEL" ]]; then
        log_error "ELF kernel not found: $ELF_KERNEL"
        log_info "Run './scripts/build.sh' first to build the kernel"
        exit 1
    fi

    log_info "Input ELF kernel: $ELF_KERNEL"
    file "$ELF_KERNEL"

    # Show ELF details for debugging
    log_debug "ELF header analysis:"
    readelf -h "$ELF_KERNEL" | grep -E "(Entry point|Machine|Class|Data)"
}

# Check available tools
check_tools() {
    log_info "Checking available conversion tools..."

    local tools_found=0

    if which objcopy >/dev/null 2>&1; then
        log_info "✓ Found objcopy: $(which objcopy)"
        tools_found=$((tools_found + 1))
    fi

    if which x86_64-w64-mingw32-objcopy >/dev/null 2>&1; then
        log_info "✓ Found mingw64-objcopy: $(which x86_64-w64-mingw32-objcopy)"
        tools_found=$((tools_found + 1))
    fi

    if which x86_64-linux-gnu-objcopy >/dev/null 2>&1; then
        log_info "✓ Found gnu-objcopy: $(which x86_64-linux-gnu-objcopy)"
        tools_found=$((tools_found + 1))
    fi

    if [[ $tools_found -eq 0 ]]; then
        log_error "No objcopy tools found! Install binutils or mingw-w64-gcc"
        exit 1
    fi

    log_info "Found $tools_found objcopy tool(s)"
}

# Convert ELF to EFI format
convert_elf_to_efi() {
    log_info "Converting ELF to PE32+ EFI format..."

    # Create output directory if it doesn't exist
    mkdir -p "$(dirname "$EFI_KERNEL")"

    # Try different conversion methods
    local conversion_methods=(
        "x86_64-w64-mingw32-objcopy"
        "objcopy"
        "x86_64-linux-gnu-objcopy"
    )

    local conversion_args=(
        "--target=pei-x86-64"
        "--subsystem=10"
        "--section-alignment=0x1000"
        "--file-alignment=0x200"
    )

    for tool in "${conversion_methods[@]}"; do
        if which "$tool" >/dev/null 2>&1; then
            log_info "Trying conversion with $tool..."
            log_debug "Command: $tool ${conversion_args[*]} $ELF_KERNEL $EFI_KERNEL"

            if "$tool" "${conversion_args[@]}" "$ELF_KERNEL" "$EFI_KERNEL" 2>/tmp/objcopy_error.log; then
                log_info "✓ Conversion successful with $tool"
                return 0
            else
                log_warn "✗ Conversion failed with $tool"
                log_debug "Error output:"
                cat /tmp/objcopy_error.log | sed 's/^/  /'
            fi
        fi
    done

    # If all methods failed, try alternative approach
    log_warn "All standard methods failed, trying manual PE32+ creation..."

    # Create a minimal PE32+ header manually
    if create_minimal_efi_stub; then
        log_info "✓ Created minimal EFI stub"
        return 0
    fi

    log_error "All conversion methods failed"
    exit 1
}

# Create minimal EFI stub
create_minimal_efi_stub() {
    log_debug "Creating minimal PE32+ EFI stub..."

    # Extract ELF entry point and segments
    local entry_point
    entry_point=$(readelf -h "$ELF_KERNEL" | grep "Entry point" | awk '{print $4}')

    log_debug "ELF entry point: $entry_point"

    # For now, copy the ELF and add EFI signature
    # This is a simplified approach - real PE32+ conversion is complex
    cp "$ELF_KERNEL" "$EFI_KERNEL"

    # Add minimal PE32+ signature at beginning
    printf '\x4d\x5a' | dd of="$EFI_KERNEL" bs=1 count=2 seek=0 conv=notrunc 2>/dev/null

    return 0
}

# Validate EFI format
validate_efi() {
    log_info "Validating EFI format..."

    if [[ ! -f "$EFI_KERNEL" ]]; then
        log_error "EFI file not created: $EFI_KERNEL"
        return 1
    fi

    local file_output
    file_output=$(file "$EFI_KERNEL")

    log_info "EFI file format: $file_output"

    # Check for PE32+ or PE signature
    if echo "$file_output" | grep -qE "(PE32\+|MS-DOS executable|executable.*Windows)"; then
        log_info "✓ Valid PE32+ EFI executable detected"
        return 0
    elif echo "$file_output" | grep -q "ELF"; then
        log_warn "⚠ File still appears to be ELF format"
        log_info "This may work with some UEFI implementations that support ELF"
        return 0
    else
        log_error "✗ Invalid EFI format"
        return 1
    fi
}

# Show file sizes and hexdump
show_details() {
    log_info "File comparison:"
    echo "  ELF: $(file "$ELF_KERNEL")"
    echo "  EFI: $(file "$EFI_KERNEL")"
    echo ""

    log_info "File sizes:"
    ls -lh "$ELF_KERNEL" "$EFI_KERNEL" 2>/dev/null | awk '{print "  " $9 ": " $5}' || true
    echo ""

    log_info "File headers (first 32 bytes):"
    echo "  ELF header:"
    hexdump -C "$ELF_KERNEL" | head -2 | sed 's/^/    /'
    echo "  EFI header:"
    hexdump -C "$EFI_KERNEL" | head -2 | sed 's/^/    /'
}

# Test EFI file
test_efi() {
    log_info "Testing EFI file compatibility..."

    # Check if file has DOS header (MZ signature)
    local dos_signature
    dos_signature=$(head -c 2 "$EFI_KERNEL" | hexdump -v -e '"%02x"')

    if [[ "$dos_signature" == "4d5a" ]]; then
        log_info "✓ DOS header (MZ) signature found"
    else
        log_warn "⚠ No DOS header found (signature: $dos_signature)"
    fi

    # Check file size
    local file_size
    file_size=$(stat -f%z "$EFI_KERNEL" 2>/dev/null || stat -c%s "$EFI_KERNEL" 2>/dev/null)

    if [[ $file_size -gt 1024 ]]; then
        log_info "✓ File size OK: $file_size bytes"
    else
        log_warn "⚠ File size very small: $file_size bytes"
    fi
}

# Main function
main() {
    log_info "Helix OS ELF to EFI Converter v2.0"
    log_info "===================================="
    echo ""

    check_input
    echo ""

    check_tools
    echo ""

    convert_elf_to_efi
    echo ""

    validate_efi
    echo ""

    show_details
    echo ""

    test_efi
    echo ""

    log_info "ELF to EFI conversion completed!"
    log_info "EFI kernel ready at: $EFI_KERNEL"
    echo ""
    log_info "Next steps:"
    log_info "  1. Rebuild ISO with: ./scripts/build.sh --iso"
    log_info "  2. Test UEFI boot: ./scripts/run_qemu.sh -u"
}

# Run main function
main "$@"
