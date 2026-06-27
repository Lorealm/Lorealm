-- Migration: create_templates_table
-- Description: Add templates table for storing user-created node graph templates

CREATE TABLE templates (
    id BIGINT PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    nodes_json JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT templates_owner_name_unique UNIQUE(owner_id, name)
);

-- Indexes for efficient querying
CREATE INDEX idx_templates_owner_id ON templates(owner_id);
CREATE INDEX idx_templates_updated_at ON templates(updated_at DESC);
CREATE INDEX idx_templates_name ON templates(name);

-- Index for fast node counting (useful for metadata)
CREATE INDEX idx_templates_nodes_json ON templates USING GIN(nodes_json);
