mod std;
pub mod stdlib;

pub struct LorealmWeaverPlugin;

impl context_weaver::Plugin for LorealmWeaverPlugin {
    fn name(&self) -> &str {
        "core"
    }

    fn register(&self, registry: &mut weaver_lang::Registry) {
        registry.register_processor(std::GenerateRandomProcessor);
        registry.register_processor(std::PickRandomProcessor);
        tracing::debug!("Registered 2 auxiliary processors");
    }
}
