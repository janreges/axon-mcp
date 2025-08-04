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

# Helper functions with beautiful colored dots
info() {
    printf "${BLUE}●${RESET}  %s\n" "$1"
}

success() {
    printf "${GREEN}●${RESET}  %s\n" "$1"
}

warning() {
    printf "${YELLOW}●${RESET}  %s\n" "$1" >&2
}

error() {
    printf "${RED}●${RESET}  %s\n" "$1" >&2
}

fatal() {
    error "$1"
    exit 1
}

# Interactive prompt with yes/no
prompt_yes_no() {
    local prompt_message="$1"
    local default_answer="${2:-Y}" # Default to 'Y' if not specified
    while true; do
        if [ "$default_answer" = "Y" ] || [ "$default_answer" = "y" ]; then
            read -rp "$prompt_message [Y/n]: " yn
        else
            read -rp "$prompt_message [y/N]: " yn
        fi
        case "$yn" in
            [Yy]* ) return 0;; # Yes
            [Nn]* ) return 1;; # No
            "" )
                if [ "$default_answer" = "Y" ] || [ "$default_answer" = "y" ]; then
                    return 0
                else
                    return 1
                fi;;
            * ) warning "Please answer 'y' or 'n'.";;
        esac
    done
}

# Function to find project root (Git or .claude marker)
find_project_root() {
    local current_dir="$PWD"
    local root_found=""

    while [ "$current_dir" != "/" ] && [ "$current_dir" != "" ]; do
        if [ -d "$current_dir/.git" ]; then
            root_found="$current_dir"
            break
        elif [ -d "$current_dir/.claude" ]; then
            root_found="$current_dir"
            break
        fi
        current_dir=$(dirname "$current_dir")
    done

    echo "$root_found"
}

# Detect OS and architecture
detect_platform() {
    RAW_OS="$(uname -s)"
    RAW_ARCH="$(uname -m)"
    
    case "$RAW_OS" in
        Linux*)
            PLATFORM_OS="linux"
            ;;
        Darwin*)
            PLATFORM_OS="darwin"
            ;;
        MINGW* | MSYS* | CYGWIN*)
            PLATFORM_OS="windows"
            ;;
        *)
            fatal "Unsupported operating system: $RAW_OS"
            ;;
    esac
    
    case "$RAW_ARCH" in
        x86_64 | amd64)
            PLATFORM_ARCH="amd64"
            ;;
        aarch64 | arm64)
            PLATFORM_ARCH="arm64"
            ;;
        *)
            fatal "Unsupported architecture: $RAW_ARCH"
            ;;
    esac
    
    PLATFORM="${PLATFORM_OS}-${PLATFORM_ARCH}"
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

