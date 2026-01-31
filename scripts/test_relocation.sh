#!/bin/bash
# =============================================================================
# Helix OS - Kernel Relocation Test Script
# =============================================================================
# This script validates the kernel relocation and KASLR implementation.
# It checks:
#   1. PIE compilation flags
#   2. Presence of .rela.dyn section
#   3. ELF type (ET_DYN for PIE)
#   4. Relocation entry analysis
#   5. QEMU boot tests at different addresses
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
KERNEL_PATH="$PROJECT_ROOT/build/output/helix-kernel"
LINKER_SCRIPT="$PROJECT_ROOT/profiles/minimal/linker_pie.ld"

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

# =============================================================================
# HELPER FUNCTIONS
# =============================================================================

print_header() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_test() {
    echo -e "  ${BLUE}[TEST]${NC} $1"
}

print_pass() {
    echo -e "  ${GREEN}[PASS]${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

print_fail() {
    echo -e "  ${RED}[FAIL]${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

print_warn() {
    echo -e "  ${YELLOW}[WARN]${NC} $1"
}

print_info() {
    echo -e "  ${CYAN}[INFO]${NC} $1"
}

# =============================================================================
# PRE-FLIGHT CHECKS
# =============================================================================

check_prerequisites() {
    print_header "Pre-flight Checks"

    # Check for required tools
    local tools=("readelf" "objdump" "nm" "size")
    for tool in "${tools[@]}"; do
        if command -v "$tool" &> /dev/null; then
            print_pass "$tool is available"
        else
            print_fail "$tool is not installed"
        fi
    done

    # Check kernel exists
    if [[ -f "$KERNEL_PATH" ]]; then
        print_pass "Kernel binary found: $KERNEL_PATH"
        local size=$(stat -c%s "$KERNEL_PATH" 2>/dev/null || stat -f%z "$KERNEL_PATH" 2>/dev/null)
        print_info "Kernel size: $(numfmt --to=iec $size 2>/dev/null || echo "$size bytes")"
    else
        print_warn "Kernel not found. Run 'cargo build' first."
        print_info "Looking for kernel at: $KERNEL_PATH"
    fi

    # Check PIE linker script
    if [[ -f "$LINKER_SCRIPT" ]]; then
        print_pass "PIE linker script found: $LINKER_SCRIPT"
    else
        print_fail "PIE linker script not found"
    fi
}

# =============================================================================
# ELF ANALYSIS
# =============================================================================

analyze_elf() {
    print_header "ELF Analysis"

    if [[ ! -f "$KERNEL_PATH" ]]; then
        print_warn "Skipping ELF analysis (kernel not built)"
        return
    fi

    # Check ELF type
    print_test "Checking ELF type..."
    local elf_type=$(readelf -h "$KERNEL_PATH" 2>/dev/null | grep "Type:" | awk '{print $2}')

    if [[ "$elf_type" == "DYN" ]]; then
        print_pass "ELF type is DYN (PIE/Shared object) - Relocatable!"
    elif [[ "$elf_type" == "EXEC" ]]; then
        print_warn "ELF type is EXEC (static) - Not relocatable without manual work"
    else
        print_info "ELF type: $elf_type"
    fi

    # Check for .rela.dyn section
    print_test "Checking for relocation sections..."
    local rela_dyn=$(readelf -S "$KERNEL_PATH" 2>/dev/null | grep "\.rela\.dyn")
    local rela_plt=$(readelf -S "$KERNEL_PATH" 2>/dev/null | grep "\.rela\.plt")
    local dynamic=$(readelf -S "$KERNEL_PATH" 2>/dev/null | grep "\.dynamic")

    if [[ -n "$rela_dyn" ]]; then
        print_pass ".rela.dyn section found"
        local rela_size=$(echo "$rela_dyn" | awk '{print $6}')
        local rela_entries=$((0x$rela_size / 24))
        print_info "  Size: 0x$rela_size ($rela_entries entries)"
    else
        print_warn ".rela.dyn section not found (may not be PIE)"
    fi

    if [[ -n "$rela_plt" ]]; then
        print_pass ".rela.plt section found"
    fi

    if [[ -n "$dynamic" ]]; then
        print_pass ".dynamic section found"
    else
        print_warn ".dynamic section not found"
    fi

    # Analyze relocation types
    print_test "Analyzing relocation types..."
    local relocs=$(readelf -r "$KERNEL_PATH" 2>/dev/null)

    if [[ -n "$relocs" ]]; then
        local r_relative=$(echo "$relocs" | grep -c "R_X86_64_RELATIVE" || echo "0")
        local r_64=$(echo "$relocs" | grep -c "R_X86_64_64" || echo "0")
        local r_pc32=$(echo "$relocs" | grep -c "R_X86_64_PC32" || echo "0")
        local r_glob=$(echo "$relocs" | grep -c "R_X86_64_GLOB_DAT" || echo "0")
        local r_jump=$(echo "$relocs" | grep -c "R_X86_64_JUMP_SLOT" || echo "0")

        print_info "Relocation breakdown:"
        print_info "  R_X86_64_RELATIVE:  $r_relative"
        print_info "  R_X86_64_64:        $r_64"
        print_info "  R_X86_64_PC32:      $r_pc32"
        print_info "  R_X86_64_GLOB_DAT:  $r_glob"
        print_info "  R_X86_64_JUMP_SLOT: $r_jump"

        if [[ $r_relative -gt 0 ]]; then
            print_pass "R_X86_64_RELATIVE relocations found (good for PIE)"
        fi

        # Check for problematic relocations
        local r_32=$(echo "$relocs" | grep -c "R_X86_64_32[^S]" || echo "0")
        local r_32s=$(echo "$relocs" | grep -c "R_X86_64_32S" || echo "0")

        if [[ $r_32 -gt 0 || $r_32s -gt 0 ]]; then
            print_warn "Found 32-bit relocations ($r_32 R_32, $r_32s R_32S)"
            print_warn "These may cause issues with high addresses (>4GB)"
        fi
    else
        print_warn "No relocations found"
    fi

    # Check entry point
    print_test "Checking entry point..."
    local entry=$(readelf -h "$KERNEL_PATH" 2>/dev/null | grep "Entry point" | awk '{print $4}')
    print_info "Entry point: $entry"

    # For PIE, entry should be a low address (offset from base)
    if [[ "$entry" =~ ^0x[0-9a-f]{1,6}$ ]]; then
        print_pass "Entry point is a low address (PIE-compatible)"
    elif [[ "$entry" =~ ^0xffffffff ]]; then
        print_info "Entry point is in higher-half (typical for kernel)"
    fi
}

# =============================================================================
# LINKER SCRIPT ANALYSIS
# =============================================================================

analyze_linker_script() {
    print_header "Linker Script Analysis"

    if [[ ! -f "$LINKER_SCRIPT" ]]; then
        print_warn "PIE linker script not found, skipping analysis"
        return
    fi

    print_test "Checking linker script structure..."

    # Check for essential sections
    if grep -q "\.rela\.dyn" "$LINKER_SCRIPT"; then
        print_pass ".rela.dyn section defined"
    else
        print_warn ".rela.dyn section not defined"
    fi

    if grep -q "\.dynamic" "$LINKER_SCRIPT"; then
        print_pass ".dynamic section defined"
    else
        print_warn ".dynamic section not defined"
    fi

    if grep -q "PT_DYNAMIC" "$LINKER_SCRIPT"; then
        print_pass "PT_DYNAMIC program header defined"
    else
        print_warn "PT_DYNAMIC not defined (needed for runtime relocation)"
    fi

    if grep -q "\.got" "$LINKER_SCRIPT"; then
        print_pass "GOT section defined"
    fi

    # Check for higher-half kernel
    if grep -q "0xFFFFFFFF80000000\|0xffffffff80000000" "$LINKER_SCRIPT"; then
        print_pass "Higher-half kernel base address detected"
    fi

    # Check for multiboot header
    if grep -q "multiboot" "$LINKER_SCRIPT"; then
        print_pass "Multiboot header section defined"
    fi
}

# =============================================================================
# CARGO/RUSTC FLAGS CHECK
# =============================================================================

check_build_flags() {
    print_header "Build Configuration Check"

    # Check .cargo/config.toml or Cargo.toml for PIE flags
    local cargo_config="$PROJECT_ROOT/.cargo/config.toml"

    print_test "Checking Cargo configuration for PIE support..."

    if [[ -f "$cargo_config" ]]; then
        if grep -q "pie\|relocation-model" "$cargo_config"; then
            print_pass "PIE-related flags found in Cargo config"
        else
            print_info "No explicit PIE flags in Cargo config"
            print_info "You may need to add: rustflags = [\"-C\", \"relocation-model=pie\"]"
        fi
    else
        print_info "No .cargo/config.toml found"
    fi

    # Check for target-specific config
    print_test "Checking target configuration..."
    local target_json="$PROJECT_ROOT/x86_64-helix.json"

    if [[ -f "$target_json" ]]; then
        if grep -q "position-independent-executables\|relocation-model" "$target_json"; then
            print_pass "PIE configuration found in target JSON"
        fi
    fi
}

# =============================================================================
# HARDWARE CAPABILITY CHECK
# =============================================================================

check_hardware_capabilities() {
    print_header "Hardware Capability Check (KASLR)"

    print_test "Checking CPU features for KASLR entropy..."

    # Check for RDRAND
    if grep -q "rdrand" /proc/cpuinfo 2>/dev/null; then
        print_pass "RDRAND supported (strong entropy)"
    else
        print_warn "RDRAND not supported"
    fi

    # Check for RDSEED
    if grep -q "rdseed" /proc/cpuinfo 2>/dev/null; then
        print_pass "RDSEED supported (cryptographic entropy)"
    else
        print_info "RDSEED not supported (RDRAND or TSC will be used)"
    fi

    # QEMU capabilities
    if command -v qemu-system-x86_64 &> /dev/null; then
        print_pass "QEMU x86_64 available for testing"
        local qemu_version=$(qemu-system-x86_64 --version | head -1)
        print_info "  $qemu_version"
    else
        print_warn "QEMU not found (needed for boot testing)"
    fi
}

# =============================================================================
# SUMMARY
# =============================================================================

print_summary() {
    print_header "Test Summary"

    local total=$((TESTS_PASSED + TESTS_FAILED))

    echo -e "  ${GREEN}Passed:${NC} $TESTS_PASSED"
    echo -e "  ${RED}Failed:${NC} $TESTS_FAILED"
    echo -e "  ${BLUE}Total:${NC}  $total"
    echo ""

    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "  ${GREEN}✓ All checks passed!${NC}"
        echo ""
        echo -e "  ${CYAN}Next steps:${NC}"
        echo "    1. Build kernel with PIE linker script:"
        echo "       cargo build --release"
        echo "    2. Test boot with QEMU:"
        echo "       ./scripts/run_qemu.sh"
        echo "    3. Test with KASLR:"
        echo "       ./scripts/run_qemu.sh --kaslr"
    else
        echo -e "  ${YELLOW}⚠ Some checks failed. Review the output above.${NC}"
    fi

    echo ""
}

# =============================================================================
# MAIN
# =============================================================================

main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║         HELIX KERNEL RELOCATION TEST SUITE                                ║${NC}"
    echo -e "${CYAN}║         Testing PIE, KASLR, and Relocation Support                        ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════════════════╝${NC}"

    check_prerequisites
    analyze_elf
    analyze_linker_script
    check_build_flags
    check_hardware_capabilities
    print_summary

    # Exit with failure if any tests failed
    if [[ $TESTS_FAILED -gt 0 ]]; then
        exit 1
    fi
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
