-- AI-assisted. Squashed from 31 migrations down to 1.
-- This represents the baseline for when SQLite support was added.

-- ===========================================================================
-- Users, roles, sessions
-- ===========================================================================

CREATE TABLE users (
    id            BIGINT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    email         TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    display_name  TEXT,
    avatar_uri    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_email ON users (email);

CREATE TABLE roles (
    id          BIGINT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    permissions BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_roles (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id     BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user_id ON user_roles (user_id);
CREATE INDEX idx_user_roles_role_id ON user_roles (role_id);

CREATE TABLE sessions (
    id                           BIGINT PRIMARY KEY,
    user_id                      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token                        VARCHAR(512) NOT NULL UNIQUE,
    csrf_token                   VARCHAR(256) NOT NULL,
    ip_address                   VARCHAR(256),
    user_agent                   VARCHAR(256),
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at                   TIMESTAMPTZ NOT NULL,
    revoked_at                   TIMESTAMPTZ,
    last_rotated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    previous_token               VARCHAR(512),
    grace_expires_at             TIMESTAMPTZ,
    last_csrf_rotated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    requests_since_csrf_rotation INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_sessions_token ON sessions (token);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- ===========================================================================
-- Personas
-- ===========================================================================

CREATE TABLE user_personas (
    id           BIGINT PRIMARY KEY,
    user_id      BIGINT REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL DEFAULT '',
    display_name TEXT,
    pseudonym    TEXT,
    avatar_uri   TEXT,
    description  TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_persona_user_id ON user_personas (user_id);

-- ===========================================================================
-- Secrets and connection profiles
-- ===========================================================================

CREATE TABLE secrets (
    id          BIGINT PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL,
    label       TEXT NOT NULL,
    key_hint    TEXT NOT NULL,
    enc_version INTEGER NOT NULL,
    context     TEXT NOT NULL,
    salt        BYTEA NOT NULL,
    nonce       BYTEA NOT NULL,
    ciphertext  BYTEA NOT NULL,
    tag         BYTEA NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_secrets_user ON secrets (user_id);

CREATE TABLE connection_profiles (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_id       BIGINT REFERENCES secrets(id) ON DELETE SET NULL,
    name            TEXT NOT NULL,
    provider        TEXT NOT NULL,
    model           TEXT,
    token_budget    INTEGER NOT NULL DEFAULT 0,
    common_params   JSONB NOT NULL DEFAULT '{}'::jsonb,
    provider_params JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_connection_profiles_user ON connection_profiles (user_id);
CREATE INDEX idx_connection_profiles_secret ON connection_profiles (secret_id);

-- ===========================================================================
-- Templates
-- ===========================================================================

CREATE TABLE templates (
    id          BIGINT PRIMARY KEY,
    owner_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    nodes_json  JSONB NOT NULL,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT templates_owner_name_unique UNIQUE (owner_id, name)
);

CREATE INDEX idx_templates_owner_id ON templates (owner_id);
CREATE INDEX idx_templates_name ON templates (name);
CREATE INDEX idx_templates_updated_at ON templates (updated_at DESC);
CREATE INDEX idx_templates_nodes_json ON templates USING GIN (nodes_json);

-- ===========================================================================
-- Tokenizers
-- ===========================================================================

CREATE TABLE tokenizers (
    id        BIGINT PRIMARY KEY,
    name      TEXT NOT NULL,
    blob_uri  TEXT NOT NULL,
    owner_id  BIGINT NOT NULL REFERENCES users(id),
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (owner_id, name)
);

CREATE TABLE tokenizer_shares (
    tok_id  BIGINT NOT NULL REFERENCES tokenizers(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (tok_id, user_id)
);

-- ===========================================================================
-- Lorebooks
--
-- `lorebooks.current_version_id` intentionally carries no foreign key; the
-- application writes the version row and the pointer in one transaction.
-- ===========================================================================

CREATE TABLE lorebooks (
    id                 BIGINT PRIMARY KEY,
    owner_id           BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version_id BIGINT NOT NULL,
    name               TEXT NOT NULL,
    description        TEXT NOT NULL DEFAULT '',
    config_yaml        TEXT NOT NULL,
    schema_version     INTEGER NOT NULL DEFAULT 1,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_lorebook_owner ON lorebooks (owner_id);

CREATE TABLE lorebook_versions (
    id           BIGINT PRIMARY KEY,
    lorebook_id  BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    version_num  INTEGER NOT NULL,
    book_version TEXT NOT NULL DEFAULT '',
    snapshot     JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (lorebook_id, version_num)
);

CREATE INDEX idx_lorebook_version_book ON lorebook_versions (lorebook_id, version_num DESC);

CREATE TABLE lorebook_entries (
    id          BIGINT PRIMARY KEY,
    lorebook_id BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL,
    source      TEXT NOT NULL,
    UNIQUE (lorebook_id, entry_id)
);

CREATE TABLE enabled_lorebooks (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lorebook_id BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    "position"  INTEGER NOT NULL,
    PRIMARY KEY (user_id, lorebook_id)
);

CREATE INDEX idx_enabled_lorebooks_order ON enabled_lorebooks (user_id, "position");

-- ===========================================================================
-- Widgets
-- ===========================================================================

CREATE TABLE user_widgets (
    id          BIGINT PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT '',
    prompt      TEXT NOT NULL DEFAULT '',
    render      TEXT NOT NULL DEFAULT '',
    tools       JSONB NOT NULL DEFAULT '[]'::jsonb,
    props       JSONB NOT NULL DEFAULT '[]'::jsonb,
    placement   TEXT NOT NULL DEFAULT 'widget',
    mode        TEXT NOT NULL DEFAULT 'latest',
    enabled     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ===========================================================================
-- Characters
--
-- characters <-> character_versions is a reference cycle: the FK on
-- current_version_id is deferred so both rows can be inserted in one
-- transaction.
-- ===========================================================================

CREATE TABLE characters (
    id                 BIGINT PRIMARY KEY,
    user_id            BIGINT NOT NULL, -- no FK, matching the existing schema
    current_version_id BIGINT NOT NULL,
    lorebook_id        BIGINT,
    extensions         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX characters_user_id_idx ON characters (user_id);

CREATE TABLE character_versions (
    id           BIGINT PRIMARY KEY,
    character_id BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    version_num  INTEGER NOT NULL,
    card_version TEXT NOT NULL DEFAULT '0.0.1',
    avatar_uri   TEXT,
    snapshot     JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (character_id, version_num)
);

CREATE INDEX character_versions_character_id_idx ON character_versions (character_id);

ALTER TABLE characters
    ADD CONSTRAINT characters_current_version_fk
    FOREIGN KEY (current_version_id)
    REFERENCES character_versions(id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE character_tags (
    character_id BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tag          TEXT NOT NULL,
    PRIMARY KEY (character_id, tag)
);

CREATE INDEX character_tags_tag_idx ON character_tags (tag);

CREATE TABLE character_greetings (
    id            BIGINT PRIMARY KEY,
    character_id  BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL,
    text_content  TEXT NOT NULL
);

CREATE INDEX character_greetings_character_id_idx ON character_greetings (character_id);

CREATE TABLE character_example_messages (
    id            BIGINT PRIMARY KEY,
    character_id  BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL,
    content       TEXT NOT NULL
);

CREATE INDEX character_example_messages_character_id_idx
    ON character_example_messages (character_id);

CREATE TABLE character_persona_overrides (
    character_id BIGINT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    persona_id   BIGINT REFERENCES user_personas(id) ON DELETE CASCADE,
    user_id      BIGINT REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_character_persona_character_id
    ON character_persona_overrides (character_id);
CREATE INDEX idx_character_persona_user_id
    ON character_persona_overrides (user_id);

CREATE TABLE character_widgets (
    character_id BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    widget_id    BIGINT NOT NULL REFERENCES user_widgets(id) ON DELETE CASCADE,
    "position"   INTEGER NOT NULL,
    PRIMARY KEY (character_id, widget_id)
);

CREATE INDEX character_widgets_character_id_idx
    ON character_widgets (character_id, "position");

-- ===========================================================================
-- Realms
--
-- Same deferred-cycle shape as characters. Agent and tool-scope rows were
-- folded into `realm_versions.snapshot` by the realm refactor.
-- ===========================================================================

CREATE TABLE realms (
    id                 BIGINT PRIMARY KEY,
    owner_id           BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version_id BIGINT NOT NULL,
    origin             JSONB,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_realms_owner ON realms (owner_id);

CREATE TABLE realm_versions (
    id          BIGINT PRIMARY KEY,
    realm_id    BIGINT NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    version_num INTEGER NOT NULL,
    snapshot    JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (realm_id, version_num)
);

CREATE INDEX realm_versions_realm_id_idx ON realm_versions (realm_id);

ALTER TABLE realms
    ADD CONSTRAINT realms_current_version_fk
    FOREIGN KEY (current_version_id)
    REFERENCES realm_versions(id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE realm_tags (
    realm_id BIGINT NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    tag      TEXT NOT NULL,
    PRIMARY KEY (realm_id, tag)
);

CREATE INDEX realm_tags_tag_idx ON realm_tags (tag);

-- ===========================================================================
-- Conversations
--
-- conversations <-> conversation_messages is also a cycle (active_leaf_id),
-- resolved with a plain FK added after both tables exist.
-- ===========================================================================

CREATE TABLE conversations (
    id              BIGINT PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id),
    character_id    BIGINT REFERENCES characters(id),
    char_version_id BIGINT REFERENCES character_versions(id),
    realm_id        BIGINT REFERENCES realms(id) ON DELETE SET NULL,
    active_leaf_id  BIGINT,
    title           TEXT,
    author_note     TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- List a user's conversations, most-recently-updated first.
CREATE INDEX idx_conversations_user_recent
    ON conversations (user_id, updated_at DESC);

CREATE TABLE conversation_messages (
    id              BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent          BIGINT REFERENCES conversation_messages(id) ON DELETE CASCADE, -- NULL = root; cascade deletes the subtree
    role            TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT NOT NULL DEFAULT '',
    "order"         INTEGER NOT NULL, -- "order" is a SQL reserved word; must be quoted everywhere
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Load an entire conversation's messages.
CREATE INDEX idx_messages_conversation
    ON conversation_messages (conversation_id);

-- Fetch a node's children in order (swipe / branch navigation).
CREATE INDEX idx_messages_parent_order
    ON conversation_messages (parent, "order");

ALTER TABLE conversations
    ADD CONSTRAINT fk_conversations_active_leaf
    FOREIGN KEY (active_leaf_id)
    REFERENCES conversation_messages(id)
    ON DELETE SET NULL;

CREATE TABLE conversation_casts (
    conversation_id BIGINT PRIMARY KEY REFERENCES conversations(id) ON DELETE CASCADE,
    snapshot        JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ===========================================================================
-- Realm event log and per-agent observation feeds
-- ===========================================================================

CREATE TABLE realm_events (
    id              BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      BIGINT NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL,
    source          TEXT,
    channel         TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('spoke', 'acted', 'state_changed', 'system')),
    content         TEXT NOT NULL DEFAULT '',
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, seq)
);

CREATE INDEX idx_realm_events_convo ON realm_events (conversation_id, id);

CREATE TABLE agent_observations (
    id              BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      BIGINT NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    agent_handle    TEXT NOT NULL,
    event_id        BIGINT REFERENCES realm_events(id) ON DELETE SET NULL,
    role            TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_observations_feed
    ON agent_observations (conversation_id, agent_handle, id);
CREATE INDEX idx_agent_observations_msg
    ON agent_observations (agent_handle, message_id);

-- ===========================================================================
-- Weaver state
-- ===========================================================================

CREATE TABLE weaver_message_state (
    message_id       BIGINT PRIMARY KEY REFERENCES conversation_messages(id) ON DELETE CASCADE,
    activation       JSONB NOT NULL,
    persistent_delta JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ===========================================================================
-- Per-user active selections
-- ===========================================================================

CREATE TABLE user_settings (
    user_id                      BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    active_template_id           BIGINT REFERENCES templates(id) ON DELETE SET NULL,
    active_persona_id            BIGINT REFERENCES user_personas(id) ON DELETE SET NULL,
    active_connection_profile_id BIGINT REFERENCES connection_profiles(id) ON DELETE SET NULL,
    active_tokenizer_id          BIGINT REFERENCES tokenizers(id) ON DELETE SET NULL
);

CREATE INDEX idx_user_settings_user ON user_settings (user_id);

-- ===========================================================================
-- User tools
-- ===========================================================================

CREATE TABLE user_tools (
    id           BIGINT PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL CHECK (name ~ '^[a-zA-Z0-9_-]{1,64}$'),
    display_name TEXT,
    description  TEXT NOT NULL DEFAULT '',
    params       JSONB NOT NULL DEFAULT '[]'::jsonb,
    body         TEXT NOT NULL DEFAULT '',
    surface      TEXT NOT NULL DEFAULT 'tool' CHECK (surface IN ('tool', 'command', 'both')),
    enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, name)
);

CREATE INDEX user_tools_surface_idx ON user_tools (user_id, surface);

-- ===========================================================================
-- Plugins
-- ===========================================================================

CREATE TABLE user_plugins (
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    "position" INTEGER NOT NULL,
    version    TEXT NOT NULL,
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_data (
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, handle, key)
);

CREATE TABLE plugin_data_versions (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle  TEXT NOT NULL,
    version TEXT NOT NULL,
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_settings (
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    settings   JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_logs (
    id         BIGINT PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    level      TEXT NOT NULL,
    source     TEXT NOT NULL,
    message    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX plugin_logs_user_handle ON plugin_logs (user_id, handle, id);

-- ===========================================================================
-- Blueprints
-- ===========================================================================

CREATE TABLE blueprints (
    id          BIGINT PRIMARY KEY,
    owner_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_blueprints_owner ON blueprints (owner_id);
