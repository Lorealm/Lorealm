CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(255) NOT NULL UNIQUE,
    csrf_token VARCHAR(255) NOT NULL UNIQUE,
    csrf_rotation_count INTEGER NOT NULL DEFAULT 0,
    csrf_last_rotated_at TIMESTAMPTZ NOT NULL,
    ip_address VARCHAR(45) NOT NULL,
    user_agent TEXT NOT NULL,
    added_at TIMESTAMPTZ NOT NULL,
    last_active_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

-- Create indexes for common queries
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_session_token ON sessions(token);
CREATE INDEX idx_sessions_csrf_token ON sessions(csrf_token);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);