# MCP Task Management Server - Shared Context Operations
# This Makefile provides atomic operations for agent coordination

# Default target shows help
.PHONY: help
help:
	@echo "MCP Task Management - Agent Coordination Commands"
	@echo "================================================"
	@echo ""
	@echo "Status Operations:"
	@echo "  make status-start AGENT=name CRATE=crate    - Mark crate work started"
	@echo "  make status-complete AGENT=name CRATE=crate  - Mark crate work completed"
	@echo "  make status-blocked AGENT=name TYPE=type MSG='message' - Report blocker"
	@echo "  make status-unblocked AGENT=name TYPE=type  - Mark blocker resolved"
	@echo "  make status-custom AGENT=name CODE=code MSG='message' - Add custom status"
	@echo ""
	@echo "Interface Operations:"
	@echo "  make interface-add AGENT=name INTERFACE=name FILE=path - Share interface"
	@echo "  make interface-check INTERFACE=name         - Check if interface exists"
	@echo ""
	@echo "Decision Operations:"
	@echo "  make decision AGENT=name SUMMARY='summary' RATIONALE='why' ALTERNATIVES='other options'"
	@echo ""
	@echo "Phase Operations:"
	@echo "  make phase-complete AGENT=name PHASE=number  - Mark phase complete"
	@echo ""
	@echo "Status Checking:"
	@echo "  make check-status      - Show current project status"
	@echo "  make check-deps        - Check if dependencies are ready"
	@echo "  make check-crate CRATE=name - Check specific crate status"
	@echo ""
	@echo "Utility:"
	@echo "  make validate          - Validate all status codes in files"
	@echo "  make clean-temps       - Remove temporary files"

# Ensure required directories exist
.PHONY: init
init:
	@mkdir -p tmp

# Core function to add timestamped entry to a file
# Usage: $(call add-entry,FILE,CODE,AGENT,MESSAGE)
define add-entry
	@echo "[$(2)] $$(date +%Y-%m-%d\ %H:%M:%S) $(3): $(4)" >> $(1)
endef

# Status operations
.PHONY: status-start
status-start:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(CRATE)" || (echo "ERROR: CRATE is required"; exit 1)
	$(call add-entry,STATUS.md,$$(echo $(CRATE) | tr a-z A-Z)-START,$(AGENT),Beginning $(CRATE) crate)
	@echo "✓ Marked $(CRATE) as started by $(AGENT)"

.PHONY: status-complete
status-complete:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(CRATE)" || (echo "ERROR: CRATE is required"; exit 1)
	$(call add-entry,STATUS.md,$$(echo $(CRATE) | tr a-z A-Z)-COMPLETE,$(AGENT),$(CRATE) crate ready)
	@echo "✓ Marked $(CRATE) as complete by $(AGENT)"

.PHONY: status-blocked
status-blocked:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(TYPE)" || (echo "ERROR: TYPE is required (INTERFACE/DEPENDENCY/TEST/BUILD)"; exit 1)
	@test -n "$(MSG)" || (echo "ERROR: MSG is required"; exit 1)
	$(call add-entry,STATUS.md,BLOCKED-$(TYPE),$(AGENT),$(MSG))
	@echo "✓ Reported blocker: $(TYPE) - $(MSG)"

.PHONY: status-unblocked
status-unblocked:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(TYPE)" || (echo "ERROR: TYPE is required"; exit 1)
	$(call add-entry,STATUS.md,BLOCKED-$(TYPE)-RESOLVED,$(AGENT),Blocker resolved)
	@echo "✓ Marked $(TYPE) blocker as resolved"

# Custom status message
.PHONY: status-custom
status-custom:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(CODE)" || (echo "ERROR: CODE is required"; exit 1)
	@test -n "$(MSG)" || (echo "ERROR: MSG is required"; exit 1)
	$(call add-entry,STATUS.md,$(CODE),$(AGENT),$(MSG))
	@echo "✓ Added custom status: $(CODE)"

# Interface operations
.PHONY: interface-add
interface-add:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(INTERFACE)" || (echo "ERROR: INTERFACE is required"; exit 1)
	@test -n "$(FILE)" || (echo "ERROR: FILE is required"; exit 1)
	$(call add-entry,INTERFACES.md,INTERFACE-$(INTERFACE),$(AGENT),$(INTERFACE) trait ready)
	@echo "--- BEGIN DEFINITION ---" >> INTERFACES.md
	@cat $(FILE) >> INTERFACES.md 2>/dev/null || echo "ERROR: Could not read $(FILE)" >> INTERFACES.md
	@echo "--- END DEFINITION ---" >> INTERFACES.md
	@echo "✓ Added interface $(INTERFACE) from $(FILE)"

