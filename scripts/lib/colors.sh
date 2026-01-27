#!/bin/bash
# =============================================================================
# Helix OS Framework - Color Library
# =============================================================================
# Provides consistent color output for all build scripts
# =============================================================================

# Guard against multiple sourcing
[[ -n "${_HELIX_COLORS_LOADED:-}" ]] && return 0
_HELIX_COLORS_LOADED=1

# Disable colors if not in a terminal or if NO_COLOR is set
if [[ ! -t 1 ]] || [[ -n "${NO_COLOR:-}" ]]; then
    export HELIX_NO_COLOR=1
fi

# =============================================================================
# Color Definitions
# =============================================================================

if [[ -z "${HELIX_NO_COLOR:-}" ]]; then
    # Regular Colors
    export BLACK='\033[0;30m'
    export RED='\033[0;31m'
    export GREEN='\033[0;32m'
    export YELLOW='\033[0;33m'
    export BLUE='\033[0;34m'
    export PURPLE='\033[0;35m'
    export CYAN='\033[0;36m'
    export WHITE='\033[0;37m'

    # Bold Colors
    export BOLD_BLACK='\033[1;30m'
    export BOLD_RED='\033[1;31m'
    export BOLD_GREEN='\033[1;32m'
    export BOLD_YELLOW='\033[1;33m'
    export BOLD_BLUE='\033[1;34m'
    export BOLD_PURPLE='\033[1;35m'
    export BOLD_CYAN='\033[1;36m'
    export BOLD_WHITE='\033[1;37m'

    # Background Colors
    export BG_BLACK='\033[40m'
    export BG_RED='\033[41m'
    export BG_GREEN='\033[42m'
    export BG_YELLOW='\033[43m'
    export BG_BLUE='\033[44m'
    export BG_PURPLE='\033[45m'
    export BG_CYAN='\033[46m'
    export BG_WHITE='\033[47m'

    # Special
    export RESET='\033[0m'
    export BOLD='\033[1m'
    export DIM='\033[2m'
    export ITALIC='\033[3m'
    export UNDERLINE='\033[4m'
    export BLINK='\033[5m'
    export REVERSE='\033[7m'
    export HIDDEN='\033[8m'
else
    # No colors
    export BLACK='' RED='' GREEN='' YELLOW='' BLUE='' PURPLE='' CYAN='' WHITE=''
    export BOLD_BLACK='' BOLD_RED='' BOLD_GREEN='' BOLD_YELLOW=''
    export BOLD_BLUE='' BOLD_PURPLE='' BOLD_CYAN='' BOLD_WHITE=''
    export BG_BLACK='' BG_RED='' BG_GREEN='' BG_YELLOW=''
    export BG_BLUE='' BG_PURPLE='' BG_CYAN='' BG_WHITE=''
    export RESET='' BOLD='' DIM='' ITALIC='' UNDERLINE='' BLINK='' REVERSE='' HIDDEN=''
fi

# =============================================================================
# Helix Theme Colors
# =============================================================================

export HELIX_PRIMARY="${BOLD_CYAN}"
export HELIX_SECONDARY="${BOLD_PURPLE}"
export HELIX_SUCCESS="${BOLD_GREEN}"
export HELIX_WARNING="${BOLD_YELLOW}"
export HELIX_ERROR="${BOLD_RED}"
export HELIX_INFO="${BOLD_BLUE}"
export HELIX_DEBUG="${DIM}${WHITE}"
export HELIX_ACCENT="${BOLD_WHITE}"

# =============================================================================
# Unicode Symbols
# =============================================================================

export SYMBOL_CHECK="✓"
export SYMBOL_CROSS="✗"
export SYMBOL_ARROW="→"
export SYMBOL_BULLET="•"
export SYMBOL_STAR="★"
export SYMBOL_WARNING="⚠"
export SYMBOL_INFO="ℹ"
export SYMBOL_GEAR="⚙"
export SYMBOL_ROCKET="🚀"
export SYMBOL_PACKAGE="📦"
export SYMBOL_WRENCH="🔧"
export SYMBOL_HAMMER="🔨"
export SYMBOL_LIGHTNING="⚡"
export SYMBOL_DNA="🧬"
export SYMBOL_KERNEL="🔲"
export SYMBOL_MODULE="🔌"
export SYMBOL_TEST="🧪"
export SYMBOL_CLEAN="🧹"
export SYMBOL_CLOCK="⏱"
export SYMBOL_PROGRESS="▓"
export SYMBOL_EMPTY="░"

# Fallback for non-unicode terminals
if [[ "${TERM:-}" == "linux" ]] || [[ -z "${LANG:-}" ]] || [[ "${LANG}" != *"UTF"* ]]; then
    export SYMBOL_CHECK="[OK]"
    export SYMBOL_CROSS="[X]"
    export SYMBOL_ARROW="->"
    export SYMBOL_BULLET="*"
    export SYMBOL_STAR="*"
    export SYMBOL_WARNING="[!]"
    export SYMBOL_INFO="[i]"
    export SYMBOL_GEAR="[*]"
    export SYMBOL_ROCKET="[>]"
    export SYMBOL_PACKAGE="[P]"
    export SYMBOL_WRENCH="[W]"
    export SYMBOL_HAMMER="[H]"
    export SYMBOL_LIGHTNING="[!]"
    export SYMBOL_DNA="[D]"
    export SYMBOL_KERNEL="[K]"
    export SYMBOL_MODULE="[M]"
    export SYMBOL_TEST="[T]"
    export SYMBOL_CLEAN="[C]"
    export SYMBOL_CLOCK="[:]"
    export SYMBOL_PROGRESS="#"
    export SYMBOL_EMPTY="-"
fi
