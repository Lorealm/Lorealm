CREATE TABLE character_lorebooks (
    character_id BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    lorebook_id  BIGINT NOT NULL REFERENCES lorebooks(id) ON DELETE CASCADE,
    "position"   INTEGER NOT NULL,
    PRIMARY KEY (character_id, lorebook_id)
);

CREATE INDEX character_lorebooks_character_id_idx
    ON character_lorebooks (character_id, "position");

CREATE TABLE character_tools (
    character_id BIGINT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tool_id      BIGINT NOT NULL REFERENCES user_tools(id) ON DELETE CASCADE,
    "position"   INTEGER NOT NULL,
    PRIMARY KEY (character_id, tool_id)
);

CREATE INDEX character_tools_character_id_idx
    ON character_tools (character_id, "position");

INSERT INTO character_lorebooks (character_id, lorebook_id, "position")
SELECT c.id, c.lorebook_id, 0
FROM characters c
WHERE c.lorebook_id IS NOT NULL
  AND EXISTS (SELECT 1 FROM lorebooks l WHERE l.id = c.lorebook_id);

ALTER TABLE characters DROP COLUMN lorebook_id;
