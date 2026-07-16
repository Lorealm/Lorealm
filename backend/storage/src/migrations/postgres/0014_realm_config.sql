CREATE TABLE realm_config (
    realm_id            BIGINT PRIMARY KEY REFERENCES realms(id) ON DELETE CASCADE,
    user_id             BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    speech_format       TEXT,
    action_format       TEXT,
    tool_result_format  TEXT,
    state_change_format TEXT,
    state_write_format  TEXT,
    state_delete_format TEXT
);

CREATE INDEX realm_config_user_id_idx ON realm_config (user_id);
