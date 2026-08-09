CREATE TABLE realm_versions (
    id          BIGINT       PRIMARY KEY,
    realm_id    BIGINT       NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    version_num INTEGER      NOT NULL,
    snapshot    JSONB        NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),

    UNIQUE (realm_id, version_num)
);

CREATE INDEX realm_versions_realm_id_idx ON realm_versions (realm_id);

CREATE TABLE realm_tags (
    realm_id  BIGINT  NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    tag       TEXT    NOT NULL,
    PRIMARY KEY (realm_id, tag)
);

CREATE INDEX realm_tags_tag_idx ON realm_tags (tag);

INSERT INTO realm_versions (id, realm_id, version_num, snapshot)
SELECT
    r.id,
    r.id,
    1,
    jsonb_build_object(
        'name', r.name,
        'description', '',
        'agents', COALESCE((
            SELECT jsonb_agg(jsonb_build_object(
                'id',              a.id::text,
                'name',            a.name,
                'kind',            a.kind,
                'character_id',    a.character_id::text,
                'char_version_id', a.char_version_id::text,
                'template_id',     a.template_id::text,
                'profile_id',      a.profile_id::text,
                'user_id',         a.user_id::text,
                'persona_id',      a.persona_id::text,
                'turn_order',      a.turn_order,
                'grants', COALESCE((
                    SELECT jsonb_agg(
                        jsonb_build_object('scope', g.scope, 'access', g.access)
                        ORDER BY g.scope)
                    FROM agent_grants g WHERE g.agent_id = a.id
                ), '[]'::jsonb),
                'tools', COALESCE((
                    SELECT jsonb_agg(
                        jsonb_build_object('tool_id', t.tool_id::text, 'event_scope', t.event_scope)
                        ORDER BY t.tool_id)
                    FROM agent_tools t WHERE t.agent_id = a.id
                ), '[]'::jsonb)
            ) ORDER BY a.turn_order, a.id)
            FROM realm_agents a WHERE a.realm_id = r.id
        ), '[]'::jsonb)
    )
FROM realms r;

ALTER TABLE realms ADD COLUMN current_version_id BIGINT;
UPDATE realms SET current_version_id = id;
ALTER TABLE realms ALTER COLUMN current_version_id SET NOT NULL;

ALTER TABLE realms
    ADD CONSTRAINT realms_current_version_fk
    FOREIGN KEY (current_version_id)
    REFERENCES realm_versions(id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED;

-- The relational cast is now the version snapshot.
DROP TABLE agent_tools;
DROP TABLE agent_grants;
DROP TABLE realm_agents;

-- Name lives in the snapshot (dropping the column also drops the
-- UNIQUE(owner_id, name) constraint); status and tokenizer are gone.
ALTER TABLE realms DROP COLUMN name;
ALTER TABLE realms DROP COLUMN status;
ALTER TABLE realms DROP COLUMN tokenizer_id;
