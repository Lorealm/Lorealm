CREATE TABLE lorebooks (
    id                 BIGINT PRIMARY KEY,
    owner_id           BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version_id BIGINT NOT NULL,            -- soft pointer; set within save_lorebook's txn
    name               text   NOT NULL,
    description        text   NOT NULL DEFAULT '',
    config_yaml        text   NOT NULL,            -- raw lorebook.yaml; parse via serde_yaml
    schema_version     int    NOT NULL DEFAULT 1,  -- your forward-compat column
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_lorebook_owner ON lorebooks (owner_id);

CREATE TABLE lorebook_entries (
    id          BIGINT PRIMARY KEY,
    lorebook_id BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    entry_id    text   NOT NULL,                   -- the EntryMeta.id
    source      text   NOT NULL,                   -- the FULL .weaver file (frontmatter + body)
    UNIQUE (lorebook_id, entry_id)
);

-- Version snapshots. Each save writes one row holding the full serialized
-- aggregate (config + entries) so get_at_version / restore reconstruct from JSON.
CREATE TABLE lorebook_versions (
    id           BIGINT PRIMARY KEY,               -- = the caller's new version_id
    lorebook_id  BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    version_num  int    NOT NULL,                  -- monotonic per lorebook, maps to i32
    book_version text   NOT NULL DEFAULT '',       -- derived from config_yaml `version:`
    snapshot     jsonb  NOT NULL,                  -- serde_json of the Lorebook aggregate
    created_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (lorebook_id, version_num)
);

CREATE INDEX idx_lorebook_version_book ON lorebook_versions (lorebook_id, version_num DESC);

-- Per-conversation save state (the part that changes every turn).
CREATE TABLE weaver_session (
    conversation_id  BIGINT PRIMARY KEY,
    lorebook_id      BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    activation_state jsonb NOT NULL,
    persistent_state jsonb NOT NULL,
    updated_at       timestamptz NOT NULL DEFAULT now()
);
