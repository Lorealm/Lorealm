CREATE TABLE user_plugins(
    user_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle   TEXT NOT NULL,
    position INT NOT NULL,
    PRIMARY KEY (user_id, handle)
);
