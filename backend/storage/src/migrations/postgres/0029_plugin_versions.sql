DELETE FROM user_plugins;
ALTER TABLE user_plugins ADD COLUMN version TEXT NOT NULL;
