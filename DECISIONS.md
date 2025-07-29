# Architectural Decisions - MCP Task Management Server

## APPEND-ONLY FILE - NEVER OVERWRITE, ONLY APPEND WITH >>

This file records important architectural and implementation decisions.

### Decision Format:
```
[DECISION-XXX] TIMESTAMP AGENT: Brief decision summary
RATIONALE: Why this decision was made
ALTERNATIVES: What other options were considered
IMPACT: How this affects the system
```

---
## Decision Log (newest at bottom)

[DECISION-001] 2025-01-29 10:00 control-agent: Use multi-crate architecture
RATIONALE: Enables parallel development by multiple agents with clear boundaries
ALTERNATIVES: Monolithic structure, feature-based modules
IMPACT: More complex build but better separation of concerns

[DECISION-002] 2025-01-29 10:00 control-agent: SQLite only, no PostgreSQL
RATIONALE: Simplifies deployment and reduces complexity per user requirements
ALTERNATIVES: Multi-database support with PostgreSQL
IMPACT: Simpler codebase, embedded database only