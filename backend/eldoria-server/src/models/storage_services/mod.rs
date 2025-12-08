mod message;
mod user;
mod session;
mod character;
mod templates;

use std::sync::Arc;

pub use message::MessageService;
use storage_system::Storage;
pub use user::UserService;
pub use session::SessionService;
pub use character::CharacterService;
pub use templates::*;

pub trait StorageService<S: Storage + ?Sized> {
    fn new(storage: Arc<S>) -> Self;
}