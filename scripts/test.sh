#!/bin/bash
# =============================================================================
# Helix OS Framework - Test Suite Runner
# =============================================================================
# Run comprehensive tests with beautiful output
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/progress.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"

# =============================================================================
# Test Configuration
# =============================================================================

TEST_CATEGORIES=(
    "unit:Unit Tests:--lib"
    "integration:Integration Tests:--test '*'"
    "doc:Documentation Tests:--doc"
)

# =============================================================================
# Test Functions
# =============================================================================

run_unit_tests() {
    local log_file="${HELIX_ROOT}/build/logs/test_unit.log"
    mkdir -p "$(dirname "${log_file}")"
    
    print_subheader "Unit Tests"
    
    local test_output
    start_spinner "Running unit tests..."
    
    if test_output=$(cargo_test --workspace --lib 2>&1 | tee "${log_file}"); then
        stop_spinner "success" "Unit tests passed"
        
        # Parse results
        local passed=$(echo "${test_output}" | grep -oP 'test result: ok\. \K\d+' || echo "0")
        local failed=$(echo "${test_output}" | grep -oP 'test result: FAILED\. \K\d+' || echo "0")
        
        echo -e "    ${HELIX_SUCCESS}${SYMBOL_CHECK} ${passed} passed${RESET}"
        
        return 0
    else
        stop_spinner "error" "Unit tests failed"
        
        # Show failures
        echo ""
        grep -A 5 "^failures:" "${log_file}" || true
        
        return 1
    fi
}

run_integration_tests() {
    local log_file="${HELIX_ROOT}/build/logs/test_integration.log"
    mkdir -p "$(dirname "${log_file}")"
    
    print_subheader "Integration Tests"
    
    # Check if integration tests exist
    local test_count=$(find "${HELIX_ROOT}" -path "*/tests/*.rs" -not -path "./target/*" 2>/dev/null | wc -l)
    
    if [[ ${test_count} -eq 0 ]]; then
        log_info "No integration tests found"
        return 0
    fi
    
    start_spinner "Running integration tests..."
    
    if cargo_test --workspace 2>&1 | tee "${log_file}" | grep -q "test result: ok"; then
        stop_spinner "success" "Integration tests passed"
        return 0
    else
        stop_spinner "error" "Integration tests failed"
        return 1
    fi
}

run_doc_tests() {
    local log_file="${HELIX_ROOT}/build/logs/test_doc.log"
    mkdir -p "$(dirname "${log_file}")"
    
    print_subheader "Documentation Tests"
    
    start_spinner "Running doc tests..."
    
    if cargo_test --workspace --doc 2>&1 | tee "${log_file}" | grep -q "test result: ok"; then
        stop_spinner "success" "Doc tests passed"
        return 0
    else
        stop_spinner "warn" "Doc tests skipped or failed"
        return 0  # Don't fail on doc tests
    fi
}

run_clippy() {
    local log_file="${HELIX_ROOT}/build/logs/clippy.log"
    mkdir -p "$(dirname "${log_file}")"
    
    print_subheader "Clippy Analysis"
    
    start_spinner "Running clippy..."
    
    if cargo_clippy --workspace 2>&1 | tee "${log_file}"; then
        local warnings=$(grep -c "warning:" "${log_file}" 2>/dev/null || echo "0")
        
        if [[ ${warnings} -eq 0 ]]; then
            stop_spinner "success" "No clippy warnings"
        else
            stop_spinner "warn" "${warnings} clippy warnings"
        fi
        return 0
    else
        stop_spinner "error" "Clippy failed"
        return 1
    fi
}

run_fmt_check() {
    print_subheader "Format Check"
    
    start_spinner "Checking formatting..."
    
    if cargo_fmt --all -- --check 2>/dev/null; then
        stop_spinner "success" "Code is properly formatted"
        return 0
    else
        stop_spinner "warn" "Code needs formatting"
        log_info "Run 'cargo fmt' to fix"
        return 0
    fi
}

