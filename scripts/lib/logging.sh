#!/bin/bash
# =============================================================================
# Helix OS Framework - Logging Library
# =============================================================================
# Provides consistent logging for all build scripts
# =============================================================================

# Guard against multiple sourcing
[[ -n "${_HELIX_LOGGING_LOADED:-}" ]] && return 0
_HELIX_LOGGING_LOADED=1

# Source colors if not already loaded
_LOGGING_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${_LOGGING_SCRIPT_DIR}/colors.sh" 2>/dev/null || true

# =============================================================================
# Configuration
# =============================================================================

export HELIX_LOG_LEVEL="${HELIX_LOG_LEVEL:-INFO}"
export HELIX_LOG_FILE="${HELIX_LOG_FILE:-}"
export HELIX_LOG_TIMESTAMP="${HELIX_LOG_TIMESTAMP:-1}"

# Log level numbers
declare -A LOG_LEVELS=(
    ["DEBUG"]=0
    ["INFO"]=1
    ["WARN"]=2
    ["ERROR"]=3
    ["FATAL"]=4
    ["SILENT"]=5
)

# =============================================================================
# Internal Functions
# =============================================================================

_get_timestamp() {
    if [[ "${HELIX_LOG_TIMESTAMP}" == "1" ]]; then
        echo "[$(date '+%H:%M:%S')]"
    fi
}

_should_log() {
    local level="$1"
    local current_level="${LOG_LEVELS[${HELIX_LOG_LEVEL}]:-1}"
    local msg_level="${LOG_LEVELS[${level}]:-1}"
    [[ ${msg_level} -ge ${current_level} ]]
}

_write_log() {
    local message="$1"
    echo -e "${message}"
    if [[ -n "${HELIX_LOG_FILE}" ]]; then
        echo -e "${message}" | sed 's/\x1b\[[0-9;]*m//g' >> "${HELIX_LOG_FILE}"
    fi
}

# =============================================================================
# Public Logging Functions
# =============================================================================

log_debug() {
    if _should_log "DEBUG"; then
        _write_log "${HELIX_DEBUG}$(_get_timestamp) [DEBUG] $*${RESET}"
    fi
}

log_info() {
    if _should_log "INFO"; then
        _write_log "${HELIX_INFO}$(_get_timestamp) ${SYMBOL_INFO} ${RESET}$*"
    fi
}

log_success() {
    if _should_log "INFO"; then
        _write_log "${HELIX_SUCCESS}$(_get_timestamp) ${SYMBOL_CHECK} $*${RESET}"
    fi
}

log_warn() {
    if _should_log "WARN"; then
        _write_log "${HELIX_WARNING}$(_get_timestamp) ${SYMBOL_WARNING} WARNING: $*${RESET}"
    fi
}

log_error() {
    if _should_log "ERROR"; then
        _write_log "${HELIX_ERROR}$(_get_timestamp) ${SYMBOL_CROSS} ERROR: $*${RESET}" >&2
    fi
}

log_fatal() {
    if _should_log "FATAL"; then
        _write_log "${BG_RED}${BOLD_WHITE}$(_get_timestamp) ${SYMBOL_CROSS} FATAL: $*${RESET}" >&2
    fi
}

# =============================================================================
# Styled Output Functions
# =============================================================================

print_header() {
    local title="$1"
    local width="${2:-70}"
    local line=$(printf '═%.0s' $(seq 1 $width))
    
    echo ""
    echo -e "${HELIX_PRIMARY}╔${line}╗${RESET}"
    printf "${HELIX_PRIMARY}║${RESET} ${BOLD_WHITE}%-$((width-2))s${RESET} ${HELIX_PRIMARY}║${RESET}\n" "${title}"
    echo -e "${HELIX_PRIMARY}╚${line}╝${RESET}"
    echo ""
}

print_subheader() {
    local title="$1"
    echo ""
    echo -e "${HELIX_SECONDARY}━━━ ${BOLD}${title}${RESET} ${HELIX_SECONDARY}━━━${RESET}"
    echo ""
}

