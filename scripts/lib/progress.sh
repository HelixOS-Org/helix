#!/bin/bash
# =============================================================================
# Helix OS Framework - Progress Bar Library
# =============================================================================
# Provides beautiful progress bars for build steps
# =============================================================================

# Guard against multiple sourcing
[[ -n "${_HELIX_PROGRESS_LOADED:-}" ]] && return 0
_HELIX_PROGRESS_LOADED=1

_PROGRESS_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${_PROGRESS_SCRIPT_DIR}/colors.sh" 2>/dev/null || true

# =============================================================================
# Configuration
# =============================================================================

PROGRESS_BAR_WIDTH="${PROGRESS_BAR_WIDTH:-50}"
PROGRESS_FILL_CHAR="█"
PROGRESS_EMPTY_CHAR="░"
PROGRESS_EDGE_LEFT="["
PROGRESS_EDGE_RIGHT="]"

# =============================================================================
# Progress Bar Functions
# =============================================================================

# Draw a progress bar
# Usage: draw_progress_bar <current> <total> [description]
draw_progress_bar() {
    local current="$1"
    local total="$2"
    local description="${3:-}"
    
    # Calculate percentage
    local percent=0
    if [[ ${total} -gt 0 ]]; then
        percent=$(( (current * 100) / total ))
    fi
    
    # Calculate filled width
    local filled=$(( (current * PROGRESS_BAR_WIDTH) / total ))
    local empty=$(( PROGRESS_BAR_WIDTH - filled ))
    
    # Build the bar
    local bar=""
    for ((i=0; i<filled; i++)); do
        bar+="${PROGRESS_FILL_CHAR}"
    done
    for ((i=0; i<empty; i++)); do
        bar+="${PROGRESS_EMPTY_CHAR}"
    done
    
    # Choose color based on progress
    local color="${HELIX_INFO}"
    if [[ ${percent} -ge 100 ]]; then
        color="${HELIX_SUCCESS}"
    elif [[ ${percent} -ge 75 ]]; then
        color="${HELIX_PRIMARY}"
    elif [[ ${percent} -ge 50 ]]; then
        color="${CYAN}"
    elif [[ ${percent} -ge 25 ]]; then
        color="${YELLOW}"
    fi
    
    # Print the bar
    if [[ -n "${description}" ]]; then
        printf "\r  ${color}${PROGRESS_EDGE_LEFT}${bar}${PROGRESS_EDGE_RIGHT}${RESET} %3d%% ${DIM}%s${RESET}" \
            "${percent}" "${description}"
    else
        printf "\r  ${color}${PROGRESS_EDGE_LEFT}${bar}${PROGRESS_EDGE_RIGHT}${RESET} %3d%%" "${percent}"
    fi
}

# Complete the progress bar and move to new line
complete_progress_bar() {
    local message="${1:-Done}"
    printf "\r\033[K"  # Clear line
    echo -e "  ${HELIX_SUCCESS}${SYMBOL_CHECK}${RESET} ${message}"
}

# Fail the progress bar
fail_progress_bar() {
    local message="${1:-Failed}"
    printf "\r\033[K"  # Clear line
    echo -e "  ${HELIX_ERROR}${SYMBOL_CROSS}${RESET} ${message}"
}

# =============================================================================
# Multi-Step Progress Tracker
# =============================================================================

declare -A STEP_STATUS
declare -a STEP_NAMES
declare -a STEP_DESCRIPTIONS
CURRENT_STEP=0
TOTAL_STEPS=0

# Initialize step tracker
# Usage: init_steps "Step1:Description1" "Step2:Description2" ...
init_steps() {
    STEP_NAMES=()
    STEP_DESCRIPTIONS=()
    STEP_STATUS=()
    CURRENT_STEP=0
    TOTAL_STEPS=$#
    
    for step in "$@"; do
        local name="${step%%:*}"
        local desc="${step#*:}"
        STEP_NAMES+=("${name}")
        STEP_DESCRIPTIONS+=("${desc}")
        STEP_STATUS["${name}"]="pending"
    done
}

# Start a step
# Usage: start_step "StepName"
start_step() {
    local name="$1"
    STEP_STATUS["${name}"]="running"
    CURRENT_STEP=$((CURRENT_STEP + 1))
    
    local desc=""
    for i in "${!STEP_NAMES[@]}"; do
        if [[ "${STEP_NAMES[$i]}" == "${name}" ]]; then
            desc="${STEP_DESCRIPTIONS[$i]}"
            break
        fi
    done
    
    echo ""
    echo -e "${HELIX_PRIMARY}┌─────────────────────────────────────────────────────────────────────┐${RESET}"
    printf "${HELIX_PRIMARY}│${RESET} ${BOLD}Step %d/%d:${RESET} %-54s ${HELIX_PRIMARY}│${RESET}\n" \
        "${CURRENT_STEP}" "${TOTAL_STEPS}" "${desc}"
    echo -e "${HELIX_PRIMARY}└─────────────────────────────────────────────────────────────────────┘${RESET}"
}