# Download and extract binary to specified directory
download_binary_to_dir() {
    local target_install_dir="$1"
    
    # Determine file extension based on platform
    if [ "$PLATFORM_OS" = "windows" ]; then
        FILE_EXT=".zip"
    else
        FILE_EXT=".tar.gz"
    fi
    
    # Generate beautiful asset name based on version
    if [ "$VERSION" = "latest" ]; then
        # For latest, we need to get the actual version tag from GitHub API
        LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep -o '"tag_name": *"[^"]*"' | grep -o 'v[^"]*' | head -1)
        if [ -z "$LATEST_VERSION" ]; then
            fatal "Failed to get latest version from GitHub API"
        fi
        VERSION_TAG="$LATEST_VERSION"
    else
        VERSION_TAG="v$VERSION"
    fi
    
    BINARY_NAME="${MCP_NAME}-${PLATFORM}-${VERSION_TAG}"
    
    if [ "$VERSION" = "latest" ]; then
        DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/${BINARY_NAME}${FILE_EXT}"
    else
        DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION_TAG}/${BINARY_NAME}${FILE_EXT}"
    fi
    
    info "Downloading $MCP_NAME from $DOWNLOAD_URL"
    
    # Create temp directory
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    
    # Download with timeout
    ARCHIVE_FILE="${BINARY_NAME}${FILE_EXT}"
    if ! curl -fsSL --connect-timeout 30 --max-time 300 "$DOWNLOAD_URL" -o "$TMP_DIR/${ARCHIVE_FILE}"; then
        fatal "Failed to download binary. Please check your internet connection and try again."
    fi
    
    # Verify download (check it's not empty)
    if [ ! -s "$TMP_DIR/${ARCHIVE_FILE}" ]; then
        fatal "Downloaded file is empty"
    fi
    
    # Extract
    info "Extracting binary..."
    if [ "$PLATFORM_OS" = "windows" ]; then
        # Extract zip file
        if ! unzip -q "$TMP_DIR/${ARCHIVE_FILE}" -d "$TMP_DIR"; then
            fatal "Failed to extract binary from zip."
        fi
    else
        # Extract tar.gz file
        if ! tar -xzf "$TMP_DIR/${ARCHIVE_FILE}" -C "$TMP_DIR"; then
            fatal "Failed to extract binary from tar.gz."
        fi
    fi
    
    # Create install directory if it doesn't exist
    info "Creating installation directory: $target_install_dir"
    if ! mkdir -p "$target_install_dir"; then
        fatal "Failed to create directory $target_install_dir. Check permissions or run with sudo if necessary."
    fi
    
    # Install binary atomically with update support
    if [ "$PLATFORM_OS" = "windows" ]; then
        SOURCE_BINARY="$TMP_DIR/${MCP_NAME}.exe"
        TARGET_BINARY="$target_install_dir/${MCP_NAME}.exe"
    else
        SOURCE_BINARY="$TMP_DIR/$MCP_NAME"
        TARGET_BINARY="$target_install_dir/$MCP_NAME"
    fi
    
    # Check if binary already exists and get version info
    if [ -f "$TARGET_BINARY" ]; then
        EXISTING_VERSION=$("$TARGET_BINARY" --version 2>/dev/null || echo "unknown")
        info "Found existing binary: $EXISTING_VERSION"
        info "Updating to latest version..."
        
        # Create backup of existing binary
        BACKUP_BINARY="${TARGET_BINARY}.backup"
        if ! cp "$TARGET_BINARY" "$BACKUP_BINARY" 2>/dev/null; then
            warning "Could not create backup of existing binary"
        fi
    else
        info "Installing binary '$MCP_NAME' to '$target_install_dir'..."
    fi
    
    # Atomic installation: write to temp file first, then move
    TEMP_BINARY="${TARGET_BINARY}.tmp"
    if ! cp "$SOURCE_BINARY" "$TEMP_BINARY"; then
        fatal "Failed to copy binary to temporary location. Check permissions."
    fi
    
    # Atomic move to final location
    if ! mv "$TEMP_BINARY" "$TARGET_BINARY"; then
        fatal "Failed to install binary. Check permissions."
    fi
    
    success "Binary '$MCP_NAME' successfully installed to '$target_install_dir'."
    
    # Ensure executable permissions (not needed on Windows)
    if [ "$PLATFORM_OS" != "windows" ]; then
        if ! chmod +x "$TARGET_BINARY"; then
            warning "Failed to set executable permissions for '$TARGET_BINARY'."
        fi
    fi
    
    # Clean up backup file after successful installation
    if [ -f "${TARGET_BINARY}.backup" ]; then
        rm -f "${TARGET_BINARY}.backup" 2>/dev/null || true
    fi
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
    if [ "$PLATFORM_OS" = "windows" ]; then
        BINARY_PATH="$INSTALL_DIR/${MCP_NAME}.exe"
    else
        BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
    fi
    
    info "Configuring Claude Code..."
    
    # Method 1: Try using claude CLI with proper scope
    if command -v claude >/dev/null 2>&1; then
        info "Found claude CLI, attempting automatic configuration..."
        
        # Try project scope first if in a project directory
        if [ -d ".git" ] || [ -d ".claude" ]; then
            info "Attempting project-scoped MCP configuration..."
            if claude mcp add -s project "$MCP_NAME" -- "$BINARY_PATH" 2>/dev/null; then
                success "Claude Code configured successfully in project scope!"
                return 0
            else
                warning "Project-scoped configuration failed, trying user scope..."
            fi
        fi
        
        # Try user scope as fallback
        info "Attempting user-scoped MCP configuration..."
        if claude mcp add -s user "$MCP_NAME" -- "$BINARY_PATH" 2>/dev/null; then
            success "Claude Code configured successfully in user scope!"
            return 0
        else
            warning "User-scoped configuration failed, trying local scope..."
        fi
        
        # Try local scope as final fallback
        info "Attempting local-scoped MCP configuration..."
        if claude mcp add -s local "$MCP_NAME" -- "$BINARY_PATH" 2>/dev/null; then
            success "Claude Code configured successfully in local scope!"
            return 0
        else
            warning "All claude mcp add attempts failed."
        fi
    fi
    
    # Method 3: Manual configuration instructions
    warning "Could not configure Claude Code automatically."
    echo ""
    echo "Please add the following configuration manually:"
    echo ""
    printf "For project-specific configuration, add to %b./.mcp.json%b:\n" "$BOLD" "$RESET"
    printf "For global configuration, add to:\n"
    printf "  - macOS: %b~/Library/Application Support/Claude/claude_desktop_config.json%b\n" "$BOLD" "$RESET"
    printf "  - Windows: %b%%APPDATA%%\\Claude\\claude_desktop_config.json%b\n" "$BOLD" "$RESET"
    printf "\n"
    printf "%b{\n" "$BLUE"
    printf "  \"mcpServers\": {\n"
    printf "    \"%s\": {\n" "$MCP_NAME"
    printf "      \"command\": [\"%s\"],\n" "$BINARY_PATH"
    printf "      \"env\": {\n"
    printf "        \"AXON_MCP_SCOPE\": \"project\"\n"
    printf "      }\n"
    printf "    }\n"
    printf "  }\n"
    printf "}%b\n" "$RESET"
    printf "\n"
    printf "%bDatabase Configuration:%b\n" "$YELLOW" "$RESET"
    printf "• Project scope: Database stored in %b.axon/axon-mcp.sqlite%b within your project\n" "$BOLD" "$RESET"
    printf "• User scope: Database stored in user data directory with project isolation\n"
    printf "• To force user-scope, set %bAXON_MCP_SCOPE=user%b in env config\n" "$BOLD" "$RESET"
    echo ""
    
    # Add .axon to .gitignore if in a git repository and using project scope
    setup_gitignore_for_project_scope
}

