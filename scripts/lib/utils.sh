#!/bin/bash
# =============================================================================
# Helix OS Framework - Utility Functions
# =============================================================================
# Common utilities for build scripts
# =============================================================================

# Guard against multiple sourcing
[[ -n "${_HELIX_UTILS_LOADED:-}" ]] && return 0
_HELIX_UTILS_LOADED=1

_UTILS_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${_UTILS_SCRIPT_DIR}/colors.sh" 2>/dev/null || true
source "${_UTILS_SCRIPT_DIR}/logging.sh" 2>/dev/null || true

# =============================================================================
# Path Utilities
# =============================================================================

# Get the project root directory
get_project_root() {
    local dir="${SCRIPT_DIR}"
    while [[ "${dir}" != "/" ]]; do
        if [[ -f "${dir}/Cargo.toml" ]] && [[ -d "${dir}/scripts" ]]; then
            echo "${dir}"
            return 0
        fi
        dir="$(dirname "${dir}")"
    done
    echo ""
    return 1
}

export HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"
export HELIX_BUILD_DIR="${HELIX_ROOT}/build"
export HELIX_TARGET_DIR="${HELIX_ROOT}/target"
export HELIX_CACHE_DIR="${HELIX_BUILD_DIR}/.cache"
export HELIX_LOG_DIR="${HELIX_BUILD_DIR}/logs"

# Ensure directories exist
ensure_directories() {
    mkdir -p "${HELIX_BUILD_DIR}"
    mkdir -p "${HELIX_TARGET_DIR}"
    mkdir -p "${HELIX_CACHE_DIR}"
    mkdir -p "${HELIX_LOG_DIR}"
}

# =============================================================================
# Command Utilities
# =============================================================================

# Check if a command exists
cmd_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Run command with logging
run_cmd() {
    local cmd="$*"
    log_debug "Running: ${cmd}"

    local output
    local exit_code

    output=$("$@" 2>&1)
    exit_code=$?

    if [[ ${exit_code} -ne 0 ]]; then
        log_error "Command failed: ${cmd}"
        log_error "Output: ${output}"
        return ${exit_code}
    fi

    log_debug "Output: ${output}"
    return 0
}

# =============================================================================
# Cargo/Rust Utilities
# =============================================================================

# Get the toolchain from rust-toolchain.toml
get_toolchain_from_file() {
    local toolchain_file="${HELIX_ROOT}/rust-toolchain.toml"
    if [[ -f "${toolchain_file}" ]]; then
        # Extract channel from rust-toolchain.toml
        grep -oP 'channel\s*=\s*"\K[^"]+' "${toolchain_file}" 2>/dev/null || echo ""
    fi
}

# Setup rustup environment to override system rust
setup_rustup_env() {
    if command -v rustup &>/dev/null; then
        local rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
        local cargo_home="${CARGO_HOME:-$HOME/.cargo}"

        # Get toolchain
        local toolchain
        toolchain=$(get_toolchain_from_file)
        [[ -z "${toolchain}" ]] && toolchain="nightly"

        # Set environment variables
        export RUSTUP_TOOLCHAIN="${toolchain}"

        # Prepend cargo bin to PATH to override system rust
        if [[ -d "${cargo_home}/bin" ]]; then
            export PATH="${cargo_home}/bin:${PATH}"
        fi
    fi
}

# Initialize rustup environment
setup_rustup_env

# Get the cargo command with proper toolchain handling
# Always use rustup run to guarantee correct toolchain
get_cargo_cmd() {
    local toolchain="${1:-}"

    # Try to get toolchain from rust-toolchain.toml
    if [[ -z "${toolchain}" ]]; then
        toolchain=$(get_toolchain_from_file)
    fi

    # If we have rustup and a toolchain, use rustup run
    if command -v rustup &>/dev/null && [[ -n "${toolchain}" ]]; then
        echo "rustup run ${toolchain} cargo"
        return 0
    fi

    # Fallback to cargo directly (may not work with system cargo)
    if command -v cargo &>/dev/null; then
        echo "cargo"
        return 0
    fi

    log_error "Neither rustup nor cargo found!"
    return 1
}

# Run cargo with proper toolchain handling
# Usage: run_cargo build -p helix-core --target x86_64-unknown-none
run_cargo() {
    local cargo_cmd
    cargo_cmd=$(get_cargo_cmd)

    log_debug "Running: ${cargo_cmd} $*"
    ${cargo_cmd} "$@"
}

# Run cargo build with standard options
# Usage: cargo_build <package> [extra_args...]
cargo_build() {
    local package="$1"
    shift

    local cargo_cmd
    cargo_cmd=$(get_cargo_cmd)

    local args=("build" "-p" "${package}")

    # Add target if HELIX_TARGET is set
    if [[ -n "${HELIX_TARGET:-}" ]]; then
        args+=("--target" "${HELIX_TARGET}")
    fi

    # Add release flag if HELIX_PROFILE is release
    if [[ "${HELIX_PROFILE:-debug}" == "release" ]]; then
        args+=("--release")
    fi

    # Add any extra arguments
    args+=("$@")

    log_debug "Running: ${cargo_cmd} ${args[*]}"
    ${cargo_cmd} "${args[@]}"
}

# Run cargo test with proper toolchain handling
# Tests must run on host target (not bare metal) to have access to std/test
cargo_test() {
    local cargo_cmd
    cargo_cmd=$(get_cargo_cmd)
    # Override the default bare metal target to run tests on host
    ${cargo_cmd} test --target x86_64-unknown-linux-gnu "$@"
}

