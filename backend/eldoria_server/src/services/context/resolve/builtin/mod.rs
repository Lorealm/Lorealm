use std::sync::Arc;

use crate::services::state::StateService;

use super::ResolverRegistry;

mod character;
mod chat_history;
mod world_info;

pub use character::CharacterResolver;
pub use chat_history::ChatHistoryResolver;
pub use world_info::WorldInfoResolver;

pub fn register_builtin_resolvers(registry: &mut ResolverRegistry, state: Arc<StateService>) {
    registry.register(Arc::new(ChatHistoryResolver));
    registry.register(Arc::new(CharacterResolver));
    registry.register(Arc::new(WorldInfoResolver::new(Arc::clone(&state))));
}
