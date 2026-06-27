CREATE TABLE user_tools(
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL CHECK (name ~ '^[a-zA-Z0-9_-]{1,64}$'),
    display_name    TEXT,
    description     TEXT NOT NULL DEFAULT '',
    params          JSONB NOT NULL DEFAULT '[]',
    body            TEXT NOT NULL DEFAULT '',
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now(),

    UNIQUE(user_id, name)
);
