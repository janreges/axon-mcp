#!/bin/sh
# Axon MCP Installer Script
# https://github.com/janreges/axon-mcp
#
# This script installs axon-mcp on your system.
# It detects your platform, downloads the appropriate binary,
# and configures Claude Code automatically.

set -e

# Configuration
GITHUB_REPO="janreges/axon-mcp"
INSTALL_DIR_DEFAULT="$HOME/.local/bin"
MCP_NAME="axon-mcp"
VERSION="${VERSION:-latest}"

# Colors for output (disabled in CI or if NO_COLOR is set)
if [ -t 1 ] && [ -z "$NO_COLOR" ] && [ -z "$CI" ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    RESET=''
fi

# Helper functions
info() {
    printf "${BLUE}ℹ${RESET}  %s\n" "$1"
}

success() {
    printf "${GREEN}✓${RESET}  %s\n" "$1"
}

warning() {
    printf "${YELLOW}⚠${RESET}  %s\n" "$1" >&2
}

error() {
    printf "${RED}✗${RESET}  %s\n" "$1" >&2
}

fatal() {
    error "$1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    
    case "$OS" in
        Linux*)
            OS="unknown-linux-musl"
            ;;
        Darwin*)
            OS="apple-darwin"
            # We'll use universal binary for macOS
            ARCH="universal"
            ;;
        MINGW* | MSYS* | CYGWIN*)
            OS="pc-windows-msvc"
            ;;
        *)
            fatal "Unsupported operating system: $OS"
            ;;
    esac
    
    case "$ARCH" in
        x86_64 | amd64)
            if [ "$OS" != "apple-darwin" ]; then
                ARCH="x86_64"
            fi
            ;;
        aarch64 | arm64)
            if [ "$OS" != "apple-darwin" ]; then
                ARCH="aarch64"
            fi
            ;;
        universal)
            # Already set for macOS
            ;;
        *)
            fatal "Unsupported architecture: $ARCH"
            ;;
    esac
    
    PLATFORM="${ARCH}-${OS}"
    info "Detected platform: $PLATFORM"
}

# Check for required tools
check_requirements() {
    MISSING_TOOLS=""
    
    for tool in curl tar; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            MISSING_TOOLS="$MISSING_TOOLS $tool"
        fi
    done
    
    if [ -n "$MISSING_TOOLS" ]; then
        fatal "Missing required tools:$MISSING_TOOLS"
    fi
}

# Determine installation directory
get_install_dir() {
    if [ -n "$INSTALL_DIR" ]; then
        echo "$INSTALL_DIR"
        return
    fi
    
    # Check if default dir is in PATH
    if echo "$PATH" | grep -q "$INSTALL_DIR_DEFAULT"; then
        echo "$INSTALL_DIR_DEFAULT"
        return
    fi
    
    # Check other common directories
    for dir in "$HOME/bin" "/usr/local/bin"; do
        if echo "$PATH" | grep -q "$dir" && [ -w "$dir" ]; then
            echo "$dir"
            return
        fi
    done
    
    # Default to ~/.local/bin
    echo "$INSTALL_DIR_DEFAULT"
}

# Download and extract binary
download_binary() {
    INSTALL_DIR="$(get_install_dir)"
    BINARY_NAME="${MCP_NAME}-${PLATFORM}"
    
    if [ "$VERSION" = "latest" ]; then
        DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/${BINARY_NAME}.tar.gz"
    else
        DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_NAME}.tar.gz"
    fi
    
    info "Downloading $MCP_NAME from $DOWNLOAD_URL"
    
    # Create temp directory
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    
    # Download with timeout
    if ! curl -fsSL --connect-timeout 30 --max-time 300 "$DOWNLOAD_URL" -o "$TMP_DIR/${BINARY_NAME}.tar.gz"; then
        fatal "Failed to download binary. Please check your internet connection and try again."
    fi
    
    # Verify download (check it's not empty)
    if [ ! -s "$TMP_DIR/${BINARY_NAME}.tar.gz" ]; then
        fatal "Downloaded file is empty"
    fi
    
    # Extract
    info "Extracting binary..."
    if ! tar -xzf "$TMP_DIR/${BINARY_NAME}.tar.gz" -C "$TMP_DIR"; then
        fatal "Failed to extract binary."
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Install binary atomically
    info "Installing to $INSTALL_DIR/$MCP_NAME"
    if ! mv "$TMP_DIR/$MCP_NAME" "$INSTALL_DIR/$MCP_NAME.tmp"; then
        fatal "Failed to install binary. Do you have write permissions to $INSTALL_DIR?"
    fi
    
    # Set permissions before final move
    chmod +x "$INSTALL_DIR/$MCP_NAME.tmp"
    
    # Atomic rename
    if ! mv -f "$INSTALL_DIR/$MCP_NAME.tmp" "$INSTALL_DIR/$MCP_NAME"; then
        fatal "Failed to finalize installation"
    fi
    
    # Verify executable permissions
    if [ ! -x "$INSTALL_DIR/$MCP_NAME" ]; then
        chmod +x "$INSTALL_DIR/$MCP_NAME" || fatal "Failed to set executable permissions"
    fi
    
    success "Binary installed successfully!"
}

