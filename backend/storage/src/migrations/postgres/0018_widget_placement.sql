ALTER TABLE user_widgets
    ADD COLUMN placement TEXT  NOT NULL DEFAULT 'widget',
    ADD COLUMN mode      TEXT  NOT NULL DEFAULT 'latest',
    ADD COLUMN props     JSONB NOT NULL DEFAULT '[]';
