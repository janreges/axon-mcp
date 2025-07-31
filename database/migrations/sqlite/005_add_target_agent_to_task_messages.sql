-- Add target_agent_name column to task_messages table
-- This enables messages to be directed to specific agents

ALTER TABLE task_messages ADD COLUMN target_agent_name TEXT NULL;

-- Create index for efficient filtering by target agent
CREATE INDEX idx_task_messages_target_agent ON task_messages(target_agent_name);

-- Create composite index for common filtering patterns
CREATE INDEX idx_task_messages_task_target_type ON task_messages(task_code, target_agent_name, message_type);