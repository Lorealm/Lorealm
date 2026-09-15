mod blob;
mod blueprints;
mod characters;
mod connection;
mod conversations;
mod core;
mod lorebook;
mod permissions;
mod persona;
mod realms;
mod sessions;
mod state;
mod templates;
mod tokenizer;
mod tools;
mod users;
mod widgets;

pub use blob::BlobStore;
pub use blueprints::BlueprintStorage;
pub use characters::CharacterStorage;
pub use connection::{ConnectionStorage, SecretStorage};
pub use conversations::ConversationStorage;
pub use core::StorageCore;
pub use lorebook::LorebookStorage;
pub use permissions::PermissionStorage;
pub use persona::PersonaStorage;
pub use realms::RealmStorage;
pub use sessions::SessionStorage;
pub use state::WeaverStateStorage;
pub use templates::TemplateStorage;
pub use tokenizer::TokenizerStorage;
pub use tools::UserToolStorage;
pub use users::UserStorage;
pub use widgets::UserWidgetStorage;

pub trait Storage:
    StorageCore
    + UserStorage
    + SessionStorage
    + PermissionStorage
    + CharacterStorage
    + TemplateStorage
    + LorebookStorage
    + ConversationStorage
    + WeaverStateStorage
    + PersonaStorage
    + SecretStorage
    + ConnectionStorage
    + UserToolStorage
    + UserWidgetStorage
    + TokenizerStorage
    + RealmStorage
    + BlueprintStorage
{
}

impl<T> Storage for T where
    T: StorageCore
        + UserStorage
        + SessionStorage
        + PermissionStorage
        + CharacterStorage
        + TemplateStorage
        + LorebookStorage
        + ConversationStorage
        + WeaverStateStorage
        + PersonaStorage
        + SecretStorage
        + ConnectionStorage
        + UserToolStorage
        + UserWidgetStorage
        + TokenizerStorage
        + RealmStorage
        + BlueprintStorage
        + BlueprintStorage
{
}
