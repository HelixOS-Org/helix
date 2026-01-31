#!/bin/bash
#
# Helix OS - UEFI Boot via Limine ISO
# Lance l'ISO Limine existant avec OVMF

set -euo pipefail

HELIX_ROOT="$(dirname "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)")"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; CYAN='\033[0;36m'; NC='\033[0m'
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

main() {
    local iso_path="$HELIX_ROOT/build/output/helix-limine.iso"
    local ovmf_code="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
    local ovmf_vars_template="/usr/share/edk2/x64/OVMF_VARS.4m.fd"
    local ovmf_vars="$HELIX_ROOT/build/output/OVMF_VARS_LIMINE.fd"

    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║        Helix OS - UEFI Boot via Limine                  ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Check ISO exists
    if [[ ! -f "$iso_path" ]]; then
        log_warn "ISO not found, building..."
        "$HELIX_ROOT/scripts/run_limine.sh" --iso-only
    fi

    log_info "Using ISO: $iso_path"

    # Check OVMF
    if [[ ! -f "$ovmf_code" ]]; then
        log_error "OVMF not found. Install: sudo pacman -S edk2-ovmf"
        exit 1
    fi

    # Copy OVMF vars if needed
    if [[ ! -f "$ovmf_vars" ]]; then
        cp "$ovmf_vars_template" "$ovmf_vars"
        log_info "Created OVMF variables"
    fi

    # Create logs dir
    mkdir -p "$HELIX_ROOT/build/logs"

    log_info "Starting QEMU with UEFI firmware..."
    echo ""
    log_warn "🎯 Expected: Limine bootloader → Helix OS"
    echo ""

    exec qemu-system-x86_64 \
        -enable-kvm \
        -m 512M \
        -smp 2 \
        -display gtk,grab-on-hover=on \
        -vga std \
        -drive if=pflash,format=raw,readonly=on,file="$ovmf_code" \
        -drive if=pflash,format=raw,file="$ovmf_vars" \
        -cdrom "$iso_path" \
        -boot d \
        -serial file:"$HELIX_ROOT/build/logs/serial.log" \
        -debugcon file:"$HELIX_ROOT/build/logs/debug.log" \
        -global isa-debugcon.iobase=0x402 \
        -no-reboot
}

main "$@"
