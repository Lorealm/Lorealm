ALTER TABLE user_tools
    ADD COLUMN surface TEXT NOT NULL DEFAULT 'tool'
    CHECK (surface IN ('tool', 'command', 'both'));

CREATE INDEX user_tools_surface_idx ON user_tools (user_id, surface);
