#!/bin/bash
# =============================================================================
# Helix OS Framework - Flash Utility
# =============================================================================
# Flash kernel to USB drive or virtual disk
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"
OUTPUT_DIR="${HELIX_ROOT}/build/output"

# =============================================================================
# Functions
# =============================================================================

create_disk_image() {
    local output="$1"
    local size="${2:-256M}"
    
    log_info "Creating disk image: ${output} (${size})"
    
    # Create empty image
    dd if=/dev/zero of="${output}" bs=1M count=${size%M} status=progress 2>/dev/null
    
    # Create partition table
    if cmd_exists parted; then
        parted -s "${output}" mklabel msdos
        parted -s "${output}" mkpart primary fat32 1MiB 100%
        parted -s "${output}" set 1 boot on
        log_success "Partition table created"
    elif cmd_exists fdisk; then
        echo -e "o\nn\np\n1\n\n\na\nw" | fdisk "${output}" >/dev/null 2>&1
        log_success "Partition table created (fdisk)"
    else
        log_warn "No partitioning tool found, creating raw image"
    fi
    
    log_success "Disk image created: ${output}"
}

flash_to_device() {
    local source="$1"
    local device="$2"
    
    # Safety checks
    if [[ ! -b "${device}" ]]; then
        log_error "Not a block device: ${device}"
        exit 1
    fi
    
    # Check if mounted
    if mount | grep -q "${device}"; then
        log_error "Device is mounted: ${device}"
        log_info "Unmount it first with: sudo umount ${device}*"
        exit 1
    fi
    
    # Confirm
    echo ""
    echo -e "${HELIX_WARNING}${SYMBOL_WARNING} WARNING: This will ERASE ALL DATA on ${device}${RESET}"
    echo ""
    
    read -p "Type 'YES' to continue: " confirm
    if [[ "${confirm}" != "YES" ]]; then
        log_info "Aborted"
        exit 0
    fi
    
    log_info "Flashing ${source} to ${device}..."
    
    if cmd_exists pv; then
        sudo pv "${source}" | sudo dd of="${device}" bs=4M conv=fsync
    else
        sudo dd if="${source}" of="${device}" bs=4M conv=fsync status=progress
    fi
    
    sync
    log_success "Flash complete!"
}

create_bootable_usb() {
    local kernel="${OUTPUT_DIR}/helix-kernel"
    local device="$1"
    local mount_point="/tmp/helix_usb_$$"
    
    if [[ ! -f "${kernel}" ]]; then
        log_error "Kernel not found: ${kernel}"
        log_info "Run './scripts/build.sh all' first"
        exit 1
    fi
    
    # Unmount if mounted
    sudo umount "${device}"* 2>/dev/null || true
    
    # Create partition
    log_info "Creating partition table..."
    sudo parted -s "${device}" mklabel msdos
    sudo parted -s "${device}" mkpart primary fat32 1MiB 100%
    sudo parted -s "${device}" set 1 boot on
    
    sleep 1  # Wait for kernel to recognize partition
    
    # Format
    local partition="${device}1"
    if [[ ! -b "${partition}" ]]; then
        partition="${device}p1"  # For /dev/mmcblk0 style
    fi
    
    log_info "Formatting ${partition}..."
    sudo mkfs.fat -F 32 "${partition}"
    
    # Mount
    mkdir -p "${mount_point}"
    sudo mount "${partition}" "${mount_point}"
    
    # Install bootloader
    log_info "Installing bootloader..."
    
    if cmd_exists grub-install; then
        sudo mkdir -p "${mount_point}/boot/grub"
        sudo cp "${kernel}" "${mount_point}/boot/"
        
        cat << 'EOF' | sudo tee "${mount_point}/boot/grub/grub.cfg" >/dev/null
set timeout=3
set default=0

menuentry "Helix OS" {
    multiboot2 /boot/helix-kernel
    boot
}
EOF
        
        sudo grub-install --target=i386-pc --boot-directory="${mount_point}/boot" "${device}"
    else
        log_warn "GRUB not found, copying kernel only"
        sudo cp "${kernel}" "${mount_point}/"
    fi
    
    # Cleanup
    sudo umount "${mount_point}"
    rmdir "${mount_point}"
    
    log_success "Bootable USB created!"
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
    echo -e "${DIM}    Flash Utility${RESET}"
    echo ""
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 <command> [args]"
    echo ""
    
    echo -e "${BOLD}Commands:${RESET}"
    echo "  image <output> [size]   Create a disk image"
    echo "  usb <device>            Create bootable USB"
    echo "  flash <source> <device> Flash image to device"
    echo "  help                    Show this help"
    echo ""
    
    echo -e "${BOLD}Examples:${RESET}"
    echo "  $0 image helix.img 256M"
    echo "  $0 usb /dev/sdb"
    echo "  $0 flash helix.iso /dev/sdb"
    echo ""
    
    echo -e "${HELIX_WARNING}WARNING: Flashing will ERASE ALL DATA on the target device!${RESET}"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    local command="${1:-help}"
    shift || true
    
    case "${command}" in
        image)
            local output="${1:-helix.img}"
            local size="${2:-256M}"
            create_disk_image "${output}" "${size}"
            ;;
        usb)
            local device="${1:-}"
            if [[ -z "${device}" ]]; then
                log_error "Device required"
                echo "Available devices:"
                lsblk -d -o NAME,SIZE,MODEL | grep -v "loop"
                exit 1
            fi
            create_bootable_usb "${device}"
            ;;
        flash)
            local source="${1:-}"
            local device="${2:-}"
            if [[ -z "${source}" ]] || [[ -z "${device}" ]]; then
                log_error "Usage: $0 flash <source> <device>"
                exit 1
            fi
            flash_to_device "${source}" "${device}"
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            log_error "Unknown command: ${command}"
            print_usage
            exit 1
            ;;
    esac
}

# Check for root if needed
if [[ "${1:-}" =~ ^(usb|flash)$ ]] && [[ $EUID -ne 0 ]]; then
    log_warn "Flash operations may require sudo"
fi

main "$@"