.PHONY: interface-check
interface-check:
	@test -n "$(INTERFACE)" || (echo "ERROR: INTERFACE is required"; exit 1)
	@if grep -q "\[INTERFACE-$(INTERFACE)\]" INTERFACES.md 2>/dev/null; then \
		echo "✓ Interface $(INTERFACE) is available"; \
	else \
		echo "✗ Interface $(INTERFACE) not found"; \
		exit 1; \
	fi

# Decision operations
.PHONY: decision
decision:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(SUMMARY)" || (echo "ERROR: SUMMARY is required"; exit 1)
	@test -n "$(RATIONALE)" || (echo "ERROR: RATIONALE is required"; exit 1)
	@test -n "$(ALTERNATIVES)" || (echo "ERROR: ALTERNATIVES is required"; exit 1)
	@NEXT_NUM=$$(grep -c "DECISION-" DECISIONS.md 2>/dev/null || echo "0"); \
	NEXT_NUM=$$((NEXT_NUM + 1)); \
	$(call add-entry,DECISIONS.md,DECISION-$$(printf "%03d" $$NEXT_NUM),$(AGENT),$(SUMMARY))
	@echo "RATIONALE: $(RATIONALE)" >> DECISIONS.md
	@echo "ALTERNATIVES: $(ALTERNATIVES)" >> DECISIONS.md
	@echo "✓ Recorded decision #$$NEXT_NUM"

# Phase operations
.PHONY: phase-complete
phase-complete:
	@test -n "$(AGENT)" || (echo "ERROR: AGENT is required"; exit 1)
	@test -n "$(PHASE)" || (echo "ERROR: PHASE is required"; exit 1)
	$(call add-entry,STATUS.md,PHASE-$(PHASE)-COMPLETE,$(AGENT),Phase $(PHASE) complete)
	@echo "✓ Marked Phase $(PHASE) as complete"

# Check operations
.PHONY: check-status
check-status:
	@echo "=== PROJECT STATUS CHECK ==="
	@echo ""
	@echo "PHASE STATUS:"
	@grep "\[PHASE-.*-COMPLETE\]" STATUS.md 2>/dev/null | tail -5 || echo "  No phases completed yet"
	@echo ""
	@echo "CRATE STATUS:"
	@for crate in CORE DATABASE PROTOCOL MOCKS SERVER; do \
		if grep -q "\[$$crate-COMPLETE\]" STATUS.md 2>/dev/null; then \
			echo "  ✓ $$crate: Complete"; \
		elif grep -q "\[$$crate-START\]" STATUS.md 2>/dev/null; then \
			echo "  ⚡ $$crate: In Progress"; \
		else \
			echo "  ⏳ $$crate: Not Started"; \
		fi \
	done
	@echo ""
	@echo "BLOCKING ISSUES:"
	@blockers=$$(grep "\[BLOCKED-" STATUS.md 2>/dev/null | grep -v "RESOLVED" | tail -5); \
	if [ -z "$$blockers" ]; then \
		echo "  No active blockers"; \
	else \
		echo "$$blockers"; \
	fi
	@echo ""
	@echo "AVAILABLE INTERFACES:"
	@grep "\[INTERFACE-" INTERFACES.md 2>/dev/null | tail -10 || echo "  No interfaces defined yet"
	@echo ""
	@echo "RECENT DECISIONS:"
	@grep "\[DECISION-" DECISIONS.md 2>/dev/null | tail -3 || echo "  No decisions recorded yet"

.PHONY: check-deps
check-deps:
	@echo "Checking dependencies..."
	@if ! grep -q "\[CORE-COMPLETE\]" STATUS.md 2>/dev/null; then \
		echo "✗ Core crate not complete - Phase 2 agents must wait"; \
		exit 1; \
	else \
		echo "✓ Core crate complete - Phase 2 agents can start"; \
	fi

.PHONY: check-crate
check-crate:
	@test -n "$(CRATE)" || (echo "ERROR: CRATE is required"; exit 1)
	@CRATE_UPPER=$$(echo $(CRATE) | tr a-z A-Z); \
	if grep -q "\[$$CRATE_UPPER-COMPLETE\]" STATUS.md 2>/dev/null; then \
		echo "✓ $(CRATE) is complete"; \
		grep "\[$$CRATE_UPPER-COMPLETE\]" STATUS.md | tail -1; \
	elif grep -q "\[$$CRATE_UPPER-START\]" STATUS.md 2>/dev/null; then \
		echo "⚡ $(CRATE) is in progress"; \
		grep "\[$$CRATE_UPPER-START\]" STATUS.md | tail -1; \
	else \
		echo "⏳ $(CRATE) not started"; \
	fi

