ALTER TABLE roles ADD COLUMN position INTEGER NOT NULL DEFAULT 0;

INSERT INTO roles (id, name, permissions, position) VALUES
    (1, '@everyone', 65547, 0),
    (2, 'Administrator', 4611686018427650048, 1000);

INSERT INTO user_roles (user_id, role_id)
SELECT id, 2 FROM users ORDER BY created_at ASC LIMIT 1;
