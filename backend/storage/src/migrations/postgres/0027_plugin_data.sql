CREATE TABLE plugin_data(
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      BYTEA NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, handle, key)
);