# Setup .gitignore for project scope
setup_gitignore_for_project_scope() {
    # Check if we're in a git repository
    if [ -d ".git" ] || git rev-parse --git-dir >/dev/null 2>&1; then
        GITIGNORE_FILE=".gitignore"
        
        # Check if .axon is already in .gitignore
        if [ -f "$GITIGNORE_FILE" ] && grep -q "^\.axon/" "$GITIGNORE_FILE"; then
            return 0  # Already configured
        fi
        
        info "Adding .axon/ to .gitignore for project scope database"
        
        # Add .axon entry to .gitignore
        {
            echo ""
            echo "# Axon MCP project database"
            echo ".axon/"
        } >> "$GITIGNORE_FILE"
        
        success "Added .axon/ to .gitignore"
    fi
}

# Health check
health_check() {
    INSTALL_DIR="$(get_install_dir)"
    if [ "$PLATFORM_OS" = "windows" ]; then
        BINARY_PATH="$INSTALL_DIR/${MCP_NAME}.exe"
    else
        BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
    fi
    
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
    printf "\n%b🧠 Axon MCP Installer%b\n" "$BOLD" "$RESET"
    printf "%b════════════════════════%b\n\n" "$BOLD" "$RESET"
    
    # --- CLI Parsing and Installation Path Logic ---
    PROJECT_ROOT=""
    INSTALL_MODE="auto" # 'auto', 'project', 'user'
    INSTALL_DIR=""
    
    # Parse CLI arguments
    for arg in "$@"; do
        case $arg in
            --claude-code-project)
                INSTALL_MODE="project"
                shift # Remove argument from processing
                ;;
            --claude-code-user)
                INSTALL_MODE="user"
                shift # Remove argument from processing
                ;;
            *)
                # Unknown argument, let's not fail for now, but could be added
                ;;
        esac
    done
    
    # Determine project root if not explicitly set by CLI
    if [ "$INSTALL_MODE" = "auto" ] || [ "$INSTALL_MODE" = "project" ]; then
        info "Detecting project root..."
        PROJECT_ROOT=$(find_project_root)
        if [ -n "$PROJECT_ROOT" ]; then
            success "Project root detected: $PROJECT_ROOT (installing to .axon/bin)"
            if [ "$INSTALL_MODE" = "auto" ]; then
                INSTALL_MODE="project" # Default to project scope if detected and no override
            fi
        else
            warning "Project root not found. Installation will continue in user scope."
            if [ "$INSTALL_MODE" = "project" ]; then
                fatal "Argument --claude-code-project was provided, but project root was not found. Aborting."
            fi
            INSTALL_MODE="user" # Fallback to user if --claude-code-project not specified
        fi
    fi
    
    # Set installation directory based on determined mode
    if [ "$INSTALL_MODE" = "project" ]; then
        INSTALL_DIR="$PROJECT_ROOT/.axon/bin"
    elif [ "$INSTALL_MODE" = "user" ]; then
        # Use existing get_install_dir logic
        INSTALL_DIR="$(get_install_dir)"
        info "User-scoped installation to: $INSTALL_DIR"
    else
        fatal "Unknown installation mode: $INSTALL_MODE"
    fi
    
    # Check requirements
    check_requirements
    
    # Detect platform  
    detect_platform
    
    # Download and install with custom directory
    download_binary_to_dir "$INSTALL_DIR"
    
    # Configure PATH (only for user installs)
    if [ "$INSTALL_MODE" = "user" ]; then
        configure_path
    fi
    
    # Run health check
    health_check
    # --- Post-Installation Automation ---
    if [ "$INSTALL_MODE" = "project" ]; then
        info "Running automation steps for project-scoped installation..."

        # Add .axon/ to .gitignore
        GITIGNORE_PATH="$PROJECT_ROOT/.gitignore"
        if [ -f "$GITIGNORE_PATH" ]; then
            if ! grep -q "^\.axon/$" "$GITIGNORE_PATH"; then
                if prompt_yes_no "Add '.axon/' to '$GITIGNORE_PATH'?" "Y"; then
                    echo ".axon/" >> "$GITIGNORE_PATH"
                    success "Added '.axon/' to '$GITIGNORE_PATH'."
                else
                    info "Adding '.axon/' to .gitignore skipped."
                fi
            else
                info "'.axon/' is already in '$GITIGNORE_PATH'."
            fi
        else
            info ".gitignore not found in '$PROJECT_ROOT'. Skipping adding '.axon/'."
        fi

        # claude mcp add
        CLAUDE_DIR="$PROJECT_ROOT/.claude"
        if [ -d "$CLAUDE_DIR" ]; then
            info "Detected '.claude/' folder in project root."
            if prompt_yes_no "Run 'claude mcp add' for this project?" "Y"; then
                info "Running 'claude mcp add' with project scope..."
                # Execute claude mcp add from the project root with proper binary path
                CORRECT_BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
                if [ "$PLATFORM_OS" = "windows" ]; then
                    CORRECT_BINARY_PATH="${CORRECT_BINARY_PATH}.exe"
                fi
                
                (cd "$PROJECT_ROOT" && claude mcp add -s project axon-mcp -- "$CORRECT_BINARY_PATH") 2>/dev/null
                if [ $? -eq 0 ]; then
                    success "'claude mcp add' executed successfully in project scope."
                else
                    warning "'claude mcp add' failed. Check output for details or run manually."
                fi
            else
                info "Running 'claude mcp add' skipped."
            fi
        else
            info "'.claude/' folder not found in project root. Skipping 'claude mcp add'."
        fi

        info "To use '$MCP_NAME' in this project, we recommend adding '$INSTALL_DIR' to your PATH, e.g. 'export PATH=\"\$PATH:$INSTALL_DIR\"' or run the binary directly: '$INSTALL_DIR/$MCP_NAME'."
        info "Or you can use alias: 'alias $MCP_NAME=\"$INSTALL_DIR/$MCP_NAME\"'."

    elif [ "$INSTALL_MODE" = "user" ]; then
        info "Running automation steps for user-scoped installation..."
        info "Make sure '$INSTALL_DIR' is in your PATH. You can add it to ~/.bashrc, ~/.zshrc or ~/.profile:"
        info "  export PATH=\"\$PATH:$INSTALL_DIR\""
        info "Then run 'source ~/.bashrc' (or appropriate file) or restart terminal."
    fi
    
    printf "\n%b🎉 Axon MCP Installed Successfully!%b\n\n" "$GREEN$BOLD" "$RESET"
    
    # Smart project detection and parameter pre-filling
    DETECTED_PROJECT_ROOT=""
    DETECTED_PROJECT_NAME=""
    SERVER_PORT="8499"
    
    if [ "$INSTALL_MODE" = "project" ] && [ -n "$PROJECT_ROOT" ]; then
        DETECTED_PROJECT_ROOT="$PROJECT_ROOT"
        BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
    else
        # Try to detect project from current directory
        CURRENT_DIR="$(pwd)"
        DETECTED_PROJECT_ROOT=$(find_project_root)
        if [ -z "$DETECTED_PROJECT_ROOT" ]; then
            DETECTED_PROJECT_ROOT="$CURRENT_DIR"
        fi
        
        if [ "$INSTALL_MODE" = "project" ]; then
            BINARY_PATH="$INSTALL_DIR/$MCP_NAME"
        else
            BINARY_PATH="$MCP_NAME"
        fi
    fi
    
    # Generate sanitized project name from detected path
    DETECTED_PROJECT_NAME=$(basename "$DETECTED_PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g' | sed -E 's/^-+|-+$//g')
    
    if [ "$PLATFORM_OS" = "windows" ]; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi
    
    printf "%b🚀 Ready to Start Server:%b\n\n" "$BOLD" "$RESET"
    
    
    # Display auto-detected values prominently
    printf "%b✨ Auto-detected configuration:%b\n" "$GREEN" "$RESET" 
    printf "   Project root: %b%s%b\n" "$BLUE" "$DETECTED_PROJECT_ROOT" "$RESET"
    printf "   Project name: %b%s%b (derived from project folder name)\n" "$BLUE" "$DETECTED_PROJECT_NAME" "$RESET"
    printf "   Server port:  %b%s%b (If in use, change --port=%s to e.g., --port=%s in command below)\n\n" "$BLUE" "$SERVER_PORT" "$RESET" "$SERVER_PORT" "$((SERVER_PORT + 1))"
    
    # Show the ready-to-use command in a prominent box
    printf "%b┌─────────────────────────────────────────────────────────────────────┐%b\n" "$BOLD" "$RESET"
    printf "%b│                    📋 COPY & PASTE COMMAND                          │%b\n" "$BOLD" "$RESET"
    printf "%b├─────────────────────────────────────────────────────────────────────┤%b\n" "$BOLD" "$RESET"
    printf "%b│%b %s --start --port=%s --project=\"%s\" --project-root=\"%s\" %b│%b\n" "$BOLD" "$BLUE" "$BINARY_PATH" "$SERVER_PORT" "$DETECTED_PROJECT_NAME" "$DETECTED_PROJECT_ROOT" "$BOLD" "$RESET"
    printf "%b└─────────────────────────────────────────────────────────────────────┘%b\n\n" "$BOLD" "$RESET"
    
    printf "%b📋 Step-by-step instructions:%b\n\n" "$BOLD" "$RESET"
    
    printf "%b1. Start the server:%b\n" "$BOLD" "$RESET"
    printf "   • Open a %bNEW terminal window%b\n" "$BOLD" "$RESET"
    printf "   • Copy and paste the command from the box above\n"
    printf "   • Press Enter to start the server (expect output like: \"[INFO] Server listening on http://127.0.0.1:%s\")\n\n" "$SERVER_PORT"
    
    printf "%b2. Connect Claude Code:%b\n" "$BOLD" "$RESET"
    printf "   • Open your terminal in the project root:\n"
    printf "     %bcd \"%s\"%b\n" "$BLUE" "$DETECTED_PROJECT_ROOT" "$RESET"
    printf "   • Then run:\n"
    printf "     %bclaude mcp add --url http://127.0.0.1:%s%b\n\n" "$BLUE" "$SERVER_PORT" "$RESET"
    
    printf "%b✅ Verification (optional):%b\n" "$BOLD" "$RESET"
    printf "   • Health check: %bcurl http://127.0.0.1:%s/health%b\n" "$BLUE" "$SERVER_PORT" "$RESET"
    printf "   • In Claude Code: Type %b/mcp%b to verify connection\n\n" "$BLUE" "$RESET"
    
    printf "%b💡 Pro Tips:%b\n" "$YELLOW" "$RESET"
    printf "   • Keep the server terminal open (don't close it)\n"
    printf "   • Stop server anytime with Ctrl+C\n"
    printf "   • Database will be stored in: %b%s/.axon/axon.%s.sqlite%b\n" "$BLUE" "$DETECTED_PROJECT_ROOT" "$DETECTED_PROJECT_NAME" "$RESET"
    printf "   • Wrong project name? Edit it in the command above before running\n\n"
}

# Run main function only if script is executed directly (not sourced)
# This works in both bash and POSIX sh
if [ "${0##*/}" = "install.sh" ] || [ "${0##*/}" = "sh" ] || [ -z "${BASH_SOURCE-}" ]; then
    main "$@"
fi