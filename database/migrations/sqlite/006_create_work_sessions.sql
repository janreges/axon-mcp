-- Add work_sessions table for tracking work time
CREATE TABLE IF NOT EXISTS work_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    agent_name TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    ended_at DATETIME,
    notes TEXT,
    productivity_score REAL,
    
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for performance
CREATE INDEX IF NOT EXISTS idx_work_sessions_task_id ON work_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_work_sessions_agent_name ON work_sessions(agent_name);
CREATE INDEX IF NOT EXISTS idx_work_sessions_started_at ON work_sessions(started_at);