use context_weaver::{Entry, Lorebook as CwLorebook, LorebookConfig};
use eldoria_storage::models::Lorebook as StorageLorebook;

use crate::errors::ContextError;

/// One-way adapter: storage `Lorebook` -> ContextWeaver `Lorebook`.
/// ContextWeaver only reads the result; nothing here round-trips back.
pub fn to_weaver_lorebook(book: StorageLorebook) -> Result<CwLorebook, ContextError> {
    // `config_yaml` is the serialized LorebookConfig (the former lorebook.yaml).
    let config: LorebookConfig = serde_yaml::from_str(&book.config_yaml)?; // FLAG: error conv

    let mut cw = CwLorebook::new();
    cw.config = config; // `config` is the one public field; no config-taking ctor exists

    for entry in book.entries {
        // FLAG #1 (blocking): parse one `.weaver` source string -> Entry.
        let parsed = Entry::parse(&entry.source, None)?;
        cw.add_entry(parsed);
    }

    Ok(cw)
}