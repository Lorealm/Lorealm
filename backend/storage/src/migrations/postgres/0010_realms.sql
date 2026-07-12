CREATE TABLE realms (
    id           BIGINT PRIMARY KEY,
    owner_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT   NOT NULL,
    tick         BIGINT NOT NULL DEFAULT 0,
    state        JSONB  NOT NULL DEFAULT '{}',
    tokenizer_id BIGINT NOT NULL,
    status       TEXT   NOT NULL DEFAULT 'active'
                 CHECK (status IN ('active', 'paused', 'archived')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, name)
);

CREATE INDEX idx_realms_owner ON realms (owner_id);

CREATE TABLE realm_agents (
    id              BIGINT PRIMARY KEY,
    realm_id        BIGINT NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    name            TEXT   NOT NULL,
    kind            TEXT   NOT NULL CHECK (kind IN ('llm', 'player')),
    character_id    BIGINT REFERENCES characters(id),
    char_version_id BIGINT REFERENCES character_versions(id),
    template_id     BIGINT REFERENCES templates(id),
    profile_id      BIGINT REFERENCES connection_profiles(id),
    user_id         BIGINT REFERENCES users(id),
    persona_id      BIGINT REFERENCES user_personas(id),
    turn_order      INTEGER NOT NULL DEFAULT 0,
    extensions      JSONB  NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (realm_id, name)
);

CREATE INDEX idx_realm_agents_realm ON realm_agents (realm_id);

CREATE TABLE agent_grants (
    agent_id BIGINT NOT NULL REFERENCES realm_agents(id) ON DELETE CASCADE,
    scope    TEXT   NOT NULL,
    access   TEXT   NOT NULL DEFAULT 'read' CHECK (access IN ('read', 'read_write')),
    PRIMARY KEY (agent_id, scope)
);

CREATE TABLE agent_tools (
    agent_id BIGINT NOT NULL REFERENCES realm_agents(id) ON DELETE CASCADE,
    tool_id  BIGINT NOT NULL REFERENCES user_tools(id)   ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tool_id)
);

CREATE TABLE realm_ticks (
    realm_id    BIGINT NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    tick        BIGINT NOT NULL,
    agent_id    BIGINT REFERENCES realm_agents(id) ON DELETE SET NULL,
    state_delta JSONB  NOT NULL,
    snapshot    JSONB,
    outcome     JSONB  NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (realm_id, tick)
);

CREATE INDEX idx_realm_ticks_snapshots
    ON realm_ticks (realm_id, tick DESC) WHERE snapshot IS NOT NULL;

CREATE TABLE realm_events (
    id           BIGINT PRIMARY KEY,
    realm_id     BIGINT NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    tick         BIGINT NOT NULL,
    seq          INTEGER NOT NULL,
    source       BIGINT REFERENCES realm_agents(id) ON DELETE SET NULL,
    scope        TEXT   NOT NULL,
    kind         TEXT   NOT NULL CHECK (kind IN ('spoke', 'acted', 'state_changed', 'system')),
    content      TEXT   NOT NULL DEFAULT '',
    payload      JSONB  NOT NULL DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (realm_id, tick, seq)
);

CREATE INDEX idx_realm_events_realm_tick ON realm_events (realm_id, tick);

CREATE TABLE agent_observations (
    id         BIGINT PRIMARY KEY,
    agent_id   BIGINT NOT NULL REFERENCES realm_agents(id) ON DELETE CASCADE,
    event_id   BIGINT REFERENCES realm_events(id) ON DELETE SET NULL,
    tick       BIGINT NOT NULL,
    role       TEXT   NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content    TEXT   NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_observations_log ON agent_observations (agent_id, id);