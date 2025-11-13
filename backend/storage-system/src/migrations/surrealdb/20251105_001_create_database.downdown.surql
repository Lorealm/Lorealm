-- SurrealDB Migration Rollback: Initial Schema
-- Version: 001
-- Description: Remove all tables created in the initial migration

-- Remove tables in reverse order to respect dependencies
REMOVE TABLE instantiated_entities;
REMOVE TABLE active_paths;
REMOVE TABLE messages;
REMOVE TABLE conversation_metadata;
REMOVE TABLE conversations;
REMOVE TABLE users;