run_audit() {
    print_subheader "Security Audit"
    
    if ! cmd_exists cargo-audit; then
        log_info "cargo-audit not installed, skipping"
        return 0
    fi
    
    start_spinner "Checking for vulnerabilities..."
    
    if cargo audit 2>/dev/null; then
        stop_spinner "success" "No vulnerabilities found"
        return 0
    else
        stop_spinner "warn" "Vulnerabilities found"
        return 0
    fi
}

# =============================================================================
# Summary
# =============================================================================

print_test_summary() {
    local passed="$1"
    local failed="$2"
    local skipped="$3"
    local duration="$4"
    
    echo ""
    print_header "Test Summary"
    
    if [[ ${failed} -eq 0 ]]; then
        echo -e "  ${HELIX_SUCCESS}${SYMBOL_CHECK} All tests passed!${RESET}"
    else
        echo -e "  ${HELIX_ERROR}${SYMBOL_CROSS} Some tests failed${RESET}"
    fi
    
    echo ""
    echo -e "  Passed:  ${HELIX_SUCCESS}${passed}${RESET}"
    echo -e "  Failed:  ${HELIX_ERROR}${failed}${RESET}"
    echo -e "  Skipped: ${HELIX_WARNING}${skipped}${RESET}"
    echo ""
    echo -e "  Duration: ${duration}"
    echo ""
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
    echo -e "${DIM}    Test Suite Runner${RESET}"
    echo ""
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 [command]"
    echo ""
    
    echo -e "${BOLD}Commands:${RESET}"
    echo "  all           Run all tests (default)"
    echo "  unit          Run unit tests only"
    echo "  integration   Run integration tests only"
    echo "  doc           Run documentation tests"
    echo "  clippy        Run clippy analysis"
    echo "  fmt           Check code formatting"
    echo "  audit         Security audit"
    echo "  quick         Quick check (fmt + clippy + unit)"
    echo "  help          Show this help"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    local command="${1:-all}"
    
    local start_time=$(get_timestamp)
    local passed=0
    local failed=0
    local skipped=0
    
    cd "${HELIX_ROOT}"
    
    case "${command}" in
        all)
            print_header "Helix Test Suite"
            
            init_steps \
                "fmt:Format Check" \
                "clippy:Clippy Analysis" \
                "unit:Unit Tests" \
                "integration:Integration Tests" \
                "doc:Documentation Tests"
            
            start_step "fmt"
            if run_fmt_check; then
                complete_step "fmt"
                passed=$((passed + 1))
            else
                fail_step "fmt"
                failed=$((failed + 1))
            fi
            
            start_step "clippy"
            if run_clippy; then
                complete_step "clippy"
                passed=$((passed + 1))
            else
                fail_step "clippy"
                failed=$((failed + 1))
            fi
            
            start_step "unit"
            if run_unit_tests; then
                complete_step "unit"
                passed=$((passed + 1))
            else
                fail_step "unit"
                failed=$((failed + 1))
            fi
            
            start_step "integration"
            if run_integration_tests; then
                complete_step "integration"
                passed=$((passed + 1))
            else
                fail_step "integration"
                failed=$((failed + 1))
            fi
            
            start_step "doc"
            if run_doc_tests; then
                complete_step "doc"
                passed=$((passed + 1))
            else
                fail_step "doc"
                failed=$((failed + 1))
            fi
            
            local end_time=$(get_timestamp)
            local duration=$(format_duration $((end_time - start_time)))
            
            print_test_summary ${passed} ${failed} ${skipped} "${duration}"
            print_step_summary
            
            [[ ${failed} -eq 0 ]]
            ;;
        
        unit)
            run_unit_tests
            ;;
        
        integration)
            run_integration_tests
            ;;
        
        doc)
            run_doc_tests
            ;;
        
        clippy)
            run_clippy
            ;;
        
        fmt)
            run_fmt_check
            ;;
        
        audit)
            run_audit
            ;;
        
        quick)
            print_header "Quick Test"
            run_fmt_check && run_clippy && run_unit_tests
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
