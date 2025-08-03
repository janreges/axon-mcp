#!/usr/bin/env bash
# Test helper functions for bats tests

# Mock curl for testing downloads
mock_curl() {
    if [[ "$1" == "-fsSL" ]]; then
        # Mock successful download
        echo "mock binary content"
        return 0
    fi
    return 1
}

# Mock tar for testing extraction
mock_tar() {
    if [[ "$1" == "-xzf" ]]; then
        # Mock successful extraction
        touch "$TEST_DIR/axon-mcp"
        chmod +x "$TEST_DIR/axon-mcp"
        return 0
    fi
    return 1
}

# Helper to source install.sh with mocks
source_with_mocks() {
    export -f mock_curl
    export -f mock_tar
    alias curl=mock_curl
    alias tar=mock_tar
    source "$BATS_TEST_DIRNAME/../install.sh"
}