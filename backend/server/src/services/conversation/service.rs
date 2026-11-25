use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;

use lorealm_storage::{
    Storage,
    models::{ChatRole, ConversationMessage, MessageEdit, Snowflake},
};

use crate::{
    errors::ConversationError,
    services::{conversation::EditMode, snowflake::SnowflakeService},
};

pub struct ConversationService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

impl ConversationService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    fn build_message(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
        role: ChatRole,
        content: String,
        order: i32,
        metadata: serde_json::Value,
    ) -> ConversationMessage {
        let now = Utc::now();
        ConversationMessage {
            id: self.snowflake.generate(),
            conversation_id,
            parent,
            role,
            content,
            order,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }

    /// Next `order` for a new child of `parent`
    async fn next_order(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<i32, ConversationError> {
        let children = self.storage.get_children(conversation_id, parent).await?;
        Ok(children.last().map_or(0, |m| m.order + 1))
    }

    /// Continue the current thread: append a child of the active leaf, advancing
    /// the leaf to it.
    pub async fn append_message(
        &self,
        conversation_id: Snowflake,
        role: ChatRole,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<ConversationMessage, ConversationError> {
        let conversation = self
            .storage
            .get_conversation(conversation_id)
            .await?
            .ok_or(ConversationError::ConversationNotFound(conversation_id))?;

        let parent = conversation.active_leaf_id;
        let order = self.next_order(conversation_id, parent).await?;
        let message = self.build_message(conversation_id, parent, role, content, order, metadata);

        self.storage.append_and_advance(&message).await?;
        Ok(message)
    }

    /// Fork *below* `message_id`: keep it and its existing children, start a new
    /// continuation as a fresh child of it.
    pub async fn branch_from(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        role: ChatRole,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<ConversationMessage, ConversationError> {
        self.storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        let order = self.next_order(conversation_id, Some(message_id)).await?;
        let message = self.build_message(
            conversation_id,
            Some(message_id),
            role,
            content,
            order,
            metadata,
        );

        self.storage.append_and_advance(&message).await?;
        Ok(message)
    }

    /// Create an *alternate (swipe)* of `message_id`: a new sibling sharing
    /// the same parent, which becomes the active leaf.
    pub async fn create_alternate(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        role: ChatRole,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<ConversationMessage, ConversationError> {
        let parent = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?
            .parent;

        let order = self.next_order(conversation_id, parent).await?;
        let message = self.build_message(conversation_id, parent, role, content, order, metadata);

        self.storage.append_and_advance(&message).await?;
        Ok(message)
    }

    /// Edit a message, in place or as a new branch (see `EditMode`).
    pub async fn edit_message(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        new_content: String,
        mode: super::EditMode,
    ) -> Result<ConversationMessage, ConversationError> {
        let original = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        match mode {
            EditMode::InPlace => {
                let edit = MessageEdit {
                    content: Some(new_content.clone()),
                    metadata: None,
                };
                self.storage.update_message(message_id, &edit).await?;

                let mut updated = original;
                updated.content = new_content;
                updated.updated_at = Utc::now();
                Ok(updated)
            }
            EditMode::Branch => {
                let order = self.next_order(conversation_id, original.parent).await?;
                let message = self.build_message(
                    conversation_id,
                    original.parent,
                    original.role,
                    new_content,
                    order,
                    original.metadata,
                );

                self.storage.append_and_advance(&message).await?;
                // The copy keeps `has_context`, so it needs the prompt too.
                if let Some(context) = self.storage.get_message_context(original.id).await? {
                    self.storage
                        .put_message_context(message.id, &context)
                        .await?;
                }
                Ok(message)
            }
        }
    }

    /// Replace a message's content and metadata.
    pub async fn rewrite_message(
        &self,
        message_id: Snowflake,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<ConversationMessage, ConversationError> {
        let original = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        let edit = MessageEdit {
            content: Some(content.clone()),
            metadata: Some(metadata.clone()),
        };
        self.storage.update_message(message_id, &edit).await?;

        Ok(ConversationMessage {
            content,
            metadata,
            updated_at: Utc::now(),
            ..original
        })
    }

    /// Delete `message_id` and its subtree
    pub async fn delete_subtree(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<(), ConversationError> {
        let parent = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?
            .parent;

        let on_active_path = self
            .storage
            .get_active_path(conversation_id)
            .await?
            .iter()
            .any(|m| m.id == message_id);

        self.storage.delete_message(message_id).await?;

        if on_active_path {
            self.storage
                .set_active_leaf(conversation_id, parent)
                .await?;
        }
        Ok(())
    }

    /// Move the active path so it runs through `message_id`
    pub async fn switch_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, ConversationError> {
        let messages = self.storage.list_messages(conversation_id).await?;

        let mut children: HashMap<Snowflake, Vec<Snowflake>> = HashMap::new();
        for m in &messages {
            if let Some(parent) = m.parent {
                children.entry(parent).or_default().push(m.id);
            }
        }
        let by_id: HashMap<Snowflake, ConversationMessage> =
            messages.into_iter().map(|m| (m.id, m)).collect();

        if !by_id.contains_key(&message_id) {
            return Err(ConversationError::MessageNotFound(message_id));
        }

        let mut leaf = message_id;
        for _ in 0..by_id.len() {
            let Some(next) = children.get(&leaf).and_then(|kids| {
                kids.iter().copied().max_by_key(|id| {
                    let m = &by_id[id];
                    (m.updated_at, m.order, *id)
                })
            }) else {
                break;
            };
            leaf = next;
        }

        self.storage
            .set_active_leaf(conversation_id, Some(leaf))
            .await?;

        // Build root -> leaf from the data we already loaded.
        let mut path = Vec::new();
        let mut cursor = Some(leaf);
        while let Some(id) = cursor {
            let Some(m) = by_id.get(&id) else { break };
            path.push(m.clone());
            cursor = m.parent;
            if path.len() > by_id.len() {
                break;
            }
        }
        path.reverse();
        Ok(path)
    }

    /// The current visible thread, root -> leaf. Empty if there is no active leaf.
    pub async fn active_thread(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, ConversationError> {
        Ok(self.storage.get_active_path(conversation_id).await?)
    }

    /// Sibling set around `message_id` plus which one the active path runs
    /// through
    #[cfg(test)]
    pub async fn branch_info(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<super::BranchInfo, ConversationError> {
        use crate::services::conversation::BranchInfo;

        let parent = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ConversationError::MessageNotFound(message_id))?
            .parent;

        let siblings = self.storage.get_children(conversation_id, parent).await?;
        let active_path = self.storage.get_active_path(conversation_id).await?;

        let active_index = siblings
            .iter()
            .position(|s| active_path.iter().any(|m| m.id == s.id));

        Ok(BranchInfo {
            siblings,
            active_index,
        })
    }

    /// Ordered children of `parent`
    #[cfg(test)] // temporary until needed
    pub async fn children_of(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, ConversationError> {
        Ok(self.storage.get_children(conversation_id, parent).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lorealm_storage::{ConversationStorage, backends::MemoryStorage, models::Conversation};

    fn snowflakes() -> Arc<SnowflakeService> {
        Arc::new(SnowflakeService::new(0))
    }

    async fn setup() -> (
        ConversationService,
        Arc<MemoryStorage>,
        Snowflake,
        Arc<SnowflakeService>,
    ) {
        let snow = snowflakes();
        let store = Arc::new(MemoryStorage::new());
        let now = Utc::now();
        let conv = Conversation {
            id: snow.generate(),
            user_id: snow.generate(),
            character_id: Some(snow.generate()),
            char_version_id: Some(snow.generate()),
            realm_id: None,
            active_leaf_id: None,
            title: Some("Test".into()),
            author_note: String::new(),
            created_at: now,
            updated_at: now,
        };
        let conv_id = conv.id;
        store.create_conversation(&conv).await.unwrap();
        let service = ConversationService::new(store.clone(), snow.clone());
        (service, store, conv_id, snow)
    }

    const NIL: serde_json::Value = serde_json::Value::Null;

    async fn append(s: &ConversationService, c: Snowflake, content: &str) -> ConversationMessage {
        s.append_message(c, ChatRole::User, content.into(), NIL)
            .await
            .unwrap()
    }

    async fn branch(
        s: &ConversationService,
        c: Snowflake,
        from: Snowflake,
        content: &str,
    ) -> ConversationMessage {
        s.branch_from(c, from, ChatRole::User, content.into(), NIL)
            .await
            .unwrap()
    }

    async fn alternate(
        s: &ConversationService,
        c: Snowflake,
        of: Snowflake,
        content: &str,
    ) -> ConversationMessage {
        s.create_alternate(c, of, ChatRole::Assistant, content.into(), NIL)
            .await
            .unwrap()
    }

    async fn active_leaf(store: &MemoryStorage, c: Snowflake) -> Option<Snowflake> {
        store
            .get_conversation(c)
            .await
            .unwrap()
            .unwrap()
            .active_leaf_id
    }

    fn contents(msgs: &[ConversationMessage]) -> Vec<String> {
        msgs.iter().map(|m| m.content.clone()).collect()
    }

    #[tokio::test]
    async fn append_to_empty_creates_root() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;

        assert_eq!(a.parent, None);
        assert_eq!(a.order, 0);
        assert_eq!(active_leaf(&store, c).await, Some(a.id));
        assert_eq!(contents(&s.active_thread(c).await.unwrap()), vec!["A"]);
    }

    #[tokio::test]
    async fn append_builds_linear_path_and_advances_leaf() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await;
        let d = append(&s, c, "C").await;

        assert_eq!(b.parent, Some(a.id));
        assert_eq!(d.parent, Some(b.id));
        assert_eq!(active_leaf(&store, c).await, Some(d.id));
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B", "C"]
        );
    }

    #[tokio::test]
    async fn branch_from_preserves_old_subtree_and_is_reachable() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await;
        let old_leaf = append(&s, c, "C").await; // A -> B -> C

        let d = branch(&s, c, a.id, "D").await; // new child of A

        assert_eq!(d.parent, Some(a.id));
        assert_eq!(active_leaf(&store, c).await, Some(d.id));
        assert_eq!(contents(&s.active_thread(c).await.unwrap()), vec!["A", "D"]);

        // Old path still in the tree and reachable.
        assert!(store.get_message(b.id).await.unwrap().is_some());
        assert!(store.get_message(old_leaf.id).await.unwrap().is_some());
        let restored = s.switch_to(c, old_leaf.id).await.unwrap();
        assert_eq!(contents(&restored), vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn create_alternate_makes_sibling_with_incremented_order() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await; // A -> B, leaf B

        let b2 = alternate(&s, c, b.id, "B2").await; // sibling of B (child of A)

        assert_eq!(b2.parent, Some(a.id));
        assert_eq!(b2.order, b.order + 1);
        assert_eq!(active_leaf(&store, c).await, Some(b2.id));
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B2"]
        );
        assert!(store.get_message(b.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn create_alternate_at_root_level() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let a2 = alternate(&s, c, a.id, "A2").await;

        assert_eq!(a2.parent, None);
        assert_eq!(a2.order, 1);
        assert_eq!(contents(&s.active_thread(c).await.unwrap()), vec!["A2"]);
        assert!(store.get_message(a.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn edit_in_place_mutates_and_keeps_subtree() {
        let (s, _store, c, _) = setup().await;
        let _a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await;
        let leaf = append(&s, c, "C").await; // A -> B -> C

        let edited = s
            .edit_message(c, b.id, "B-edited".into(), EditMode::InPlace)
            .await
            .unwrap();

        assert_eq!(edited.id, b.id);
        assert_eq!(edited.content, "B-edited");

        assert_eq!(active_leaf(&_store, c).await, Some(leaf.id));
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B-edited", "C"]
        );
    }

    #[tokio::test]
    async fn edit_branch_creates_sibling_and_drops_continuation() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = s
            .append_message(c, ChatRole::Assistant, "B".into(), NIL)
            .await
            .unwrap();
        let old_leaf = append(&s, c, "C").await; // A -> B(assistant) -> C

        let b2 = s
            .edit_message(c, b.id, "B-alt".into(), EditMode::Branch)
            .await
            .unwrap();

        assert_eq!(b2.parent, Some(a.id));
        assert_eq!(b2.role, ChatRole::Assistant); // role carried from original
        assert_eq!(b2.order, b.order + 1);
        assert_eq!(active_leaf(&store, c).await, Some(b2.id));
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B-alt"]
        );
        // Original and its continuation preserved.
        assert!(store.get_message(b.id).await.unwrap().is_some());
        assert!(store.get_message(old_leaf.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn edit_branch_keeps_the_prompt_of_the_reply_it_copies() {
        let (s, store, c, _) = setup().await;
        append(&s, c, "A").await;
        let b = s
            .append_message(
                c,
                ChatRole::Assistant,
                "B".into(),
                serde_json::json!({ "has_context": true }),
            )
            .await
            .unwrap();
        let prompt = serde_json::json!([{ "role": "system", "content": "be brief" }]);
        store.put_message_context(b.id, &prompt).await.unwrap();

        let b2 = s
            .edit_message(c, b.id, "B-alt".into(), EditMode::Branch)
            .await
            .unwrap();

        assert_eq!(b2.metadata["has_context"], true);
        assert_eq!(
            store.get_message_context(b2.id).await.unwrap(),
            Some(prompt)
        );
    }

    #[tokio::test]
    async fn switch_to_descends_to_most_recent_leaf() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b1 = branch(&s, c, a.id, "B1").await;
        let _c1 = append(&s, c, "C1").await; // A -> B1 -> C1
        let b2 = alternate(&s, c, b1.id, "B2").await; // newer sibling of B1

        // Switching to the fork follows the most recently updated child (B2).
        let path = s.switch_to(c, a.id).await.unwrap();
        assert_eq!(active_leaf(&store, c).await, Some(b2.id));
        assert_eq!(contents(&path), vec!["A", "B2"]);
    }

    #[tokio::test]
    async fn switch_to_into_deeper_branch() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b1 = branch(&s, c, a.id, "B1").await;
        let c1 = append(&s, c, "C1").await; // A -> B1 -> C1
        let _b2 = alternate(&s, c, b1.id, "B2").await; // now active

        let path = s.switch_to(c, b1.id).await.unwrap();
        assert_eq!(active_leaf(&store, c).await, Some(c1.id));
        assert_eq!(contents(&path), vec!["A", "B1", "C1"]);
    }

    #[tokio::test]
    async fn switch_to_return_matches_active_thread() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let _b = append(&s, c, "B").await;
        let returned = s.switch_to(c, a.id).await.unwrap();
        let thread = s.active_thread(c).await.unwrap();
        assert_eq!(contents(&returned), contents(&thread));
    }

    #[tokio::test]
    async fn append_after_switch_continues_chosen_branch() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b1 = branch(&s, c, a.id, "B1").await;
        let b2 = alternate(&s, c, b1.id, "B2").await; // active leaf B2

        s.switch_to(c, b1.id).await.unwrap();
        let new = append(&s, c, "C").await;

        assert_eq!(new.parent, Some(b1.id));
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B1", "C"]
        );
        // B2 untouched.
        assert_eq!(
            _store.get_message(b2.id).await.unwrap().unwrap().parent,
            Some(a.id)
        );
    }

    #[tokio::test]
    async fn branch_info_reports_siblings_and_active_index() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b0 = branch(&s, c, a.id, "B0").await;
        let b1 = alternate(&s, c, b0.id, "B1").await;
        let _b2 = alternate(&s, c, b1.id, "B2").await; // active leaf B2

        let info = s.branch_info(c, b1.id).await.unwrap();
        assert_eq!(info.total(), 3);
        assert_eq!(contents(&info.siblings), vec!["B0", "B1", "B2"]);
        assert_eq!(info.active_index, Some(2)); // active path runs through B2

        let info0 = s.branch_info(c, b0.id).await.unwrap();
        assert_eq!(info0.active_index, Some(2));
    }

    #[tokio::test]
    async fn branch_info_active_index_none_when_group_off_path() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b0 = branch(&s, c, a.id, "B0").await;
        let b1 = alternate(&s, c, b0.id, "B1").await;
        let d = append(&s, c, "D").await; // child of B1 (active leaf B1 -> D)
        assert_eq!(d.parent, Some(b1.id));

        // Move the active path away from B1's subtree.
        s.switch_to(c, b0.id).await.unwrap();

        let info = s.branch_info(c, d.id).await.unwrap();
        assert_eq!(info.total(), 1);
        assert_eq!(info.active_index, None);
    }

    #[tokio::test]
    async fn branch_info_single_child() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await;
        let info = s.branch_info(c, b.id).await.unwrap();
        assert_eq!(info.total(), 1);
        assert_eq!(info.active_index, Some(0));
        let _ = a;
    }

    #[tokio::test]
    async fn delete_subtree_on_active_path_repoints_to_parent() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b = append(&s, c, "B").await;
        let cc = append(&s, c, "C").await;
        let d = append(&s, c, "D").await; // A -> B -> C -> D

        s.delete_subtree(c, b.id).await.unwrap();

        for gone in [b.id, cc.id, d.id] {
            assert!(store.get_message(gone).await.unwrap().is_none());
        }
        assert!(store.get_message(a.id).await.unwrap().is_some());
        assert_eq!(active_leaf(&store, c).await, Some(a.id)); // repointed to fork
        assert_eq!(contents(&s.active_thread(c).await.unwrap()), vec!["A"]);
    }

    #[tokio::test]
    async fn delete_subtree_off_active_path_keeps_leaf() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b1 = branch(&s, c, a.id, "B1").await;
        let b2 = alternate(&s, c, b1.id, "B2").await; // active leaf B2

        s.delete_subtree(c, b1.id).await.unwrap(); // B1 not on active path

        assert!(store.get_message(b1.id).await.unwrap().is_none());
        assert_eq!(active_leaf(&store, c).await, Some(b2.id)); // unchanged
        assert_eq!(
            contents(&s.active_thread(c).await.unwrap()),
            vec!["A", "B2"]
        );
    }

    #[tokio::test]
    async fn delete_root_clears_active_leaf() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;

        s.delete_subtree(c, a.id).await.unwrap();

        assert!(store.get_message(a.id).await.unwrap().is_none());
        assert_eq!(active_leaf(&store, c).await, None);
        assert!(s.active_thread(c).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_subtree_keeps_siblings() {
        let (s, store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b1 = branch(&s, c, a.id, "B1").await;
        let b2 = alternate(&s, c, b1.id, "B2").await;

        s.delete_subtree(c, b1.id).await.unwrap();

        assert!(store.get_message(b1.id).await.unwrap().is_none());
        assert!(store.get_message(b2.id).await.unwrap().is_some());
        assert!(store.get_message(a.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn next_order_uses_max_not_count_after_delete() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let b0 = branch(&s, c, a.id, "B0").await; // order 0
        let b1 = alternate(&s, c, b0.id, "B1").await; // order 1
        let b2 = alternate(&s, c, b1.id, "B2").await; // order 2

        s.delete_subtree(c, b1.id).await.unwrap(); // remove the middle (order 1)
        let b3 = alternate(&s, c, b2.id, "B3").await;

        // count would give 2 and collide with B2; max+1 gives 3.
        assert_eq!(b3.order, 3);
    }

    #[tokio::test]
    async fn children_of_returns_ordered_children_and_roots() {
        let (s, _store, c, _) = setup().await;
        let a = append(&s, c, "A").await;
        let _a2 = alternate(&s, c, a.id, "A2").await; // second root
        let _b = branch(&s, c, a.id, "B").await;

        let roots = s.children_of(c, None).await.unwrap();
        assert_eq!(contents(&roots), vec!["A", "A2"]);

        let children = s.children_of(c, Some(a.id)).await.unwrap();
        assert_eq!(contents(&children), vec!["B"]);
    }

    #[tokio::test]
    async fn deep_linear_path_reconstructs() {
        let (s, _store, c, _) = setup().await;
        let mut first = None;
        for i in 0..10 {
            let m = append(&s, c, &format!("m{i}")).await;
            if i == 0 {
                first = Some(m.id);
            }
        }

        let thread = s.active_thread(c).await.unwrap();
        assert_eq!(thread.len(), 10);
        assert_eq!(thread[0].content, "m0");
        assert_eq!(thread[9].content, "m9");

        // Switching to the root descends back down the only branch.
        let path = s.switch_to(c, first.unwrap()).await.unwrap();
        assert_eq!(path.len(), 10);
    }

    #[tokio::test]
    async fn missing_message_errors_across_ops() {
        let (s, _store, c, snow) = setup().await;
        let missing = snow.generate();

        assert!(matches!(
            s.branch_from(c, missing, ChatRole::User, "x".into(), NIL)
                .await,
            Err(ConversationError::MessageNotFound(_))
        ));
        assert!(matches!(
            s.create_alternate(c, missing, ChatRole::User, "x".into(), NIL)
                .await,
            Err(ConversationError::MessageNotFound(_))
        ));
        assert!(matches!(
            s.edit_message(c, missing, "x".into(), EditMode::InPlace)
                .await,
            Err(ConversationError::MessageNotFound(_))
        ));
        assert!(matches!(
            s.edit_message(c, missing, "x".into(), EditMode::Branch)
                .await,
            Err(ConversationError::MessageNotFound(_))
        ));
        assert!(matches!(
            s.switch_to(c, missing).await,
            Err(ConversationError::MessageNotFound(_))
        ));
        assert!(matches!(
            s.delete_subtree(c, missing).await,
            Err(ConversationError::MessageNotFound(_))
        ));
    }

    #[tokio::test]
    async fn append_to_missing_conversation_errors() {
        let (s, _store, _c, snow) = setup().await;
        let missing = snow.generate();
        assert!(matches!(
            s.append_message(missing, ChatRole::User, "x".into(), NIL)
                .await,
            Err(ConversationError::ConversationNotFound(_))
        ));
    }

    #[tokio::test]
    async fn empty_conversation_has_empty_thread() {
        let (s, _store, c, _) = setup().await;
        assert!(s.active_thread(c).await.unwrap().is_empty());
    }
}
