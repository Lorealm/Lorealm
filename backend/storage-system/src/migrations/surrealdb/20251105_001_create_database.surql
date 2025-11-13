-- SurrealDB Migration: Initial Schema
-- Version: 001
-- Description: Define tables for conversations, messages, users, and related entities

-- Define Users table
DEFINE TABLE users SCHEMAFULL;
DEFINE FIELD id ON TABLE users TYPE record<users>;
DEFINE FIELD permissions ON TABLE users TYPE int DEFAULT 0;
DEFINE FIELD email ON TABLE users TYPE option<string>;
DEFINE FIELD display_name ON TABLE users TYPE option<string>;
DEFINE FIELD created_at ON TABLE users TYPE datetime;

-- Index on email for faster lookups
DEFINE INDEX idx_users_email ON TABLE users COLUMNS email;

-- Define Conversations table
DEFINE TABLE conversations SCHEMAFULL;
DEFINE FIELD id ON TABLE conversations TYPE record<conversations>;
DEFINE FIELD user_id ON TABLE conversations TYPE record<users>;
DEFINE FIELD created_at ON TABLE conversations TYPE datetime;
DEFINE FIELD updated_at ON TABLE conversations TYPE datetime;
DEFINE FIELD context_type ON TABLE conversations TYPE string 
    ASSERT $value IN ['empty', 'character', 'scenario', 'world', 'custom'];
DEFINE FIELD context_id ON TABLE conversations TYPE option<string>;
DEFINE FIELD context_version ON TABLE conversations TYPE option<int>;
DEFINE FIELD context_metadata ON TABLE conversations TYPE option<string>;
DEFINE FIELD sync_policy ON TABLE conversations TYPE string;

-- Indexes for conversations
DEFINE INDEX idx_conversations_user_id ON TABLE conversations COLUMNS user_id;
DEFINE INDEX idx_conversations_context_id ON TABLE conversations COLUMNS context_id;
DEFINE INDEX idx_conversations_updated_at ON TABLE conversations COLUMNS updated_at;

-- Define Conversation Metadata table
DEFINE TABLE conversation_metadata SCHEMAFULL;
DEFINE FIELD id ON TABLE conversation_metadata TYPE record<conversation_metadata>;
DEFINE FIELD conversation_id ON TABLE conversation_metadata TYPE record<conversations>;
DEFINE FIELD title ON TABLE conversation_metadata TYPE option<string>;
DEFINE FIELD message_count ON TABLE conversation_metadata TYPE int DEFAULT 0;
DEFINE FIELD last_message_at ON TABLE conversation_metadata TYPE datetime;
DEFINE FIELD created_at ON TABLE conversation_metadata TYPE datetime;
DEFINE FIELD updated_at ON TABLE conversation_metadata TYPE datetime;

-- Unique index on conversation_id
DEFINE INDEX idx_conversation_metadata_conversation_id ON TABLE conversation_metadata COLUMNS conversation_id UNIQUE;
DEFINE INDEX idx_conversation_metadata_last_message_at ON TABLE conversation_metadata COLUMNS last_message_at;

-- Define Messages table (tree structure with parent relationships)
DEFINE TABLE messages SCHEMAFULL;
DEFINE FIELD id ON TABLE messages TYPE record<messages>;
DEFINE FIELD conversation_id ON TABLE messages TYPE record<conversations>;
DEFINE FIELD parent_message_id ON TABLE messages TYPE option<record<messages>>;
DEFINE FIELD role ON TABLE messages TYPE string 
    ASSERT $value IN ['user', 'assistant', 'system'];
DEFINE FIELD content ON TABLE messages TYPE string;
DEFINE FIELD created_at ON TABLE messages TYPE datetime;
DEFINE FIELD updated_at ON TABLE messages TYPE datetime;

-- Indexes for messages
DEFINE INDEX idx_messages_conversation_id ON TABLE messages COLUMNS conversation_id;
DEFINE INDEX idx_messages_parent_message_id ON TABLE messages COLUMNS parent_message_id;
DEFINE INDEX idx_messages_created_at ON TABLE messages COLUMNS created_at;

-- Define Active Paths table (tracks current path through message tree)
DEFINE TABLE active_paths SCHEMAFULL;
DEFINE FIELD conversation_id ON TABLE active_paths TYPE record<conversations>;
DEFINE FIELD message_ids ON TABLE active_paths TYPE array;
DEFINE FIELD message_ids.* ON TABLE active_paths TYPE string;
DEFINE FIELD updated_at ON TABLE active_paths TYPE datetime;

-- Unique index on conversation_id (one active path per conversation)
DEFINE INDEX idx_active_paths_conversation_id ON TABLE active_paths COLUMNS conversation_id UNIQUE;

-- Define Instantiated Entities table (snapshots of entities from world/character db)
DEFINE TABLE instantiated_entities SCHEMAFULL;
DEFINE FIELD id ON TABLE instantiated_entities TYPE record<instantiated_entities>;
DEFINE FIELD conversation_id ON TABLE instantiated_entities TYPE record<conversations>;
DEFINE FIELD entity_id ON TABLE instantiated_entities TYPE string;
DEFINE FIELD entity_type ON TABLE instantiated_entities TYPE string;
DEFINE FIELD snapshot_data ON TABLE instantiated_entities TYPE string;
DEFINE FIELD instantiated_at ON TABLE instantiated_entities TYPE datetime;
DEFINE FIELD modified_at ON TABLE instantiated_entities TYPE datetime;

-- Indexes for instantiated entities
DEFINE INDEX idx_instantiated_entities_conversation_id ON TABLE instantiated_entities COLUMNS conversation_id;
DEFINE INDEX idx_instantiated_entities_entity_id ON TABLE instantiated_entities COLUMNS entity_id;
DEFINE INDEX idx_instantiated_entities_entity_type ON TABLE instantiated_entities COLUMNS entity_type;
