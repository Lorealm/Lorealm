CREATE TABLE plugin_data_versions(
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    version    TEXT NOT NULL,
    PRIMARY KEY (user_id, handle)
);
