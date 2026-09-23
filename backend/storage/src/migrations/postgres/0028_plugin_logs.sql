CREATE TABLE plugin_logs(
    id         BIGINT PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    level      TEXT NOT NULL,
    source     TEXT NOT NULL,
    message    TEXT NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX plugin_logs_user_handle ON plugin_logs(user_id, handle, id);
