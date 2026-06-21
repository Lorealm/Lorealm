CREATE TABLE tokenizers (
    id          BIGINT PRIMARY KEY,
    name        TEXT NOT NULL,
    blob_uri    TEXT NOT NULL,
    owner_id    BIGINT NOT NULL REFERENCES users(id),
    is_public   BOOLEAN NOT NULL DEFAULT false,
    UNIQUE (owner_id, name)
);

CREATE TABLE tokenizer_shares (
    tok_id   BIGINT NOT NULL REFERENCES tokenizers(id) ON DELETE CASCADE,
    user_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (tok_id, user_id)
);

CREATE TABLE user_settings (
    user_id                      BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    active_template_id           BIGINT REFERENCES templates(id) ON DELETE SET NULL,
    active_persona_id            BIGINT REFERENCES user_personas(id) ON DELETE SET NULL,
    active_connection_profile_id BIGINT REFERENCES connection_profiles(id) ON DELETE SET NULL,
    active_tokenizer_id          BIGINT REFERENCES tokenizers(id) ON DELETE SET NULL
);

CREATE INDEX idx_user_settings_user ON user_settings (user_id);
