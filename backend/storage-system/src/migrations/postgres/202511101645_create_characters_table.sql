CREATE TABLE characters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    spec VARCHAR(255) NOT NULL,
    version REAL NOT NULL,
    
    display_name VARCHAR(255),
    creator VARCHAR(255) NOT NULL,
    creator_notes TEXT,
    image_path TEXT,
    
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    personality TEXT NOT NULL,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_characters_user_id ON characters(user_id);
CREATE INDEX idx_characters_name ON characters(name);
CREATE INDEX idx_characters_creator ON characters(creator);

CREATE TABLE character_tags (
    character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (character_id, tag)
);

CREATE INDEX idx_character_tags_tag ON character_tags(tag);

CREATE TABLE character_greetings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    greeting_text TEXT NOT NULL,
    display_order INT NOT NULL
);

CREATE INDEX idx_character_greetings_character ON character_greetings(character_id);

CREATE TABLE character_example_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    message_text TEXT NOT NULL,
    display_order INT NOT NULL
);

CREATE INDEX idx_character_example_messages_character ON character_example_messages(character_id);