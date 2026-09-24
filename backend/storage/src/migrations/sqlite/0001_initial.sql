-- AI-assisted. SQLite counterpart of migrations/postgres/0001_initial.sql.

CREATE TABLE users (
    id            INTEGER PRIMARY KEY NOT NULL,
    username      TEXT NOT NULL UNIQUE,
    email         TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    display_name  TEXT,
    avatar_uri    TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_email ON users (email);

CREATE TABLE roles (
    id          INTEGER PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    permissions INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE TABLE user_roles (
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id     INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user_id ON user_roles (user_id);
CREATE INDEX idx_user_roles_role_id ON user_roles (role_id);

CREATE TABLE sessions (
    id                           INTEGER PRIMARY KEY NOT NULL,
    user_id                      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token                        TEXT NOT NULL UNIQUE,
    csrf_token                   TEXT NOT NULL,
    ip_address                   TEXT,
    user_agent                   TEXT,
    created_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    expires_at                   TEXT NOT NULL,
    revoked_at                   TEXT,
    last_rotated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    previous_token               TEXT,
    grace_expires_at             TEXT,
    last_csrf_rotated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    requests_since_csrf_rotation INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_sessions_token ON sessions (token);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- ===========================================================================
-- Personas
-- ===========================================================================

CREATE TABLE user_personas (
    id           INTEGER PRIMARY KEY NOT NULL,
    user_id      INTEGER REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL DEFAULT '',
    display_name TEXT,
    pseudonym    TEXT,
    avatar_uri   TEXT,
    description  TEXT,
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_user_persona_user_id ON user_personas (user_id);

-- ===========================================================================
-- Secrets and connection profiles
-- ===========================================================================

CREATE TABLE secrets (
    id          INTEGER PRIMARY KEY NOT NULL,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL,
    label       TEXT NOT NULL,
    key_hint    TEXT NOT NULL,
    enc_version INTEGER NOT NULL,
    context     TEXT NOT NULL,
    salt        BLOB NOT NULL,
    nonce       BLOB NOT NULL,
    ciphertext  BLOB NOT NULL,
    tag         BLOB NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_secrets_user ON secrets (user_id);

CREATE TABLE connection_profiles (
    id              INTEGER PRIMARY KEY NOT NULL,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_id       INTEGER REFERENCES secrets(id) ON DELETE SET NULL,
    name            TEXT NOT NULL,
    provider        TEXT NOT NULL,
    model           TEXT,
    token_budget    INTEGER NOT NULL DEFAULT 0,
    common_params   TEXT NOT NULL DEFAULT '{}',
    provider_params TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_connection_profiles_user ON connection_profiles (user_id);
CREATE INDEX idx_connection_profiles_secret ON connection_profiles (secret_id);

-- ===========================================================================
-- Templates
-- ===========================================================================

CREATE TABLE templates (
    id          INTEGER PRIMARY KEY NOT NULL,
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    nodes_json  TEXT NOT NULL,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    CONSTRAINT templates_owner_name_unique UNIQUE (owner_id, name)
);

CREATE INDEX idx_templates_owner_id ON templates (owner_id);
CREATE INDEX idx_templates_name ON templates (name);
CREATE INDEX idx_templates_updated_at ON templates (updated_at DESC);
-- No counterpart to the Postgres GIN index on nodes_json; SQLite has no
-- inverted JSON index. Nothing queries into nodes_json today.

-- ===========================================================================
-- Tokenizers
-- ===========================================================================

CREATE TABLE tokenizers (
    id        INTEGER PRIMARY KEY NOT NULL,
    name      TEXT NOT NULL,
    blob_uri  TEXT NOT NULL,
    owner_id  INTEGER NOT NULL REFERENCES users(id),
    is_public BOOLEAN NOT NULL DEFAULT 0,
    UNIQUE (owner_id, name)
);

CREATE TABLE tokenizer_shares (
    tok_id  INTEGER NOT NULL REFERENCES tokenizers(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (tok_id, user_id)
);

-- ===========================================================================
-- Lorebooks
--
-- `lorebooks.current_version_id` intentionally carries no foreign key; the
-- application writes the version row and the pointer in one transaction.
-- ===========================================================================

CREATE TABLE lorebooks (
    id                 INTEGER PRIMARY KEY NOT NULL,
    owner_id           INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version_id INTEGER NOT NULL,
    name               TEXT NOT NULL,
    description        TEXT NOT NULL DEFAULT '',
    config_yaml        TEXT NOT NULL,
    schema_version     INTEGER NOT NULL DEFAULT 1,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_lorebook_owner ON lorebooks (owner_id);

CREATE TABLE lorebook_versions (
    id           INTEGER PRIMARY KEY NOT NULL,
    lorebook_id  INTEGER NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    version_num  INTEGER NOT NULL,
    book_version TEXT NOT NULL DEFAULT '',
    snapshot     TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    UNIQUE (lorebook_id, version_num)
);

CREATE INDEX idx_lorebook_version_book ON lorebook_versions (lorebook_id, version_num DESC);

CREATE TABLE lorebook_entries (
    id          INTEGER PRIMARY KEY NOT NULL,
    lorebook_id INTEGER NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL,
    source      TEXT NOT NULL,
    UNIQUE (lorebook_id, entry_id)
);

CREATE TABLE enabled_lorebooks (
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lorebook_id INTEGER NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    "position"  INTEGER NOT NULL,
    PRIMARY KEY (user_id, lorebook_id)
);

CREATE INDEX idx_enabled_lorebooks_order ON enabled_lorebooks (user_id, "position");

-- ===========================================================================
-- Widgets
-- ===========================================================================

CREATE TABLE user_widgets (
    id          INTEGER PRIMARY KEY NOT NULL,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT '',
    prompt      TEXT NOT NULL DEFAULT '',
    render      TEXT NOT NULL DEFAULT '',
    tools       TEXT NOT NULL DEFAULT '[]',
    props       TEXT NOT NULL DEFAULT '[]',
    placement   TEXT NOT NULL DEFAULT 'widget',
    mode        TEXT NOT NULL DEFAULT 'latest',
    enabled     BOOLEAN NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

-- ===========================================================================
-- Characters
--
-- characters <-> character_versions is a reference cycle: the FK on
-- current_version_id is deferred so both rows can be inserted in one
-- transaction.
-- ===========================================================================

CREATE TABLE characters (
    id                 INTEGER PRIMARY KEY NOT NULL,
    user_id            INTEGER NOT NULL, -- no FK, matching the existing schema
    current_version_id INTEGER NOT NULL
                       REFERENCES character_versions(id)
                       ON DELETE RESTRICT
                       DEFERRABLE INITIALLY DEFERRED,
    lorebook_id        INTEGER,
    extensions         TEXT NOT NULL DEFAULT '{}',
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX characters_user_id_idx ON characters (user_id);

CREATE TABLE character_versions (
    id           INTEGER PRIMARY KEY NOT NULL,
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    version_num  INTEGER NOT NULL,
    card_version TEXT NOT NULL DEFAULT '0.0.1',
    avatar_uri   TEXT,
    snapshot     TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    UNIQUE (character_id, version_num)
);

CREATE INDEX character_versions_character_id_idx ON character_versions (character_id);

CREATE TABLE character_tags (
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tag          TEXT NOT NULL,
    PRIMARY KEY (character_id, tag)
);

CREATE INDEX character_tags_tag_idx ON character_tags (tag);

CREATE TABLE character_greetings (
    id            INTEGER PRIMARY KEY NOT NULL,
    character_id  INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL,
    text_content  TEXT NOT NULL
);

CREATE INDEX character_greetings_character_id_idx ON character_greetings (character_id);

CREATE TABLE character_example_messages (
    id            INTEGER PRIMARY KEY NOT NULL,
    character_id  INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL,
    content       TEXT NOT NULL
);

CREATE INDEX character_example_messages_character_id_idx
    ON character_example_messages (character_id);

CREATE TABLE character_persona_overrides (
    character_id INTEGER PRIMARY KEY NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    persona_id   INTEGER REFERENCES user_personas(id) ON DELETE CASCADE,
    user_id      INTEGER REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_character_persona_character_id
    ON character_persona_overrides (character_id);
CREATE INDEX idx_character_persona_user_id
    ON character_persona_overrides (user_id);

CREATE TABLE character_widgets (
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    widget_id    INTEGER NOT NULL REFERENCES user_widgets(id) ON DELETE CASCADE,
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
    id                 INTEGER PRIMARY KEY NOT NULL,
    owner_id           INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version_id INTEGER NOT NULL
                       REFERENCES realm_versions(id)
                       ON DELETE RESTRICT
                       DEFERRABLE INITIALLY DEFERRED,
    origin             TEXT,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_realms_owner ON realms (owner_id);

CREATE TABLE realm_versions (
    id          INTEGER PRIMARY KEY NOT NULL,
    realm_id    INTEGER NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    version_num INTEGER NOT NULL,
    snapshot    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    UNIQUE (realm_id, version_num)
);

CREATE INDEX realm_versions_realm_id_idx ON realm_versions (realm_id);

CREATE TABLE realm_tags (
    realm_id INTEGER NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    tag      TEXT NOT NULL,
    PRIMARY KEY (realm_id, tag)
);

CREATE INDEX realm_tags_tag_idx ON realm_tags (tag);

-- ===========================================================================
-- Conversations
--
-- conversations <-> conversation_messages is also a cycle (active_leaf_id),
-- inlined here as a forward reference.
-- ===========================================================================

CREATE TABLE conversations (
    id              INTEGER PRIMARY KEY NOT NULL,
    user_id         INTEGER NOT NULL REFERENCES users(id),
    character_id    INTEGER REFERENCES characters(id),
    char_version_id INTEGER REFERENCES character_versions(id),
    realm_id        INTEGER REFERENCES realms(id) ON DELETE SET NULL,
    active_leaf_id  INTEGER REFERENCES conversation_messages(id) ON DELETE SET NULL,
    title           TEXT,
    author_note     TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

-- List a user's conversations, most-recently-updated first.
CREATE INDEX idx_conversations_user_recent
    ON conversations (user_id, updated_at DESC);

CREATE TABLE conversation_messages (
    id              INTEGER PRIMARY KEY NOT NULL,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent          INTEGER REFERENCES conversation_messages(id) ON DELETE CASCADE, -- NULL = root; cascade deletes the subtree
    role            TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT NOT NULL DEFAULT '',
    "order"         INTEGER NOT NULL, -- "order" is a SQL reserved word; must be quoted everywhere
    metadata        TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

-- Load an entire conversation's messages.
CREATE INDEX idx_messages_conversation
    ON conversation_messages (conversation_id);

-- Fetch a node's children in order (swipe / branch navigation).
CREATE INDEX idx_messages_parent_order
    ON conversation_messages (parent, "order");

CREATE TABLE conversation_casts (
    conversation_id INTEGER PRIMARY KEY NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    snapshot        TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

-- ===========================================================================
-- Realm event log and per-agent observation feeds
-- ===========================================================================

CREATE TABLE realm_events (
    id              INTEGER PRIMARY KEY NOT NULL,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL,
    source          TEXT,
    channel         TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('spoke', 'acted', 'state_changed', 'system')),
    content         TEXT NOT NULL DEFAULT '',
    payload         TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    UNIQUE (message_id, seq)
);

CREATE INDEX idx_realm_events_convo ON realm_events (conversation_id, id);

CREATE TABLE agent_observations (
    id              INTEGER PRIMARY KEY NOT NULL,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id      INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    agent_handle    TEXT NOT NULL,
    event_id        INTEGER REFERENCES realm_events(id) ON DELETE SET NULL,
    role            TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_agent_observations_feed
    ON agent_observations (conversation_id, agent_handle, id);
CREATE INDEX idx_agent_observations_msg
    ON agent_observations (agent_handle, message_id);

-- ===========================================================================
-- Weaver state
-- ===========================================================================

CREATE TABLE weaver_message_state (
    message_id       INTEGER PRIMARY KEY NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    activation       TEXT NOT NULL,
    persistent_delta TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

-- ===========================================================================
-- Per-user active selections
-- ===========================================================================

CREATE TABLE user_settings (
    user_id                      INTEGER PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    active_template_id           INTEGER REFERENCES templates(id) ON DELETE SET NULL,
    active_persona_id            INTEGER REFERENCES user_personas(id) ON DELETE SET NULL,
    active_connection_profile_id INTEGER REFERENCES connection_profiles(id) ON DELETE SET NULL,
    active_tokenizer_id          INTEGER REFERENCES tokenizers(id) ON DELETE SET NULL
);

CREATE INDEX idx_user_settings_user ON user_settings (user_id);

-- ===========================================================================
-- User tools
--
-- The Postgres baseline validates `name` with the regex '^[a-zA-Z0-9_-]{1,64}$'.
-- SQLite has no regex operator, so the equivalent is expressed as a length
-- bound plus a GLOB that rejects any character outside the allowed set.
-- ===========================================================================

CREATE TABLE user_tools (
    id           INTEGER PRIMARY KEY NOT NULL,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL CHECK (
                     length(name) BETWEEN 1 AND 64
                     AND name NOT GLOB '*[^a-zA-Z0-9_-]*'
                 ),
    display_name TEXT,
    description  TEXT NOT NULL DEFAULT '',
    params       TEXT NOT NULL DEFAULT '[]',
    body         TEXT NOT NULL DEFAULT '',
    surface      TEXT NOT NULL DEFAULT 'tool' CHECK (surface IN ('tool', 'command', 'both')),
    enabled      BOOLEAN NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    UNIQUE (user_id, name)
);

CREATE INDEX user_tools_surface_idx ON user_tools (user_id, surface);

-- ===========================================================================
-- Plugins
-- ===========================================================================

CREATE TABLE user_plugins (
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    "position" INTEGER NOT NULL,
    version    TEXT NOT NULL,
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_data (
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    PRIMARY KEY (user_id, handle, key)
);

CREATE TABLE plugin_data_versions (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle  TEXT NOT NULL,
    version TEXT NOT NULL,
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_settings (
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    settings   TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    PRIMARY KEY (user_id, handle)
);

CREATE TABLE plugin_logs (
    id         INTEGER PRIMARY KEY NOT NULL,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    level      TEXT NOT NULL,
    source     TEXT NOT NULL,
    message    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX plugin_logs_user_handle ON plugin_logs (user_id, handle, id);

-- ===========================================================================
-- Blueprints
-- ===========================================================================

CREATE TABLE blueprints (
    id          INTEGER PRIMARY KEY NOT NULL,
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
);

CREATE INDEX idx_blueprints_owner ON blueprints (owner_id);