# Configure PATH if needed
configure_path() {
    INSTALL_DIR="$(get_install_dir)"
    
    # Check if install dir is in PATH
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return 0
    fi
    
    warning "$INSTALL_DIR is not in your PATH"
    
    # Detect shell and config file
    SHELL_NAME="$(basename "$SHELL")"
    case "$SHELL_NAME" in
        bash)
            CONFIG_FILE="$HOME/.bashrc"
            ;;
        zsh)
            CONFIG_FILE="$HOME/.zshrc"
            ;;
        fish)
            CONFIG_FILE="$HOME/.config/fish/config.fish"
            ;;
        *)
            CONFIG_FILE="$HOME/.profile"
            ;;
    esac
    
    info "Adding $INSTALL_DIR to PATH in $CONFIG_FILE"
    
    # Add PATH export with marker comment
    {
        echo ""
        echo "# >>> axon-mcp installer >>>"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
        echo "# <<< axon-mcp installer <<<"
    } >> "$CONFIG_FILE"
    
    warning "Please restart your shell or run: source $CONFIG_FILE"
}

# Configure Claude Code
configure_claude() {
    INSTALL_DIR="$(get_install_dir)"
    BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
    
    info "Configuring Claude Code..."
    
    # Method 1: Try using claude CLI
    if command -v claude >/dev/null 2>&1; then
        info "Found claude CLI, attempting automatic configuration..."
        
        if claude mcp add "$MCP_NAME" -- "$BINARY_PATH" 2>/dev/null; then
            success "Claude Code configured successfully!"
            return 0
        else
            warning "claude mcp add failed, trying alternative method..."
        fi
        
        # Method 2: Try claude mcp add-json
        JSON_CONFIG="{\"command\":[\"$BINARY_PATH\"]}"
        if echo "$JSON_CONFIG" | claude mcp add-json "$MCP_NAME" 2>/dev/null; then
            success "Claude Code configured successfully using add-json!"
            return 0
        fi
    fi
    
    # Method 3: Manual configuration instructions
    warning "Could not configure Claude Code automatically."
    echo ""
    echo "Please add the following configuration manually:"
    echo ""
    echo "For project-specific configuration, add to ${BOLD}./.mcp.json${RESET}:"
    echo "For global configuration, add to:"
    echo "  - macOS: ${BOLD}~/Library/Application Support/Claude/claude_desktop_config.json${RESET}"
    echo "  - Windows: ${BOLD}%APPDATA%\\Claude\\claude_desktop_config.json${RESET}"
    echo ""
    echo "${BLUE}{"
    echo "  \"mcpServers\": {"
    echo "    \"$MCP_NAME\": {"
    echo "      \"command\": [\"$BINARY_PATH\"]"
    echo "    }"
    echo "  }"
    echo "}${RESET}"
    echo ""
}

# Health check
health_check() {
    INSTALL_DIR="$(get_install_dir)"
    BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
    
    info "Running health check..."
    
    if [ -x "$BINARY_PATH" ]; then
        VERSION_OUTPUT="$("$BINARY_PATH" --version 2>&1 || true)"
        if [ -n "$VERSION_OUTPUT" ]; then
            success "axon-mcp is installed and working: $VERSION_OUTPUT"
        else
            warning "axon-mcp is installed but --version returned no output"
        fi
    else
        error "axon-mcp binary not found or not executable at $BINARY_PATH"
        return 1
    fi
}

# Main installation flow
main() {
    echo "${BOLD}Axon MCP Installer${RESET}"
    echo "==================="
    echo ""
    
    # Check requirements
    check_requirements
    
    # Detect platform
    detect_platform
    
    # Download and install
    download_binary
    
    # Configure PATH
    configure_path
    
    # Configure Claude Code
    configure_claude
    
    # Run health check
    health_check
    
    echo ""
    success "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Restart your shell or run: ${BOLD}source ~/.bashrc${RESET} (or appropriate config file)"
    echo "  2. Verify installation: ${BOLD}${MCP_NAME} --version${RESET}"
    echo "  3. In Claude Code, verify connection with: ${BOLD}/mcp${RESET}"
    echo ""
    echo "For updates, run: ${BOLD}${MCP_NAME} self-update${RESET}"
    echo ""
}

# Run main function
main "$@"