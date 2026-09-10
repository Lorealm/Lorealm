-- Per-conversation author's note, surfaced by the `author_note` context source.
ALTER TABLE conversations ADD COLUMN author_note TEXT NOT NULL DEFAULT '';
