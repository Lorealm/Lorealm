-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    permissions BIGINT NOT NULL DEFAULT 0,
    email TEXT,
    display_name TEXT UNIQUE,
    password TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;

-- Conversations table
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    context_type TEXT NOT NULL CHECK (context_type IN ('empty', 'character', 'scenario', 'world', 'custom')),
    context_id UUID,
    context_version BIGINT,
    context_metadata JSONB,
    sync_policy TEXT NOT NULL
);

CREATE INDEX idx_conversations_user_id ON conversations(user_id);
CREATE INDEX idx_conversations_context_id ON conversations(context_id) WHERE context_id IS NOT NULL;
CREATE INDEX idx_conversations_updated_at ON conversations(updated_at DESC);

-- Conversation metadata table
CREATE TABLE conversation_metadata (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    title TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    last_message_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(conversation_id)
);

CREATE INDEX idx_conversation_metadata_last_message_at ON conversation_metadata(last_message_at DESC);

-- Messages table (tree structure with parent relationships)
CREATE TABLE messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent_message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_parent_message_id ON messages(parent_message_id) WHERE parent_message_id IS NOT NULL;
CREATE INDEX idx_messages_created_at ON messages(created_at);

-- Active paths table (tracks current path through message tree)
CREATE TABLE active_paths (
    conversation_id UUID PRIMARY KEY REFERENCES conversations(id) ON DELETE CASCADE,
    message_ids UUID[] NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Instantiated entities table (snapshots of entities from world/character db)
CREATE TABLE instantiated_entities (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    entity_id UUID NOT NULL,
    entity_type TEXT NOT NULL,
    snapshot_data JSONB NOT NULL,
    instantiated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    modified_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX idx_instantiated_entities_conversation_id ON instantiated_entities(conversation_id);
CREATE INDEX idx_instantiated_entities_entity_id ON instantiated_entities(entity_id);
CREATE INDEX idx_instantiated_entities_entity_type ON instantiated_entities(entity_type);

-- Add some useful composite indexes
CREATE INDEX idx_messages_conversation_parent ON messages(conversation_id, parent_message_id);
CREATE INDEX idx_conversations_user_updated ON conversations(user_id, updated_at DESC);