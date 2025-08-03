# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-08-03

### 🔄 Major Database Architecture Enhancement

This release introduces a comprehensive scope-based database isolation system to prevent conflicts when developing multiple projects in parallel.

### Added

#### Scope-Based Database Isolation
- **Project-scoped installation**: Database stored in `.axon/axon-mcp.sqlite` within project directory
- **User-scoped installation**: Database stored in user data directory with project-specific hash isolation
- **Dynamic scope detection**: Automatic detection based on executable location and project markers
- **Environment overrides**: Support for `AXON_MCP_SCOPE` and `AXON_MCP_DB` environment variables

#### Enhanced Installation System
- **Project-scope installation**: New `--claude-code-project` CLI switch for project-specific installation
- **User-scope installation**: New `--claude-code-user` CLI switch for global installation
- **Smart automation**: Automatic `.gitignore` management for `.axon/` directories
- **Claude Code integration**: Automatic `claude mcp add` when `.claude/` folder is detected
- **Interactive prompts**: User-friendly installation with sensible defaults

#### Security & Reliability Improvements
- **Legacy database migration**: Automatic migration from `~/axon-mcp.sqlite` to new scope-based locations
- **Secure file permissions**: Database files protected with 0600/0700 permissions on Unix systems  
- **Atomic migration**: Safe database migration with verification and backup retention
- **Cross-platform compatibility**: Enhanced Windows PowerShell and Unix shell installers

### Changed

#### Database Path Resolution
- **Breaking Change**: Default database location changed from `~/axon-mcp.sqlite` to scope-based paths:
  - Project scope: `{project-root}/.axon/axon-mcp.sqlite`
  - User scope: `{user-data-dir}/axon-mcp/dbs/{project-hash}.sqlite`
- **Automatic migration**: Legacy databases automatically migrated to new locations
- **Hash-based isolation**: User-scope databases use SHA256 project path hash for isolation

#### Installation Experience
- **Enhanced CLI**: Both shell and PowerShell installers support new scope-based installation modes
- **Improved automation**: Smart detection of Git repositories and Claude Code projects
- **Better error handling**: Clear error messages for common installation scenarios

### Fixed
- **Multi-project conflicts**: Resolved database sharing conflicts between parallel development projects
- **Test suite reliability**: All tests now pass with no warnings
- **Platform detection**: Fixed ARM64 macOS detection in self-update functionality
- **Clippy warnings**: Eliminated all linter warnings across the codebase

### Technical Details

#### Migration Strategy
- Legacy `~/axon-mcp.sqlite` automatically detected and migrated
- Migration only occurs for user-scope installations when new database doesn't exist
- Original database preserved for backup purposes with clear user instructions
- Migration marker created to prevent repeated migration attempts

#### Security Enhancements
- Database directories created with secure permissions (0700 on Unix)
- Database files protected with owner-only access (0600 on Unix)
- Windows relies on NTFS permissions for security
- Path traversal vulnerabilities eliminated through canonicalization

### Breaking Changes
- **Database Location**: Default database location changed (with automatic migration)
- **Environment Variables**: `AXON_MCP_SCOPE` now affects database path resolution

### Migration Guide
1. **Automatic Migration**: For most users, migration happens automatically on first run
2. **Manual Override**: Use `AXON_MCP_DB` environment variable to specify custom database path
3. **Scope Control**: Use `AXON_MCP_SCOPE=project` or `AXON_MCP_SCOPE=user` to control scope detection
4. **Legacy Cleanup**: After verifying migration worked, legacy `~/axon-mcp.sqlite` can be safely deleted

---

## [0.1.0] - 2025-08-03

### 🎉 Initial Release

This is the first stable release of axon-mcp, a production-ready MCP Task Management Server built in Rust.

### Added

#### Core Features
- **Cross-platform MCP Task Management Server** supporting Linux, macOS, and Windows
- **Complete MCP v2 protocol implementation** with 22 functions for comprehensive task coordination
- **Multi-agent coordination system** with task discovery, atomic claiming, and work session tracking
- **Inter-agent messaging system** with targeted communication and conversation threading
- **SQLite database backend** with automatic setup and migration support

#### Installation & Distribution
- **One-line installation scripts** for seamless setup across platforms:
  - macOS/Linux: `curl -fsSL https://raw.githubusercontent.com/janreges/axon-mcp/main/install.sh | sh`
  - Windows: `irm https://raw.githubusercontent.com/janreges/axon-mcp/main/install.ps1 | iex`
- **Self-update functionality** with automatic platform detection and secure binary replacement
- **Universal macOS binary** supporting both Intel x86_64 and Apple Silicon ARM64
- **Static Linux binaries** with musl linking for maximum compatibility
- **Native Windows support** with PowerShell installer

#### Developer Experience
- **Automatic Claude Code configuration** during installation
- **PATH management** with shell-specific configuration
- **Health check endpoints** for monitoring and verification
- **Comprehensive error handling** with detailed error messages
- **Production-ready logging** and monitoring capabilities

#### Security & Reliability
- **Atomic task claiming** preventing race conditions
- **Network timeouts and retry logic** for robust operation
- **Secure installation process** with file verification and atomic operations
- **Comprehensive input validation** and error handling
- **Database integrity** with proper transaction management

#### Documentation & Testing
- **Complete API documentation** with usage examples
- **Architecture documentation** explaining multi-crate design
- **Comprehensive test suite** with >90% coverage on critical paths
- **Integration tests** for MCP protocol compliance
- **Installation verification** with automated testing

### Technical Details

#### Supported MCP Functions (22 total)
- **Core Task Management**: create_task, update_task, set_task_state, get_task_by_id, get_task_by_code, list_tasks, assign_task, archive_task, health_check
- **Multi-Agent Coordination**: discover_work, claim_task, release_task, start_work_session, end_work_session
- **Inter-Agent Messaging**: create_task_message, get_task_messages
- **Workspace Automation**: get_setup_instructions, get_agentic_workflow_description, register_agent, get_instructions_for_main_ai_file, create_main_ai_file, get_workspace_manifest

#### System Requirements
- **Runtime**: No external dependencies (static binaries)
- **Database**: SQLite (automatically created at ~/axon-mcp.sqlite)
- **Network**: HTTP server on configurable port (default 3000)
- **Platforms**: Linux (x86_64, aarch64), macOS (Universal), Windows (x86_64)

#### Architecture
- **Multi-crate Rust workspace** with clean separation of concerns
- **Core domain models** with trait-based interfaces
- **Database abstraction layer** with SQLite implementation
- **MCP protocol handler** with Server-Sent Events transport
- **Main server binary** with configuration management

### Performance Characteristics
- **Concurrent agents**: Supports 50-200 concurrent agents comfortably
- **Latency**: Sub-10ms for task operations (create_task, claim_task)
- **Scalability**: Handles 10K+ tasks with proper database indexing
- **Memory footprint**: Lightweight server with minimal resource usage

### Breaking Changes
None (initial release)

### Known Limitations
- SQLite backend limits horizontal scaling (single-node deployment)
- Write operations are serialized (single writer limitation)
- No built-in GUI dashboard (command-line and API only)

### Migration Guide
Not applicable (initial release)

---

**Full Changelog**: https://github.com/janreges/axon-mcp/commits/v0.1.0