# Validation
.PHONY: validate
validate:
	@echo "Validating status codes..."
	@VALID_CODES="CORE DATABASE PROTOCOL MOCKS SERVER PHASE BLOCKED INTERFACE DECISION INTEGRATION"; \
	for file in STATUS.md INTERFACES.md DECISIONS.md; do \
		if [ -f $$file ]; then \
			echo "Checking $$file..."; \
			grep -o '\[[A-Z-]*\]' $$file | sort | uniq | while read code; do \
				code_clean=$$(echo $$code | tr -d '[]'); \
				valid=0; \
				for prefix in $$VALID_CODES; do \
					if echo $$code_clean | grep -q "^$$prefix"; then \
						valid=1; \
						break; \
					fi \
				done; \
				if [ $$valid -eq 0 ]; then \
					echo "  WARNING: Unknown code $$code"; \
				fi \
			done \
		fi \
	done
	@echo "✓ Validation complete"

# Utility operations
.PHONY: clean-temps
clean-temps:
	@echo "Cleaning temporary files..."
	@find . -name "*.tmp" -delete 2>/dev/null || true
	@find . -name "*.log" -delete 2>/dev/null || true
	@find . -type d -name "tmp" -exec rm -rf {} + 2>/dev/null || true
	@echo "✓ Temporary files cleaned"

# Special target for control agent to check phase readiness
.PHONY: check-phase-ready
check-phase-ready:
	@if [ "$(PHASE)" = "2" ]; then \
		if grep -q "\[CORE-COMPLETE\]" STATUS.md 2>/dev/null; then \
			echo "✓ Ready for Phase 2"; \
		else \
			echo "✗ Not ready for Phase 2 - Core not complete"; \
			exit 1; \
		fi \
	elif [ "$(PHASE)" = "3" ]; then \
		completed=$$(grep -c "\[.*-COMPLETE\]" STATUS.md 2>/dev/null | grep -E "(DATABASE|PROTOCOL|MOCKS|SERVER)" | wc -l); \
		if [ "$$completed" -ge 3 ]; then \
			echo "✓ Ready for Phase 3"; \
		else \
			echo "✗ Not ready for Phase 3 - Phase 2 crates not complete"; \
			exit 1; \
		fi \
	fi

# =============================================================================
# Development and Build Tasks
# =============================================================================

# Development targets for local testing and cross-platform builds
.PHONY: dev-help install-targets check-cross test-cross build-cross lint fix-warnings pre-commit

dev-help:
	@echo ""
	@echo "=== DEVELOPMENT TASKS ==="
	@echo ""
	@echo "🔧 Setup:"
	@echo "  make install-targets  Install all Rust cross-compilation targets"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  make check-cross      Check compilation for all target platforms"
	@echo "  make test-cross       Run cross-platform compatibility tests"
	@echo "  make test             Run all tests"
	@echo ""
	@echo "🔨 Building:"
	@echo "  make build-cross      Build for all target platforms"
	@echo "  make build-release    Build release version"
	@echo ""
	@echo "🧹 Code Quality:"
	@echo "  make lint             Run clippy lints"
	@echo "  make fix-warnings     Fix automatically fixable warnings"
	@echo "  make fmt              Format code"
	@echo "  make pre-commit       Run all pre-commit checks"

# Setup targets for cross-compilation
install-targets:
	@echo "📦 Installing Rust cross-compilation targets..."
	rustup target add x86_64-unknown-linux-musl
	rustup target add aarch64-unknown-linux-musl
	rustup target add x86_64-pc-windows-msvc
	rustup target add x86_64-apple-darwin
	rustup target add aarch64-apple-darwin
	@echo "✅ All targets installed!"

# Cross-platform compilation check
check-cross:
	@echo "🌍 Cross-platform compilation check..."
	@./scripts/test-cross-platform.sh

# Comprehensive cross-platform testing
test-cross: check-cross
	@echo "✅ Cross-platform tests completed"

# Run all tests
test:
	@echo "🧪 Running all tests..."
	cargo test --workspace --all-features

# Build release version
build-release:
	@echo "🚀 Building release version..."
	cargo build --workspace --release

# Build for all target platforms
build-cross:
	@echo "🌍 Building for all platforms..."
	cargo build --target x86_64-unknown-linux-musl --bin axon-mcp --release
	cargo build --target x86_64-pc-windows-msvc --bin axon-mcp --release
	cargo build --target x86_64-apple-darwin --bin axon-mcp --release
	cargo build --target aarch64-apple-darwin --bin axon-mcp --release

# Lint code
lint:
	@echo "🔍 Running clippy lints..."
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Fix automatically fixable warnings
fix-warnings:
	@echo "🔧 Fixing warnings..."
	cargo fix --workspace --all-targets --all-features --allow-dirty
	cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

# Format code
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all

# Pre-commit checks - run this before committing!
pre-commit: fmt lint check-cross test
	@echo "✅ All pre-commit checks passed! Ready to commit."