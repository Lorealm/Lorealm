CREATE TABLE invites (
    id         BIGINT PRIMARY KEY,
    code_hash  TEXT NOT NULL UNIQUE,
    role_id    BIGINT REFERENCES roles(id) ON DELETE CASCADE,
    created_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    max_uses   INTEGER,
    uses       INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
