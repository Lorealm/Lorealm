CREATE TABLE blueprints (
    id          BIGINT      PRIMARY KEY,
    owner_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL DEFAULT '',
    -- WTN source, with its `#inputs` block intact.
    source      TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_blueprints_owner ON blueprints (owner_id);

-- Which blueprint a realm was expanded from.
ALTER TABLE realms ADD COLUMN origin JSONB;
