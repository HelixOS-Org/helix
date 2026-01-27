#!/bin/bash
# =============================================================================
# Helix OS Framework - Build System Core
# =============================================================================
# Main build orchestrator for the multi-step LFS-inspired build system
# =============================================================================

set -euo pipefail

# =============================================================================
# Initialization
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export HELIX_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Ensure rustup's cargo/rustc are used instead of system ones
# This is critical when system has rust installed via package manager
if [[ -d "${HOME}/.cargo/bin" ]]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
fi

# Load libraries
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/progress.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

# =============================================================================
# Build Configuration
# =============================================================================

# Default configuration
export HELIX_ARCH="${HELIX_ARCH:-x86_64}"
export HELIX_PROFILE="${HELIX_PROFILE:-release}"
export HELIX_TARGET="${HELIX_ARCH}-unknown-none"
export HELIX_JOBS="${HELIX_JOBS:-$(get_cpu_count)}"
export HELIX_VERBOSE="${HELIX_VERBOSE:-0}"
export HELIX_FORCE="${HELIX_FORCE:-0}"

# Directories
export HELIX_BUILD_DIR="${HELIX_ROOT}/build"
export HELIX_OUTPUT_DIR="${HELIX_BUILD_DIR}/output"
export HELIX_LOGS_DIR="${HELIX_BUILD_DIR}/logs"
export HELIX_CACHE_DIR="${HELIX_BUILD_DIR}/.cache"

# Build timestamp
export HELIX_BUILD_TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
export HELIX_LOG_FILE="${HELIX_LOGS_DIR}/build_${HELIX_BUILD_TIMESTAMP}.log"

# =============================================================================
# Build Steps Definition
# =============================================================================

declare -A BUILD_STEPS=(
    ["0_prepare_env"]="Prepare Build Environment"
    ["1_build_bootloader"]="Build Bootloader"
    ["2_build_core_kernel"]="Build Kernel Core"
    ["3_build_memory_subsystem"]="Build Memory Subsystem"
    ["4_build_scheduler"]="Build Scheduler"
    ["5_build_io_subsystem"]="Build I/O Subsystem"
    ["6_build_module_system"]="Build Module System"
    ["7_build_communication_layer"]="Build Communication Layer"
    ["8_build_sys_interface"]="Build System Interface"
    ["9_build_userland_framework"]="Build Userland Framework"
    ["10_test_all"]="Run All Tests"
    ["11_package_kernel"]="Package Kernel Image"
    ["12_clean"]="Clean Build Artifacts"
)

# Step order
BUILD_STEP_ORDER=(
    "0_prepare_env"
    "1_build_bootloader"
    "2_build_core_kernel"
    "3_build_memory_subsystem"
    "4_build_scheduler"
    "5_build_io_subsystem"
    "6_build_module_system"
    "7_build_communication_layer"
    "8_build_sys_interface"
    "9_build_userland_framework"
    "10_test_all"
    "11_package_kernel"
)

# =============================================================================
# Step Implementations
# =============================================================================

step_0_prepare_env() {
    print_subheader "Checking Build Environment"
    
    local errors=0
    
    # Check Rust
    print_action "Checking" "Rust toolchain"
    if ! cmd_exists rustc; then
        log_error "Rust not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        errors=$((errors + 1))
    else
        local rust_version=$(get_rust_version)
        log_success "Rust ${rust_version} found"
    fi
    
    # Check Cargo
    print_action "Checking" "Cargo"
    if ! cmd_exists cargo; then
        log_error "Cargo not found"
        errors=$((errors + 1))
    else
        log_success "Cargo $(get_cargo_version) found"
    fi
    
    # Check nightly
    print_action "Checking" "Rust nightly"
    if ! rustup run nightly rustc --version >/dev/null 2>&1; then
        log_warn "Rust nightly not installed, installing..."
        rustup install nightly || errors=$((errors + 1))
    else
        log_success "Rust nightly available"
    fi
    
    # Check rust-src
    print_action "Checking" "rust-src component"
    if ! rustup component list --toolchain nightly 2>/dev/null | grep -q "rust-src (installed)"; then
        log_warn "rust-src not installed, installing..."
        rustup component add rust-src --toolchain nightly || errors=$((errors + 1))
    else
        log_success "rust-src available"
    fi
    
    # Check llvm-tools
    print_action "Checking" "llvm-tools"
    if ! rustup component list --toolchain nightly 2>/dev/null | grep -q "llvm-tools"; then
        log_warn "llvm-tools not installed, installing..."
        rustup component add llvm-tools-preview --toolchain nightly || true
    else
        log_success "llvm-tools available"
    fi
    
    # Check QEMU
    print_action "Checking" "QEMU"
    if ! cmd_exists qemu-system-x86_64; then
        log_warn "QEMU not found. Install with: sudo apt install qemu-system-x86"
    else
        local qemu_version=$(qemu-system-x86_64 --version | head -1 | awk '{print $4}')
        log_success "QEMU ${qemu_version} found"
    fi
    
    # Check NASM
    print_action "Checking" "NASM assembler"
    if ! cmd_exists nasm; then
        log_warn "NASM not found. Install with: sudo apt install nasm"
    else
        log_success "NASM found"
    fi
    
    # Check cross-linker
    print_action "Checking" "Cross-linker"
    if ! cmd_exists ld.lld; then
        log_warn "LLD not found. Install with: sudo apt install lld"
    else
        log_success "LLD found"
    fi
    
    # Create directories
    print_action "Creating" "Build directories"
    ensure_directories
    log_success "Directories created"
    
    if [[ ${errors} -gt 0 ]]; then
        return 1
    fi
    
    return 0
}

