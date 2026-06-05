CREATE TABLE enabled_lorebooks (
    user_id     BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lorebook_id BIGINT      NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    position    INT         NOT NULL,
    PRIMARY KEY (user_id, lorebook_id)
);
CREATE INDEX idx_enabled_lorebooks_order ON enabled_lorebooks (user_id, position);

CREATE TABLE user_settings (
    user_id            BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    active_template_id BIGINT REFERENCES templates(id) ON DELETE SET NULL
    -- TODO: active_connection_profile_id
);

-- Per-message save state
CREATE TABLE weaver_message_state (
    message_id       BIGINT PRIMARY KEY REFERENCES conversation_messages(id) ON DELETE CASCADE,
    activation       jsonb NOT NULL,  -- { "<lorebook_id>": <ActivationState>, ... }
    persistent_delta jsonb NOT NULL,  -- shared-pool delta { writes, deletes }
    created_at       timestamptz NOT NULL DEFAULT now()
);
