CREATE TABLE characters (
    id                   BIGINT       PRIMARY KEY,
    user_id              BIGINT       NOT NULL,
    current_version_id   BIGINT       NOT NULL,
    lorebook_id          BIGINT,
    extensions           JSONB        NOT NULL DEFAULT '{}',
    created_at           TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX characters_user_id_idx ON characters (user_id);

CREATE TABLE character_versions (
    id              BIGINT       PRIMARY KEY,
    character_id    BIGINT       NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    version_num     INTEGER      NOT NULL,
    card_version    TEXT         NOT NULL DEFAULT '0.0.1',
    avatar_uri      TEXT,
    snapshot        JSONB        NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),

    UNIQUE (character_id, version_num)
);

CREATE INDEX character_versions_character_id_idx
    ON character_versions (character_id);

-- FK from head → versions added after both tables exist to avoid circular
-- issues during creation. current_version_id always points at an existing
-- version row; deleting the last version is not allowed while the head
-- exists (enforced by the ON DELETE RESTRICT).
ALTER TABLE characters
    ADD CONSTRAINT characters_current_version_fk
    FOREIGN KEY (current_version_id)
    REFERENCES character_versions(id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE character_tags (
    character_id  BIGINT  NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tag           TEXT    NOT NULL,
    PRIMARY KEY (character_id, tag)
);

CREATE INDEX character_tags_tag_idx ON character_tags (tag);

CREATE TABLE character_greetings (
    id             BIGINT   PRIMARY KEY,
    character_id   BIGINT   NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order  INTEGER  NOT NULL,
    text_content   TEXT     NOT NULL
);

CREATE INDEX character_greetings_character_id_idx
    ON character_greetings (character_id);

CREATE TABLE character_example_messages (
    id             BIGINT   PRIMARY KEY,
    character_id   BIGINT   NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    display_order  INTEGER  NOT NULL,
    content        TEXT     NOT NULL
);

CREATE INDEX character_example_messages_character_id_idx
    ON character_example_messages (character_id);
