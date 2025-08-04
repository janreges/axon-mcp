-- Add claimed_at timestamp for task timeout support
-- This enables 15-minute timeout mechanism for claimed tasks

-- Step 1: Add claimed_at column to track when task was claimed
ALTER TABLE tasks ADD COLUMN claimed_at TIMESTAMP NULL;

-- Step 2: Create index for efficient timeout cleanup queries
CREATE INDEX idx_tasks_claimed_at ON tasks(claimed_at) WHERE claimed_at IS NOT NULL;

-- Step 3: Create composite index for timeout cleanup (InProgress tasks with old claimed_at)
CREATE INDEX idx_tasks_timeout_check ON tasks(state, claimed_at) 
WHERE state = 'InProgress' AND claimed_at IS NOT NULL;

-- Step 4: Update existing InProgress tasks to have claimed_at = inserted_at
-- This gives existing tasks a reasonable claimed_at timestamp
UPDATE tasks 
SET claimed_at = inserted_at 
WHERE state = 'InProgress' AND owner_agent_name IS NOT NULL;