print_step() {
    local step_num="$1"
    local total="$2"
    local description="$3"
    
    echo -e "${HELIX_ACCENT}[${step_num}/${total}]${RESET} ${HELIX_PRIMARY}${SYMBOL_ARROW}${RESET} ${description}"
}

print_bullet() {
    echo -e "  ${HELIX_SECONDARY}${SYMBOL_BULLET}${RESET} $*"
}

print_action() {
    local action="$1"
    local target="$2"
    printf "  ${HELIX_INFO}%-12s${RESET} %s\n" "${action}" "${target}"
}

print_keyvalue() {
    local key="$1"
    local value="$2"
    local width="${3:-20}"
    printf "  ${DIM}%-${width}s${RESET} ${BOLD}%s${RESET}\n" "${key}:" "${value}"
}

# =============================================================================
# Box Drawing Functions
# =============================================================================

print_box() {
    local title="$1"
    local content="$2"
    local color="${3:-${HELIX_PRIMARY}}"
    local width=60
    
    local line=$(printf '─%.0s' $(seq 1 $((width-2))))
    
    echo -e "${color}┌${line}┐${RESET}"
    printf "${color}│${RESET} ${BOLD}%-$((width-4))s${RESET} ${color}│${RESET}\n" "${title}"
    echo -e "${color}├${line}┤${RESET}"
    
    while IFS= read -r line_content; do
        printf "${color}│${RESET} %-$((width-4))s ${color}│${RESET}\n" "${line_content}"
    done <<< "${content}"
    
    echo -e "${color}└${line}┘${RESET}"
}

print_separator() {
    local char="${1:-─}"
    local width="${2:-70}"
    local color="${3:-${DIM}}"
    echo -e "${color}$(printf "${char}%.0s" $(seq 1 $width))${RESET}"
}

# =============================================================================
# Spinner and Progress
# =============================================================================

SPINNER_PID=""
SPINNER_CHARS="⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
SPINNER_DELAY=0.1

_spinner() {
    local message="$1"
    local i=0
    local len=${#SPINNER_CHARS}
    
    while true; do
        local char="${SPINNER_CHARS:$i:1}"
        printf "\r${HELIX_PRIMARY}${char}${RESET} ${message}"
        i=$(( (i + 1) % len ))
        sleep ${SPINNER_DELAY}
    done
}

start_spinner() {
    local message="$1"
    if [[ -z "${HELIX_NO_COLOR:-}" ]] && [[ -t 1 ]]; then
        _spinner "${message}" &
        SPINNER_PID=$!
        disown
    else
        echo -e "${HELIX_INFO}${SYMBOL_GEAR}${RESET} ${message}"
    fi
}

stop_spinner() {
    local status="$1"  # success, error, warn
    local message="$2"
    
    if [[ -n "${SPINNER_PID}" ]]; then
        kill "${SPINNER_PID}" 2>/dev/null
        wait "${SPINNER_PID}" 2>/dev/null
        SPINNER_PID=""
        printf "\r\033[K"  # Clear line
    fi
    
    case "${status}" in
        success)
            echo -e "${HELIX_SUCCESS}${SYMBOL_CHECK}${RESET} ${message}"
            ;;
        error)
            echo -e "${HELIX_ERROR}${SYMBOL_CROSS}${RESET} ${message}"
            ;;
        warn)
            echo -e "${HELIX_WARNING}${SYMBOL_WARNING}${RESET} ${message}"
            ;;
        *)
            echo -e "${HELIX_INFO}${SYMBOL_INFO}${RESET} ${message}"
            ;;
    esac
}

# =============================================================================
# Trap for cleanup
# =============================================================================

_cleanup_spinner() {
    if [[ -n "${SPINNER_PID:-}" ]]; then
        kill "${SPINNER_PID}" 2>/dev/null
        wait "${SPINNER_PID}" 2>/dev/null
        printf "\r\033[K"
    fi
}

trap _cleanup_spinner EXIT
