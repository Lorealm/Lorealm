mod message;
mod user;
mod session;
mod character;

use std::sync::Arc;

pub use message::MessageService;
use storage_system::Storage;
pub use user::UserService;
pub use session::SessionService;
pub use character::CharacterService;

pub trait StorageService<S: Storage + ?Sized> {
    fn new(storage: Arc<S>) -> Self;
}