# Run cargo clippy with proper toolchain handling
cargo_clippy() {
    local cargo_cmd
    cargo_cmd=$(get_cargo_cmd)
    ${cargo_cmd} clippy "$@"
}

# Run cargo fmt with proper toolchain handling
cargo_fmt() {
    local cargo_cmd
    cargo_cmd=$(get_cargo_cmd)
    ${cargo_cmd} fmt "$@"
}

# Run command silently (only show errors)
run_silent() {
    "$@" >/dev/null 2>&1
}

# Run command and capture output
capture_output() {
    local -n _result=$1
    shift
    _result=$("$@" 2>&1)
}

# =============================================================================
# Version Utilities
# =============================================================================

# Compare versions (returns 0 if $1 >= $2)
version_gte() {
    local v1="$1"
    local v2="$2"

    printf '%s\n%s' "${v2}" "${v1}" | sort -V -C
}

# Get Rust version
get_rust_version() {
    rustc --version 2>/dev/null | awk '{print $2}'
}

# Get cargo version
get_cargo_version() {
    cargo --version 2>/dev/null | awk '{print $2}'
}

# =============================================================================
# Time Utilities
# =============================================================================

# Get current timestamp in seconds
get_timestamp() {
    date +%s
}

# Format seconds as human-readable duration
format_duration() {
    local seconds="$1"

    if [[ ${seconds} -lt 60 ]]; then
        echo "${seconds}s"
    elif [[ ${seconds} -lt 3600 ]]; then
        local mins=$((seconds / 60))
        local secs=$((seconds % 60))
        echo "${mins}m ${secs}s"
    else
        local hours=$((seconds / 3600))
        local mins=$(( (seconds % 3600) / 60 ))
        echo "${hours}h ${mins}m"
    fi
}

# Time a command
time_cmd() {
    local start=$(get_timestamp)
    "$@"
    local exit_code=$?
    local end=$(get_timestamp)
    local duration=$((end - start))

    echo "${duration}"
    return ${exit_code}
}

# =============================================================================
# File Utilities
# =============================================================================

# Check if file is newer than another
is_newer() {
    local file1="$1"
    local file2="$2"

    [[ ! -f "${file2}" ]] || [[ "${file1}" -nt "${file2}" ]]
}

# Get file hash (for caching)
get_file_hash() {
    local file="$1"
    sha256sum "${file}" 2>/dev/null | awk '{print $1}'
}

# Check if rebuild is needed
needs_rebuild() {
    local source="$1"
    local target="$2"
    local cache_file="${HELIX_CACHE_DIR}/$(basename "${source}").hash"

    # Target doesn't exist
    [[ ! -f "${target}" ]] && return 0

    # No cache file
    [[ ! -f "${cache_file}" ]] && return 0

    # Hash changed
    local current_hash=$(get_file_hash "${source}")
    local cached_hash=$(cat "${cache_file}" 2>/dev/null)

    [[ "${current_hash}" != "${cached_hash}" ]]
}

# Update rebuild cache
update_rebuild_cache() {
    local source="$1"
    local cache_file="${HELIX_CACHE_DIR}/$(basename "${source}").hash"

    mkdir -p "$(dirname "${cache_file}")"
    get_file_hash "${source}" > "${cache_file}"
}

# =============================================================================
# Process Utilities
# =============================================================================

# Get number of CPU cores
get_cpu_count() {
    nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4
}

# Check if process is running
is_running() {
    local pid="$1"
    kill -0 "${pid}" 2>/dev/null
}

# =============================================================================
# User Interaction
# =============================================================================

# Ask yes/no question
ask_yes_no() {
    local question="$1"
    local default="${2:-n}"

    local prompt
    if [[ "${default}" == "y" ]]; then
        prompt="[Y/n]"
    else
        prompt="[y/N]"
    fi

    read -p "${question} ${prompt} " answer
    answer="${answer:-${default}}"

    [[ "${answer,,}" == "y" || "${answer,,}" == "yes" ]]
}

# Select from options
select_option() {
    local prompt="$1"
    shift
    local options=("$@")

    echo "${prompt}"
    for i in "${!options[@]}"; do
        echo "  $((i+1))) ${options[$i]}"
    done

    read -p "Select [1-${#options[@]}]: " choice

    if [[ "${choice}" -ge 1 && "${choice}" -le ${#options[@]} ]]; then
        echo "${options[$((choice-1))]}"
        return 0
    fi

    return 1
}

# =============================================================================
# Error Handling
# =============================================================================

# Exit with error
die() {
    log_fatal "$*"
    exit 1
}

# Assert condition
assert() {
    local condition="$1"
    local message="${2:-Assertion failed}"

    if ! eval "${condition}"; then
        die "${message}"
    fi
}

# Cleanup on exit
declare -a CLEANUP_TASKS

add_cleanup() {
    CLEANUP_TASKS+=("$*")
}

run_cleanup() {
    for task in "${CLEANUP_TASKS[@]}"; do
        eval "${task}" 2>/dev/null || true
    done
}

trap run_cleanup EXIT

# =============================================================================
# Lock Files (for parallel builds)
# =============================================================================

acquire_lock() {
    local lockfile="$1"
    local timeout="${2:-60}"

    local count=0
    while [[ -f "${lockfile}" ]]; do
        if [[ ${count} -ge ${timeout} ]]; then
            log_error "Timeout waiting for lock: ${lockfile}"
            return 1
        fi
        sleep 1
        count=$((count + 1))
    done

    echo $$ > "${lockfile}"
    add_cleanup "rm -f '${lockfile}'"
    return 0
}

release_lock() {
    local lockfile="$1"
    rm -f "${lockfile}"
}
