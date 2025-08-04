-- Create workspace_contexts table for persistence of WorkspaceContext
-- This table stores entire WorkspaceContext as JSON to minimize schema complexity
-- and fully utilize existing serde implementation
CREATE TABLE IF NOT EXISTS workspace_contexts (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL, -- JSON serialized WorkspaceContext
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL, -- ISO 8601 format (e.g., "2023-10-27T10:00:00Z")
    updated_at TEXT NOT NULL  -- ISO 8601 format
);

-- Index is automatically created for PRIMARY KEY in SQLite, but adding for documentation
-- CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_id ON workspace_contexts (workspace_id);