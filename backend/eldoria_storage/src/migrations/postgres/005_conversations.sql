CREATE TABLE conversations (
    id              BIGINT      PRIMARY KEY,
    user_id         BIGINT      NOT NULL REFERENCES users(id),
    character_id    BIGINT      NOT NULL REFERENCES characters(id),
    char_version_id BIGINT      NOT NULL REFERENCES character_versions(id), -- assumed table name
    active_leaf_id  BIGINT,     -- nullable until first message; FK added below (breaks cycle)
    title           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- List a user's conversations, most-recently-updated first.
CREATE INDEX idx_conversations_user_recent
    ON conversations (user_id, updated_at DESC);

CREATE TABLE conversation_messages (
    id              BIGINT      PRIMARY KEY,
    conversation_id BIGINT      NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent          BIGINT      REFERENCES conversation_messages(id) ON DELETE CASCADE, -- NULL = root; cascade deletes the subtree
    role            TEXT        NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
    content         TEXT        NOT NULL DEFAULT '',
    "order"         INTEGER     NOT NULL, -- "order" is a SQL reserved word; must be quoted everywhere
    metadata        JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Load an entire conversation's messages.
CREATE INDEX idx_messages_conversation
    ON conversation_messages (conversation_id);

-- Fetch a node's children in order (swipe / branch navigation).
CREATE INDEX idx_messages_parent_order
    ON conversation_messages (parent, "order");

-- active_leaf_id -> conversation_messages(id). Deferred to here because the two
-- tables reference each other; the messages table must exist first.
ALTER TABLE conversations
    ADD CONSTRAINT fk_conversations_active_leaf
    FOREIGN KEY (active_leaf_id)
    REFERENCES conversation_messages(id)
    ON DELETE SET NULL;
