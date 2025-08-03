#!/bin/bash
# Cross-platform testing script to catch issues before GitHub Actions
# Run this locally to test builds on multiple targets

set -e

echo "🔍 Cross-Platform Testing Script"
echo "================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "mcp-server" ]; then
    log_error "Please run this script from the project root directory"
    exit 1
fi

# Test targets to check
TARGETS=(
    "x86_64-unknown-linux-musl"
    "x86_64-pc-windows-msvc"
    "x86_64-apple-darwin"
)

if [[ "$OSTYPE" == "darwin"* ]]; then
    TARGETS+=("aarch64-apple-darwin")
fi

log_info "Installing required Rust targets..."
for target in "${TARGETS[@]}"; do
    log_info "Installing target: $target"
    rustup target add "$target" || true
done

echo ""
log_info "Running cargo check for each target..."

SUCCESS_COUNT=0
FAIL_COUNT=0

for target in "${TARGETS[@]}"; do
    echo ""
    log_info "Checking target: $target"
    
    if cargo check --target "$target" --bin axon-mcp 2>&1; then
        log_success "✓ $target - PASSED"
        ((SUCCESS_COUNT++))
    else
        log_error "✗ $target - FAILED"
        ((FAIL_COUNT++))
    fi
done

echo ""
echo "=========================================="
echo -e "Cross-platform test results:"
echo -e "${GREEN}✓ Passed: $SUCCESS_COUNT${NC}"
echo -e "${RED}✗ Failed: $FAIL_COUNT${NC}"
echo "=========================================="

if [ $FAIL_COUNT -eq 0 ]; then
    log_success "All targets passed! Ready for GitHub Actions."
    exit 0
else
    log_error "Some targets failed. Fix issues before pushing to GitHub."
    exit 1
fi