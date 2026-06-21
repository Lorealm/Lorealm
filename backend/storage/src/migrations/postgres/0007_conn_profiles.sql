CREATE TABLE secrets (
    id          BIGINT PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL,
    label       TEXT NOT NULL,
    key_hint    TEXT NOT NULL,
    enc_version INTEGER NOT NULL,
    context     TEXT NOT NULL,
    salt        BYTEA NOT NULL,
    nonce       BYTEA NOT NULL,
    ciphertext  BYTEA NOT NULL,
    tag         BYTEA NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_secrets_user ON secrets (user_id);

CREATE TABLE connection_profiles (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_id       BIGINT REFERENCES secrets(id) ON DELETE SET NULL,
    name            TEXT NOT NULL,
    provider        TEXT NOT NULL,
    model           TEXT,
    common_params   JSONB NOT NULL DEFAULT '{}',
    provider_params JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_connection_profiles_user ON connection_profiles (user_id);
CREATE INDEX idx_connection_profiles_secret ON connection_profiles (secret_id);
