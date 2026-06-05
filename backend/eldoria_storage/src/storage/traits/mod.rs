mod blob;
mod characters;
mod conversations;
mod core;
mod lorebook;
mod permissions;
mod sessions;
mod state;
mod templates;
mod users;

pub use blob::BlobStore;
pub use characters::CharacterStorage;
pub use conversations::ConversationStorage;
pub use core::StorageCore;
pub use lorebook::LorebookStorage;
pub use permissions::PermissionStorage;
pub use sessions::SessionStorage;
pub use state::WeaverStateStorage;
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
    + ConversationStorage
    + WeaverStateStorage
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
{
}
