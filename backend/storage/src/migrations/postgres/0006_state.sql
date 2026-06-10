CREATE TABLE enabled_lorebooks (
    user_id     BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lorebook_id BIGINT      NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    position    INT         NOT NULL,
    PRIMARY KEY (user_id, lorebook_id)
);
CREATE INDEX idx_enabled_lorebooks_order ON enabled_lorebooks (user_id, position);

CREATE TABLE user_personas (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT REFERENCES users(id) ON DELETE CASCADE,
    display_name    TEXT NOT NULL DEFAULT '',
    pseudonym       TEXT,
    avatar_uri      TEXT,
    description     TEXT,
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_persona_user_id ON user_personas(user_id);

CREATE TABLE user_settings (
    user_id              BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    active_template_id   BIGINT REFERENCES templates(id) ON DELETE SET NULL,
    active_persona_id    BIGINT REFERENCES user_personas(id) ON DELETE SET NULL
    -- TODO: active_connection_profile_id
);

-- Per-message save state
CREATE TABLE weaver_message_state (
    message_id       BIGINT PRIMARY KEY REFERENCES conversation_messages(id) ON DELETE CASCADE,
    activation       jsonb NOT NULL,  -- { "<lorebook_id>": <ActivationState>, ... }
    persistent_delta jsonb NOT NULL,  -- shared-pool delta { writes, deletes }
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE character_persona_overrides(
    character_id      BIGINT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    persona_id        BIGINT REFERENCES user_personas(id) ON DELETE CASCADE,
    user_id           BIGINT REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_character_persona_character_id ON character_persona_overrides(character_id);
CREATE INDEX idx_charcater_persona_user_id ON character_persona_overrides(user_id);
