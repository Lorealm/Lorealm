ALTER TABLE realms DROP COLUMN tick;
ALTER TABLE realms DROP COLUMN state;

ALTER TABLE realms DROP CONSTRAINT realms_status_check;
UPDATE realms SET status = CASE status
    WHEN 'active' THEN 'published'
    WHEN 'paused' THEN 'draft'
    ELSE status
END;
ALTER TABLE realms ALTER COLUMN status SET DEFAULT 'draft';
ALTER TABLE realms ADD CONSTRAINT realms_status_check
    CHECK (status IN ('draft', 'published', 'archived'));

ALTER TABLE conversations ALTER COLUMN character_id DROP NOT NULL;
ALTER TABLE conversations ALTER COLUMN char_version_id DROP NOT NULL;

ALTER TABLE conversations ADD COLUMN realm_id BIGINT
    REFERENCES realms(id) ON DELETE SET NULL;

CREATE TABLE conversation_casts (
    conversation_id BIGINT PRIMARY KEY
                    REFERENCES conversations(id) ON DELETE CASCADE,
    snapshot        JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

DROP TABLE agent_observations;
DROP TABLE realm_events;
DROP TABLE realm_ticks;

CREATE TABLE realm_events (
    id              BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      BIGINT NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL,
    source          BIGINT,
    scope           TEXT   NOT NULL,
    kind            TEXT   NOT NULL CHECK (kind IN ('spoke', 'acted', 'state_changed', 'system')),
    content         TEXT   NOT NULL DEFAULT '',
    payload         JSONB  NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, seq)
);

CREATE INDEX idx_realm_events_convo ON realm_events (conversation_id, id);

CREATE TABLE agent_observations (
    id              BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      BIGINT NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    agent_id        BIGINT NOT NULL,
    event_id        BIGINT REFERENCES realm_events(id) ON DELETE SET NULL,
    role            TEXT   NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT   NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_observations_msg ON agent_observations (agent_id, message_id);
CREATE INDEX idx_agent_observations_feed ON agent_observations (conversation_id, agent_id, id);