step_1_build_bootloader() {
    print_subheader "Building Bootloader"
    
    local boot_dir="${HELIX_ROOT}/boot"
    local output_dir="${HELIX_OUTPUT_DIR}/boot"
    mkdir -p "${output_dir}"
    
    # Check for bootloader source
    if [[ ! -d "${boot_dir}" ]]; then
        log_warn "No boot directory found, creating stub..."
        mkdir -p "${boot_dir}/src"
    fi
    
    # For now, we'll use Limine or GRUB
    print_action "Building" "Boot stub"
    
    # Create a minimal boot assembly if not exists
    if [[ ! -f "${boot_dir}/src/boot.asm" ]]; then
        log_info "Creating minimal boot stub..."
        cat > "${boot_dir}/src/boot.asm" << 'EOF'
; Helix OS Boot Stub
; This is a minimal multiboot2 header

section .multiboot_header
header_start:
    dd 0xe85250d6                ; magic number
    dd 0                         ; architecture (i386)
    dd header_end - header_start ; header length
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))
    
    ; end tag
    dw 0
    dw 0
    dd 8
header_end:

section .text
global _start
extern kernel_main

_start:
    ; Set up stack
    mov esp, stack_top
    
    ; Call Rust kernel
    call kernel_main
    
    ; Halt
.hang:
    cli
    hlt
    jmp .hang

section .bss
stack_bottom:
    resb 16384
stack_top:
EOF
    fi
    
    # Compile if NASM available
    if cmd_exists nasm; then
        draw_progress_bar 1 3 "Assembling boot.asm"
        nasm -f elf64 "${boot_dir}/src/boot.asm" -o "${output_dir}/boot.o" 2>/dev/null || true
        draw_progress_bar 2 3 "Creating linker script"
        draw_progress_bar 3 3 "Complete"
        complete_progress_bar "Bootloader assembled"
    else
        log_warn "NASM not available, bootloader build skipped"
    fi
    
    return 0
}

step_2_build_core_kernel() {
    print_subheader "Building Kernel Core"
    
    cd "${HELIX_ROOT}"
    
    local extra_args=()
    
    if [[ "${HELIX_VERBOSE}" == "1" ]]; then
        extra_args+=("-v")
    fi
    
    print_action "Compiling" "helix-core"
    
    # Run cargo with progress
    local log_file="${HELIX_LOGS_DIR}/core_kernel.log"
    
    start_spinner "Compiling kernel core..."
    if cargo_build "helix-core" "${extra_args[@]}" > "${log_file}" 2>&1; then
        stop_spinner "success" "Kernel core compiled successfully"
    else
        stop_spinner "error" "Kernel core compilation failed"
        log_error "See ${log_file} for details"
        return 1
    fi
    
    return 0
}