# Complete a step
# Usage: complete_step "StepName" [duration_seconds]
complete_step() {
    local name="$1"
    local duration="${2:-}"
    STEP_STATUS["${name}"]="completed"
    
    local time_str=""
    if [[ -n "${duration}" ]]; then
        time_str=" (${duration}s)"
    fi
    
    echo -e "${HELIX_SUCCESS}${SYMBOL_CHECK} Step completed${time_str}${RESET}"
}

# Fail a step
# Usage: fail_step "StepName" [error_message]
fail_step() {
    local name="$1"
    local error="${2:-Unknown error}"
    STEP_STATUS["${name}"]="failed"
    
    echo -e "${HELIX_ERROR}${SYMBOL_CROSS} Step failed: ${error}${RESET}"
}

# Skip a step
# Usage: skip_step "StepName" [reason]
skip_step() {
    local name="$1"
    local reason="${2:-Already up to date}"
    STEP_STATUS["${name}"]="skipped"
    
    echo -e "${HELIX_WARNING}${SYMBOL_ARROW} Step skipped: ${reason}${RESET}"
}

# Print summary of all steps
print_step_summary() {
    echo ""
    print_header "Build Summary"
    
    local completed=0
    local failed=0
    local skipped=0
    
    for name in "${STEP_NAMES[@]}"; do
        local status="${STEP_STATUS[${name}]}"
        local symbol=""
        local color=""
        
        case "${status}" in
            completed)
                symbol="${SYMBOL_CHECK}"
                color="${HELIX_SUCCESS}"
                completed=$((completed + 1))
                ;;
            failed)
                symbol="${SYMBOL_CROSS}"
                color="${HELIX_ERROR}"
                failed=$((failed + 1))
                ;;
            skipped)
                symbol="${SYMBOL_ARROW}"
                color="${HELIX_WARNING}"
                skipped=$((skipped + 1))
                ;;
            *)
                symbol="${SYMBOL_BULLET}"
                color="${DIM}"
                ;;
        esac
        
        echo -e "  ${color}${symbol}${RESET} ${name}"
    done
    
    echo ""
    print_separator
    echo -e "  ${HELIX_SUCCESS}Completed:${RESET} ${completed}  ${HELIX_ERROR}Failed:${RESET} ${failed}  ${HELIX_WARNING}Skipped:${RESET} ${skipped}"
    
    if [[ ${failed} -gt 0 ]]; then
        return 1
    fi
    return 0
}

# =============================================================================
# Animated Progress for Long Operations
# =============================================================================

# Progress animation characters
ANIM_CHARS=("⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏")
ANIM_INDEX=0

# Animate progress
animate_progress() {
    local message="$1"
    local current="$2"
    local total="$3"
    
    local char="${ANIM_CHARS[$ANIM_INDEX]}"
    ANIM_INDEX=$(( (ANIM_INDEX + 1) % ${#ANIM_CHARS[@]} ))
    
    local percent=0
    if [[ ${total} -gt 0 ]]; then
        percent=$(( (current * 100) / total ))
    fi
    
    printf "\r  ${HELIX_PRIMARY}${char}${RESET} ${message} [%3d%%]" "${percent}"
}

# =============================================================================
# File Progress Tracking
# =============================================================================

# Count files to process
count_files() {
    local pattern="$1"
    local dir="${2:-.}"
    find "${dir}" -name "${pattern}" -type f 2>/dev/null | wc -l
}

# Process files with progress
# Usage: process_files_with_progress "*.rs" "Compiling" <command>
process_files_with_progress() {
    local pattern="$1"
    local action="$2"
    shift 2
    local command="$@"
    
    local files
    mapfile -t files < <(find . -name "${pattern}" -type f 2>/dev/null)
    local total=${#files[@]}
    local current=0
    
    for file in "${files[@]}"; do
        current=$((current + 1))
        draw_progress_bar "${current}" "${total}" "${action}: $(basename "${file}")"
        
        # Run command
        if ! eval "${command} \"${file}\"" >/dev/null 2>&1; then
            fail_progress_bar "Failed: ${file}"
            return 1
        fi
    done
    
    complete_progress_bar "${action} complete (${total} files)"
    return 0
}
