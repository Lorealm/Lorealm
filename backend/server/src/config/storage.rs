use crate::config::Config;

impl Config for lorealm_storage::StorageConfig {
    fn key() -> &'static str {
        "storage"
    }
}
