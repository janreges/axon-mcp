-- Update task_messages table to match TaskMessage model schema
-- This migration aligns the task_messages table with the new flexible messaging system

-- Drop the existing task_messages table and recreate it with correct schema
DROP TABLE IF EXISTS task_messages;

-- Create new task_messages table with correct structure
CREATE TABLE task_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,                    -- Task code (e.g., "FEAT-123") 
    author_agent_name TEXT NOT NULL,            -- Author of the message
    target_agent_name TEXT NULL,                -- Target agent for the message
    message_type TEXT NOT NULL,                 -- Flexible string type (e.g., "handoff", "comment")
    content TEXT NOT NULL,                      -- Message content
    reply_to_message_id INTEGER NULL,           -- For threading messages
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    CHECK (length(trim(task_code)) > 0),
    CHECK (length(trim(author_agent_name)) > 0),
    CHECK (length(trim(message_type)) > 0),
    CHECK (length(trim(content)) > 0),
    
    -- Foreign key for threading
    FOREIGN KEY (reply_to_message_id) REFERENCES task_messages(id) ON DELETE SET NULL
);

-- Create optimized indexes for messaging queries
CREATE INDEX idx_task_messages_task_code ON task_messages(task_code, created_at DESC);
CREATE INDEX idx_task_messages_author ON task_messages(author_agent_name, created_at DESC);  
CREATE INDEX idx_task_messages_target_agent ON task_messages(target_agent_name);
CREATE INDEX idx_task_messages_type ON task_messages(message_type, created_at DESC);
CREATE INDEX idx_task_messages_composite ON task_messages(task_code, author_agent_name, message_type, created_at DESC);
CREATE INDEX idx_task_messages_task_target_type ON task_messages(task_code, target_agent_name, message_type);
CREATE INDEX idx_task_messages_threading ON task_messages(reply_to_message_id) WHERE reply_to_message_id IS NOT NULL;

-- Migration complete: task_messages table updated for flexible messaging