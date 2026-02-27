#!/bin/bash
# test_cli.sh - Automated CLI test script for Cortex-Mem

# Don't use set -e so the script continues on test failures

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration - modify these for your environment
CONFIG_PATH="${CONFIG_PATH:-/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/config.toml}"
TENANT_ID="${TENANT_ID:-bf323233-1f53-4337-a8e7-2ebe9b0080d0}"
CLI="${CLI:-$PROJECT_ROOT/target/release/cortex-mem}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
pass=0
fail=0
total=0

# Test function
test_case() {
    local id="$1"
    local name="$2"
    local cmd="$3"
    local expected="$4"
    local should_fail="${5:-false}"
    
    ((total++))
    echo -ne "${BLUE}[$id]${NC} $name... "
    
    # Use eval to properly handle quoted arguments with spaces
    if output=$(eval "$cmd" 2>&1); then
        if echo "$output" | grep -q "$expected"; then
            echo -e "${GREEN}PASS${NC}"
            ((pass++))
        else
            echo -e "${RED}FAIL${NC}"
            echo "  Expected to contain: $expected"
            echo "  Got: ${output:0:200}..."
            ((fail++))
        fi
    else
        if [ "$should_fail" = "true" ]; then
            if echo "$output" | grep -q "$expected"; then
                echo -e "${GREEN}PASS (expected error)${NC}"
                ((pass++))
            else
                echo -e "${RED}FAIL${NC}"
                echo "  Expected error containing: $expected"
                echo "  Got: $output"
                ((fail++))
            fi
        else
            echo -e "${RED}FAIL (unexpected error)${NC}"
            echo "  Error: $output"
            ((fail++))
        fi
    fi
}

echo "============================================"
echo "  Cortex-Mem CLI Automated Test Suite"
echo "============================================"
echo ""
echo "Configuration:"
echo "  CLI:         $CLI"
echo "  Config:      $CONFIG_PATH"
echo "  Tenant:      $TENANT_ID"
echo ""

# Check if CLI exists
if [ ! -f "$CLI" ]; then
    echo -e "${RED}Error: CLI binary not found at $CLI${NC}"
    echo "Please build it first: cargo build --release --bin cortex-mem"
    exit 1
fi

# Check if config exists
if [ ! -f "$CONFIG_PATH" ]; then
    echo -e "${RED}Error: Config file not found at $CONFIG_PATH${NC}"
    exit 1
fi

echo "============================================"
echo "  1. Basic Commands"
echo "============================================"

test_case "B01" "Help command" \
    "$CLI --help" \
    "Cortex-Mem CLI"

test_case "B02" "Version command" \
    "$CLI --version" \
    "cortex-mem"

echo ""
echo "============================================"
echo "  2. Tenant Management"
echo "============================================"

test_case "T01" "List tenants" \
    "$CLI -c \"$CONFIG_PATH\" tenant list" \
    "Found"

echo ""
echo "============================================"
echo "  3. Session Management"
echo "============================================"

test_case "S01" "List sessions" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" session list" \
    "sessions"

echo ""
echo "============================================"
echo "  4. Statistics"
echo "============================================"

test_case "ST01" "Show statistics" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" stats" \
    "Statistics"

echo ""
echo "============================================"
echo "  5. Memory Listing"
echo "============================================"

test_case "L01" "List session root" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" list" \
    "Found"

test_case "L02" "List user dimension" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" list --uri cortex://user" \
    "Found"

echo ""
echo "============================================"
echo "  6. Layer Management"
echo "============================================"

test_case "Y01" "Layer status (English output)" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" layers status" \
    "Layer file status"

test_case "Y02" "Layer status shows correct command name" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" layers status" \
    "cortex-mem layers ensure-all"

echo ""
echo "============================================"
echo "  7. Error Handling"
echo "============================================"

# Note: R07 (negative min_score) is skipped because clap rejects negative numbers
# as options before our validation code runs. This is expected clap behavior.

test_case "R08" "Invalid min_score (> 1.0)" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" search test -s 2.0" \
    "min_score must be between" \
    "true"

test_case "G06" "Invalid URI scheme" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" get invalid-uri" \
    "Invalid URI scheme" \
    "true"

echo ""
echo "============================================"
echo "  8. Directory Abstract (Bug #1 Fix)"
echo "============================================"

test_case "G03" "Get directory abstract" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" get cortex://user/tars_user/entities --abstract-only" \
    "Abstract"

echo ""
echo "============================================"
echo "  9. Add Message (Bug #4 Fix)"
echo "============================================"

test_case "M05" "Add message URI format" \
    "$CLI -c \"$CONFIG_PATH\" --tenant \"$TENANT_ID\" add --thread test-auto --role user \"Automated test message\"" \
    "cortex://session/test-auto/timeline"

echo ""
echo "============================================"
echo "  Test Summary"
echo "============================================"
echo ""
echo -e "Total:  $total"
echo -e "Passed: ${GREEN}$pass${NC}"
echo -e "Failed: ${RED}$fail${NC}"
echo ""

if [ $fail -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
