pub mod characters;
pub mod core;
pub mod permissions;
pub mod sessions;
pub mod users;

pub use characters::CharacterStorage;
pub use core::StorageCore;
pub use permissions::PermissionStorage;
pub use sessions::SessionStorage;
pub use users::UserStorage;

pub trait Storage:
    StorageCore + UserStorage + SessionStorage + PermissionStorage + CharacterStorage
{
}

impl<T> Storage for T where
    T: StorageCore + UserStorage + SessionStorage + PermissionStorage + CharacterStorage
{
}
