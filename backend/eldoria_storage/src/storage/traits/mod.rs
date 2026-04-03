pub mod core;
pub mod permissions;
pub mod sessions;
pub mod users;

pub use core::StorageCore;
pub use permissions::PermissionStorage;
pub use sessions::SessionStorage;
pub use users::UserStorage;

pub trait Storage: StorageCore + UserStorage + SessionStorage + PermissionStorage {}

impl<T> Storage for T where T: StorageCore + UserStorage + SessionStorage + PermissionStorage {}
