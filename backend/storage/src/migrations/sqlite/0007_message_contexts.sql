CREATE TABLE message_contexts (
    message_id INTEGER PRIMARY KEY NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    context    TEXT NOT NULL
);

INSERT INTO message_contexts (message_id, context)
SELECT id, json_extract(metadata, '$.context')
FROM conversation_messages
WHERE json_type(metadata, '$.context') IN ('array', 'object');

UPDATE conversation_messages
SET metadata = json_set(json_remove(metadata, '$.context'), '$.has_context', json('true'))
WHERE json_type(metadata, '$.context') IN ('array', 'object');

UPDATE conversation_messages
SET metadata = json_remove(metadata, '$.context')
WHERE json_type(metadata, '$.context') IS NOT NULL;
