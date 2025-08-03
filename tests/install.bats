#!/usr/bin/env bats
# Installation script tests using bats-core
# https://github.com/bats-core/bats-core

load 'test_helper'

setup() {
    # Create temporary directory for tests
    export TEST_DIR="$(mktemp -d)"
    export INSTALL_DIR="$TEST_DIR/bin"
    export HOME="$TEST_DIR/home"
    mkdir -p "$INSTALL_DIR" "$HOME"
}

teardown() {
    # Clean up
    rm -rf "$TEST_DIR"
}

@test "install.sh: script exists and is executable" {
    [ -f "$BATS_TEST_DIRNAME/../install.sh" ]
    [ -x "$BATS_TEST_DIRNAME/../install.sh" ]
}

@test "install.sh: detect platform on Linux" {
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        skip "Linux-specific test"
    fi
    
    run bash -c "source $BATS_TEST_DIRNAME/../install.sh && detect_platform && echo \$PLATFORM"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "linux" ]]
}

@test "install.sh: detect platform on macOS" {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        skip "macOS-specific test"
    fi
    
    run bash -c "source $BATS_TEST_DIRNAME/../install.sh && detect_platform && echo \$PLATFORM"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "apple-darwin" ]]
}

@test "install.sh: check requirements function" {
    run bash -c "source $BATS_TEST_DIRNAME/../install.sh && check_requirements"
    [ "$status" -eq 0 ]
}

@test "install.sh: get_install_dir respects INSTALL_DIR env" {
    export INSTALL_DIR="/custom/path"
    run bash -c "source $BATS_TEST_DIRNAME/../install.sh && get_install_dir"
    [ "$status" -eq 0 ]
    [ "$output" = "/custom/path" ]
}

@test "install.sh: get_install_dir defaults to ~/.local/bin" {
    unset INSTALL_DIR
    export PATH="$HOME/.local/bin:$PATH"
    run bash -c "source $BATS_TEST_DIRNAME/../install.sh && get_install_dir"
    [ "$status" -eq 0 ]
    [ "$output" = "$HOME/.local/bin" ]
}

@test "install.ps1: script exists" {
    [ -f "$BATS_TEST_DIRNAME/../install.ps1" ]
}

@test "install.ps1: valid PowerShell syntax" {
    if ! command -v pwsh &> /dev/null; then
        skip "PowerShell not available"
    fi
    
    run pwsh -NoProfile -Command "& { \$ErrorActionPreference = 'Stop'; . '$BATS_TEST_DIRNAME/../install.ps1' -WhatIf }"
    [ "$status" -eq 0 ]
}