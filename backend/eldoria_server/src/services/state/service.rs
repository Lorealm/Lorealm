use std::{collections::HashMap, sync::Arc};

use eldoria_storage::{
    Storage,
    models::{ActivationState, Lorebook, MessageState, Snowflake, StateDelta},
};

use crate::{
    errors::{StateError, TargetType},
    services::{
        context::ContextTemplate,
        state::{CapturedState, ReconstructedState, StatePool},
    },
};

pub struct StateService {
    storage: Arc<dyn Storage>,
}

impl StateService {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn set_active_lorebooks(
        &self,
        user_id: Snowflake,
        lorebooks: Vec<Snowflake>,
    ) -> Result<(), StateError> {
        self.storage
            .set_enabled_lorebooks(user_id, &lorebooks)
            .await?;
        Ok(())
    }

    pub async fn get_active_lorebook_ids(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Vec<Snowflake>>, StateError> {
        let lorebook_ids = match self.storage.list_enabled_lorebooks(user_id).await {
            Ok(ids) => ids,
            Err(e) => match e {
                eldoria_storage::StorageError::NotFound(_) => return Ok(None),
                _ => return Err(e.into()),
            },
        };

        Ok(Some(lorebook_ids))
    }

    /// Retrieves an ordered list of active lorebooks
    pub async fn get_active_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Vec<Lorebook>>, StateError> {
        let lorebook_ids = self.get_active_lorebook_ids(user_id).await?;

        if lorebook_ids.is_none() {
            return Ok(None);
        }

        let mut lorebooks: Vec<Lorebook> = Vec::new();
        // unwrap: invariant check above
        for lorebook_id in lorebook_ids.unwrap() {
            let maybe_lorebook = self.storage.get_lorebook(lorebook_id).await?;
            if let Some(lorebook) = maybe_lorebook {
                if lorebook.owner_id != user_id {
                    tracing::warn!(
                        "User ({user_id}) tried requesting a lorebook they did not own. Dropping silently."
                    );
                    continue;
                }
                lorebooks.push(lorebook);
            }
        }

        Ok(Some(lorebooks))
    }

    pub async fn set_active_template(
        &self,
        user_id: Snowflake,
        template_id: Option<Snowflake>,
    ) -> Result<(), StateError> {
        self.storage
            .set_active_template(user_id, template_id)
            .await?;
        Ok(())
    }

    pub async fn get_active_template_id(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StateError> {
        let template_id = self.storage.get_active_template(user_id).await?;

        Ok(template_id)
    }

    pub async fn get_active_template(
        &self,
        user_id: Snowflake,
    ) -> Result<ContextTemplate, StateError> {
        let template_id = self
            .get_active_template_id(user_id)
            .await?
            .ok_or(StateError::NoActiveTemplate)?;

        let template = self
            .storage
            .get_template_owned(template_id, user_id)
            .await?
            .ok_or(StateError::NotFound(
                TargetType::ContextTemplate,
                template_id,
            ))?;

        let ctx_template = ContextTemplate::from_db(template)
            .map_err(|e| StateError::TemplateConversion(e.to_string()))?;

        Ok(ctx_template)
    }

    /// Replay state up an ancestor chain.
    ///
    /// `chain` MUST be the full ancestry in root→leaf order. Messages without
    /// a state row are skipped.
    pub async fn reconstruct(&self, chain: &[Snowflake]) -> Result<ReconstructedState, StateError> {
        let states = self.storage.get_message_states(chain).await?;

        let mut persistent: StatePool = HashMap::new();
        // Borrow the nearest snapshot during the walk, clone once at the end.
        let mut nearest_activation: Option<&HashMap<Snowflake, ActivationState>> = None;

        for id in chain {
            let Some(ms) = states.get(id) else { continue };

            for (ns, vars) in &ms.persistent.writes {
                let target = persistent.entry(ns.clone()).or_default();
                for (name, v) in vars {
                    target.insert(name.clone(), v.clone());
                }
            }

            for (ns, names) in &ms.persistent.deletes {
                let now_empty = if let Some(target) = persistent.get_mut(ns) {
                    for name in names {
                        target.remove(name);
                    }
                    target.is_empty()
                } else {
                    false
                };
                if now_empty {
                    persistent.remove(ns);
                }
            }

            nearest_activation = Some(&ms.activation);
        }

        Ok(ReconstructedState {
            persistent,
            activation: nearest_activation.cloned().unwrap_or_default(),
        })
    }

    /// Per-turn changes
    pub fn diff(&self, input: &StatePool, output: &StatePool) -> StateDelta {
        let mut writes: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        for (ns, out_vars) in output {
            for (name, value) in out_vars {
                let unchanged = input
                    .get(ns)
                    .and_then(|vars| vars.get(name))
                    .is_some_and(|prev| prev == value);
                if !unchanged {
                    writes
                        .entry(ns.clone())
                        .or_default()
                        .insert(name.clone(), value.clone());
                }
            }
        }

        let mut deletes: HashMap<String, Vec<String>> = HashMap::new();
        for (ns, in_vars) in input {
            for name in in_vars.keys() {
                let gone = output
                    .get(ns)
                    .map(|vars| !vars.contains_key(name))
                    .unwrap_or(true);
                if gone {
                    deletes.entry(ns.clone()).or_default().push(name.clone());
                }
            }
        }

        StateDelta { writes, deletes }
    }

    /// Write the state produced by one assistant message.
    pub async fn persist(
        &self,
        message_id: Snowflake,
        captured: CapturedState,
    ) -> Result<(), StateError> {
        let state = MessageState {
            message_id,
            activation: captured.activation,
            persistent: captured.persistent,
        };
        self.storage.put_message_state(&state).await?;
        Ok(())
    }
}
