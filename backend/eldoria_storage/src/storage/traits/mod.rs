mod blob;
mod characters;
mod conversations;
mod core;
mod lorebook;
mod lorebook_session;
mod permissions;
mod sessions;
mod templates;
mod users;

pub use blob::BlobStore;
pub use characters::CharacterStorage;
pub use conversations::ConversationStorage;
pub use core::StorageCore;
pub use lorebook::LorebookStorage;
pub use lorebook_session::LorebookSessionStorage;
pub use permissions::PermissionStorage;
pub use sessions::SessionStorage;
pub use templates::TemplateStorage;
pub use users::UserStorage;

pub trait Storage:
    StorageCore
    + UserStorage
    + SessionStorage
    + PermissionStorage
    + CharacterStorage
    + TemplateStorage
    + LorebookStorage
    + LorebookSessionStorage
    + ConversationStorage
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
        + LorebookSessionStorage
        + ConversationStorage
{
}
