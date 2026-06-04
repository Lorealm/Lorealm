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
