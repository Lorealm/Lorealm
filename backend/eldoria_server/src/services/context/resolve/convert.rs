use context_weaver::{Entry, Lorebook as CwLorebook, LorebookConfig};
use eldoria_storage::models::Lorebook as StorageLorebook;

use crate::errors::ContextError;

/// One-way adapter: storage `Lorebook` -> ContextWeaver `Lorebook`.
/// ContextWeaver only reads the result; nothing here round-trips back.
pub fn to_weaver_lorebook(book: StorageLorebook) -> Result<CwLorebook, ContextError> {
    // `config_yaml` is the serialized LorebookConfig (the former lorebook.yaml).
    let config: LorebookConfig = serde_yaml::from_str(&book.config_yaml)?;

    let mut cw = CwLorebook::new();
    cw.config = config;

    for entry in book.entries {
        let parsed = Entry::parse(&entry.source, None)?;
        cw.add_entry(parsed);
    }

    Ok(cw)
}