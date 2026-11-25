-- The prompt each reply was built from, moved out of the message row. Turns
-- load whole message paths and never read it, and it holds an entry for every
-- earlier message, so it was most of a long conversation's bytes.
-- `metadata.has_context` marks the messages that have one.
CREATE TABLE message_contexts (
    message_id BIGINT PRIMARY KEY REFERENCES conversation_messages(id) ON DELETE CASCADE,
    context    JSONB NOT NULL
);

INSERT INTO message_contexts (message_id, context)
SELECT id, metadata -> 'context'
FROM conversation_messages
WHERE jsonb_typeof(metadata -> 'context') IN ('array', 'object');

UPDATE conversation_messages
SET metadata = (metadata - 'context') || '{"has_context": true}'::jsonb
WHERE jsonb_typeof(metadata -> 'context') IN ('array', 'object');

-- Anything else under the key (null, say) carries no prompt; just drop it.
UPDATE conversation_messages
SET metadata = metadata - 'context'
WHERE metadata -> 'context' IS NOT NULL;