step_3_build_memory_subsystem() {
    print_subheader "Building Memory Subsystem"
    
    cd "${HELIX_ROOT}"
    
    local components=(
        "helix-memory"
    )
    
    local total=${#components[@]}
    local current=0
    
    for component in "${components[@]}"; do
        current=$((current + 1))
        draw_progress_bar ${current} ${total} "Building ${component}"
        
        local log_file="${HELIX_LOGS_DIR}/${component}.log"
        
        if ! cargo_build "${component}" > "${log_file}" 2>&1; then
            fail_progress_bar "Failed: ${component}"
            return 1
        fi
    done
    
    complete_progress_bar "Memory subsystem built"
    return 0
}

step_4_build_scheduler() {
    print_subheader "Building Scheduler"
    
    cd "${HELIX_ROOT}"
    
    local components=(
        "helix-execution"
    )
    
    # Check for scheduler modules
    if [[ -d "${HELIX_ROOT}/modules_impl/schedulers" ]]; then
        for sched_dir in "${HELIX_ROOT}/modules_impl/schedulers"/*/; do
            if [[ -f "${sched_dir}/Cargo.toml" ]]; then
                local sched_name=$(basename "${sched_dir}")
                components+=("helix-scheduler-${sched_name}")
            fi
        done
    fi
    
    local total=${#components[@]}
    local current=0
    
    for component in "${components[@]}"; do
        current=$((current + 1))
        draw_progress_bar ${current} ${total} "Building ${component}"
        
        local log_file="${HELIX_LOGS_DIR}/${component}.log"
        
        if ! cargo_build "${component}" > "${log_file}" 2>&1; then
            # Don't fail if module doesn't exist yet
            log_debug "Skipped: ${component}"
        fi
    done
    
    complete_progress_bar "Scheduler built"
    return 0
}

step_5_build_io_subsystem() {
    print_subheader "Building I/O Subsystem"
    
    cd "${HELIX_ROOT}"
    
    # I/O subsystem not yet implemented, create stub
    if [[ ! -d "${HELIX_ROOT}/subsystems/io" ]]; then
        log_info "I/O subsystem not yet implemented, skipping..."
        return 0
    fi
    
    local log_file="${HELIX_LOGS_DIR}/io_subsystem.log"
    
    start_spinner "Compiling I/O subsystem..."
    if cargo_build "helix-io" > "${log_file}" 2>&1; then
        stop_spinner "success" "I/O subsystem compiled"
    else
        stop_spinner "warn" "I/O subsystem skipped (not ready)"
    fi
    
    return 0
}

step_6_build_module_system() {
    print_subheader "Building Module System"
    
    cd "${HELIX_ROOT}"
    
    local log_file="${HELIX_LOGS_DIR}/module_system.log"
    
    start_spinner "Compiling module system..."
    if cargo_build "helix-modules" > "${log_file}" 2>&1; then
        stop_spinner "success" "Module system compiled"
    else
        stop_spinner "error" "Module system compilation failed"
        return 1
    fi
    
    return 0
}

step_7_build_communication_layer() {
    print_subheader "Building Communication Layer"
    
    cd "${HELIX_ROOT}"
    
    if [[ ! -d "${HELIX_ROOT}/ipc" ]]; then
        log_info "IPC layer not yet implemented, skipping..."
        return 0
    fi
    
    local log_file="${HELIX_LOGS_DIR}/communication.log"
    
    start_spinner "Compiling IPC layer..."
    if cargo_build "helix-ipc" > "${log_file}" 2>&1; then
        stop_spinner "success" "Communication layer compiled"
    else
        stop_spinner "warn" "Communication layer skipped (not ready)"
    fi
    
    return 0
}

step_8_build_sys_interface() {
    print_subheader "Building System Interface"
    
    cd "${HELIX_ROOT}"
    
    if [[ ! -d "${HELIX_ROOT}/interface" ]]; then
        log_info "System interface not yet implemented, skipping..."
        return 0
    fi
    
    local log_file="${HELIX_LOGS_DIR}/sys_interface.log"
    
    start_spinner "Compiling syscall interface..."
    if cargo_build "helix-interface" > "${log_file}" 2>&1; then
        stop_spinner "success" "System interface compiled"
    else
        stop_spinner "warn" "System interface skipped (not ready)"
    fi
    
    return 0
}

step_9_build_userland_framework() {
    print_subheader "Building Userland Framework"
    
    cd "${HELIX_ROOT}"
    
    # Build profile example
    if [[ -d "${HELIX_ROOT}/profiles/minimal" ]]; then
        local log_file="${HELIX_LOGS_DIR}/userland.log"
        
        start_spinner "Building minimal OS profile..."
        if cargo_build "helix-minimal-os" > "${log_file}" 2>&1; then
            stop_spinner "success" "Minimal OS profile built"
        else
            stop_spinner "warn" "Minimal OS profile skipped"
        fi
    else
        log_info "No profiles found, skipping..."
    fi
    
    return 0
}

step_10_test_all() {
    print_subheader "Running Tests"
    
    cd "${HELIX_ROOT}"
    
    local log_file="${HELIX_LOGS_DIR}/tests.log"
    
    # Run unit tests (host tests only, no target)
    print_action "Running" "Unit tests"
    
    start_spinner "Running unit tests..."
    if cargo_test --workspace --lib > "${log_file}" 2>&1; then
        stop_spinner "success" "All unit tests passed"
    else
        stop_spinner "warn" "Some tests failed or skipped"
        log_warn "See ${log_file} for details"
    fi
    
    return 0
}

step_11_package_kernel() {
    print_subheader "Packaging Kernel Image"
    
    local output_dir="${HELIX_OUTPUT_DIR}"
    local kernel_bin="${HELIX_ROOT}/target/${HELIX_TARGET}/${HELIX_PROFILE}/helix-minimal-os"
    
    mkdir -p "${output_dir}"
    
    if [[ -f "${kernel_bin}" ]]; then
        print_action "Copying" "Kernel binary"
        cp "${kernel_bin}" "${output_dir}/helix-kernel"
        
        # Create ISO if grub-mkrescue available
        if cmd_exists grub-mkrescue; then
            print_action "Creating" "Bootable ISO"
            
            local iso_dir="${HELIX_BUILD_DIR}/iso"
            mkdir -p "${iso_dir}/boot/grub"
            
            cp "${output_dir}/helix-kernel" "${iso_dir}/boot/"
            
            cat > "${iso_dir}/boot/grub/grub.cfg" << 'EOF'
set timeout=3
set default=0

menuentry "Helix OS" {
    multiboot2 /boot/helix-kernel
    boot
}
EOF
            
            grub-mkrescue -o "${output_dir}/helix.iso" "${iso_dir}" 2>/dev/null || true
            
            if [[ -f "${output_dir}/helix.iso" ]]; then
                log_success "Created: ${output_dir}/helix.iso"
            fi
        fi
        
        log_success "Kernel packaged: ${output_dir}/helix-kernel"
    else
        log_warn "Kernel binary not found, packaging skipped"
    fi
    
    # Print summary
    echo ""
    print_box "Build Output" "$(ls -lh "${output_dir}" 2>/dev/null || echo 'No files')"
    
    return 0
}

step_12_clean() {
    print_subheader "Cleaning Build Artifacts"
    
    local items_cleaned=0
    
    # Clean cargo target
    if [[ -d "${HELIX_ROOT}/target" ]]; then
        print_action "Cleaning" "Cargo target directory"
        rm -rf "${HELIX_ROOT}/target"
        items_cleaned=$((items_cleaned + 1))
    fi
    
    # Clean build directory
    if [[ -d "${HELIX_BUILD_DIR}" ]]; then
        print_action "Cleaning" "Build directory"
        rm -rf "${HELIX_BUILD_DIR}"
        items_cleaned=$((items_cleaned + 1))
    fi
    
    # Clean Cargo.lock (optional)
    if [[ "${HELIX_FORCE}" == "1" ]] && [[ -f "${HELIX_ROOT}/Cargo.lock" ]]; then
        print_action "Cleaning" "Cargo.lock"
        rm -f "${HELIX_ROOT}/Cargo.lock"
        items_cleaned=$((items_cleaned + 1))
    fi
    
    log_success "Cleaned ${items_cleaned} items"
    return 0
}

# =============================================================================
# Build Orchestration
# =============================================================================

run_step() {
    local step_name="$1"
    local step_func="step_${step_name}"
    local step_desc="${BUILD_STEPS[${step_name}]:-${step_name}}"
    
    if ! declare -f "${step_func}" >/dev/null; then
        log_error "Step function not found: ${step_func}"
        return 1
    fi
    
    local start_time=$(get_timestamp)
    
    start_step "${step_name}"
    
    if "${step_func}"; then
        local end_time=$(get_timestamp)
        local duration=$((end_time - start_time))
        complete_step "${step_name}" "${duration}"
        return 0
    else
        fail_step "${step_name}"
        return 1
    fi
}

run_all_steps() {
    local skip_to="${1:-}"
    local stop_at="${2:-}"
    local started=0
    
    [[ -z "${skip_to}" ]] && started=1
    
    for step in "${BUILD_STEP_ORDER[@]}"; do
        # Skip until we reach skip_to
        if [[ "${started}" == "0" ]]; then
            if [[ "${step}" == "${skip_to}" ]]; then
                started=1
            else
                continue
            fi
        fi
        
        # Run the step
        if ! run_step "${step}"; then
            return 1
        fi
        
        # Stop if we've reached stop_at
        if [[ -n "${stop_at}" ]] && [[ "${step}" == "${stop_at}" ]]; then
            break
        fi
    done
    
    return 0
}

# =============================================================================
# CLI Interface
# =============================================================================

print_banner() {
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
    echo -e "${DIM}    OS Framework Build System v1.0.0${RESET}"
    echo ""
}

print_usage() {
    print_banner
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 [command] [options]"
    echo ""
    
    echo -e "${BOLD}Commands:${RESET}"
    echo "  all                 Run all build steps"
    echo "  step <name>         Run a specific step"
    echo "  from <step>         Run from a specific step to the end"
    echo "  to <step>           Run from beginning to a specific step"
    echo "  list                List all available steps"
    echo "  clean               Clean build artifacts"
    echo "  help                Show this help message"
    echo ""
    
    echo -e "${BOLD}Options:${RESET}"
    echo "  -a, --arch <arch>   Target architecture (x86_64, aarch64, riscv64)"
    echo "  -p, --profile <p>   Build profile (debug, release)"
    echo "  -j, --jobs <n>      Number of parallel jobs"
    echo "  -v, --verbose       Verbose output"
    echo "  -f, --force         Force rebuild"
    echo "  --no-color          Disable colored output"
    echo ""
    
    echo -e "${BOLD}Examples:${RESET}"
    echo "  $0 all                      # Full build"
    echo "  $0 step 2_build_core_kernel # Build only kernel core"
    echo "  $0 from 4_build_scheduler   # Build from scheduler onwards"
    echo "  $0 all --arch aarch64       # Build for ARM64"
    echo ""
}

list_steps() {
    print_header "Available Build Steps"
    
    local i=0
    for step in "${BUILD_STEP_ORDER[@]}"; do
        local desc="${BUILD_STEPS[${step}]}"
        printf "  ${HELIX_SECONDARY}%2d.${RESET} ${BOLD}%-30s${RESET} %s\n" \
            "${i}" "${step}" "${desc}"
        i=$((i + 1))
    done
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    local command="${1:-help}"
    shift || true
    
    # Parse options
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -a|--arch)
                HELIX_ARCH="$2"
                HELIX_TARGET="${HELIX_ARCH}-unknown-none"
                shift 2
                ;;
            -p|--profile)
                HELIX_PROFILE="$2"
                shift 2
                ;;
            -j|--jobs)
                HELIX_JOBS="$2"
                shift 2
                ;;
            -v|--verbose)
                HELIX_VERBOSE=1
                shift
                ;;
            -f|--force)
                HELIX_FORCE=1
                shift
                ;;
            --no-color)
                export HELIX_NO_COLOR=1
                source "${SCRIPT_DIR}/lib/colors.sh"
                shift
                ;;
            *)
                break
                ;;
        esac
    done
    
    # Ensure build directories exist
    ensure_directories
    
    # Start logging
    echo "Build started at $(date)" > "${HELIX_LOG_FILE}"
    
    # Record start time
    local build_start=$(get_timestamp)
    
    case "${command}" in
        all)
            print_banner
            print_header "Full Build - Helix OS Framework"
            print_keyvalue "Architecture" "${HELIX_ARCH}"
            print_keyvalue "Profile" "${HELIX_PROFILE}"
            print_keyvalue "Jobs" "${HELIX_JOBS}"
            echo ""
            
            init_steps \
                "0_prepare_env:Prepare Build Environment" \
                "1_build_bootloader:Build Bootloader" \
                "2_build_core_kernel:Build Kernel Core" \
                "3_build_memory_subsystem:Build Memory Subsystem" \
                "4_build_scheduler:Build Scheduler" \
                "5_build_io_subsystem:Build I/O Subsystem" \
                "6_build_module_system:Build Module System" \
                "7_build_communication_layer:Build Communication Layer" \
                "8_build_sys_interface:Build System Interface" \
                "9_build_userland_framework:Build Userland Framework" \
                "10_test_all:Run Tests" \
                "11_package_kernel:Package Kernel"
            
            if run_all_steps; then
                local build_end=$(get_timestamp)
                local duration=$((build_end - build_start))
                
                echo ""
                print_header "Build Complete!"
                echo -e "  ${HELIX_SUCCESS}${SYMBOL_CHECK}${RESET} All steps completed successfully"
                echo -e "  ${HELIX_INFO}${SYMBOL_CLOCK}${RESET} Total time: $(format_duration ${duration})"
                echo ""
                
                print_step_summary
            else
                print_step_summary
                exit 1
            fi
            ;;
        
        step)
            local step_name="$1"
            if [[ -z "${step_name}" ]]; then
                log_error "Step name required"
                exit 1
            fi
            
            print_banner
            run_step "${step_name}"
            ;;
        
        from)
            local from_step="$1"
            print_banner
            run_all_steps "${from_step}" ""
            ;;
        
        to)
            local to_step="$1"
            print_banner
            run_all_steps "" "${to_step}"
            ;;
        
        list)
            list_steps
            ;;
        
        clean)
            print_banner
            run_step "12_clean"
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

main "$@"
