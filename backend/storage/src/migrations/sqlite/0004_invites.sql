CREATE TABLE invites (
    id         INTEGER PRIMARY KEY NOT NULL,
    code_hash  TEXT NOT NULL UNIQUE,
    role_id    INTEGER REFERENCES roles(id) ON DELETE CASCADE,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    max_uses   INTEGER,
    uses       INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);
