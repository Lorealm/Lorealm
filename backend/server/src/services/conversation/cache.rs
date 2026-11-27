use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use lorealm_storage::models::{ConversationMessage, Snowflake};

type Messages = Arc<HashMap<Snowflake, ConversationMessage>>;

struct Entry {
    revision: i64,
    messages: Messages,
    bytes: usize,
    used: Instant,
}

/// A cache over conversations, synced on a revision counter.
pub struct MessageCache {
    inner: Mutex<Inner>,
}

struct Inner {
    entries: HashMap<Snowflake, Entry>,
    bytes: usize,
    /// Roughly how many bytes of messages to keep. 0 = none.
    budget: usize,
}

impl MessageCache {
    pub fn new(budget: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                entries: HashMap::new(),
                bytes: 0,
                budget,
            }),
        }
    }

    /// The messages of `id` at the given revision. If the revision does
    /// not match, this function returns `None`.
    pub fn get(&self, id: Snowflake, revision: i64) -> Option<Messages> {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.entries.get_mut(&id)?;
        if entry.revision != revision {
            return None;
        }
        entry.used = Instant::now();
        Some(Arc::clone(&entry.messages))
    }

    /// Stores conversation messages on the given revision
    pub fn put(
        &self,
        id: Snowflake,
        revision: i64,
        messages: Vec<ConversationMessage>,
    ) -> Messages {
        let messages: Messages = Arc::new(messages.into_iter().map(|m| (m.id, m)).collect());
        let mut inner = self.inner.lock().unwrap();
        inner.insert(id, revision, Arc::clone(&messages));
        inner.evict();
        messages
    }

    pub fn record_write(&self, id: Snowflake, revision: i64, inserted: Option<&ConversationMessage>) {
        let mut inner = self.inner.lock().unwrap();
        let Some(entry) = inner.entries.remove(&id) else {
            return;
        };
        inner.bytes -= entry.bytes;
        if let (true, Some(message)) = (entry.revision == revision - 1, inserted) {
            let mut messages = entry.messages;
            Arc::make_mut(&mut messages).insert(message.id, message.clone());
            inner.insert(id, revision, messages);
            inner.evict();
        }
    }
}

impl Inner {
    fn insert(&mut self, id: Snowflake, revision: i64, messages: Messages) {
        let bytes = messages.values().map(message_bytes).sum();
        let entry = Entry {
            revision,
            messages,
            bytes,
            used: Instant::now(),
        };
        self.bytes += bytes;
        if let Some(old) = self.entries.insert(id, entry) {
            self.bytes -= old.bytes;
        }
    }

    /// Drops the least recently used half until under budget.
    fn evict(&mut self) {
        while self.bytes > self.budget {
            let mut used: Vec<Instant> = self.entries.values().map(|e| e.used).collect();
            // At least one entry, at most half.
            let mid = (used.len() / 2).saturating_sub(1);
            let (_, &mut cutoff, _) = used.select_nth_unstable(mid);
            self.entries.retain(|_, e| e.used > cutoff);
            self.bytes = self.entries.values().map(|e| e.bytes).sum();
        }
    }
}

/// Roughly the memory a message takes, allocator overhead aside.
fn message_bytes(message: &ConversationMessage) -> usize {
    fn json(value: &serde_json::Value) -> usize {
        size_of::<serde_json::Value>()
            + match value {
                serde_json::Value::String(s) => s.len(),
                serde_json::Value::Array(items) => items.iter().map(json).sum(),
                // A key's `String`, its hash and its slot in the index.
                serde_json::Value::Object(map) => {
                    map.iter().map(|(k, v)| 40 + k.len() + json(v)).sum()
                }
                _ => 0,
            }
    }
    // The map entry holds the id and the message itself.
    size_of::<(Snowflake, ConversationMessage)>() + message.content.len() + json(&message.metadata)
}

pub fn path_to(
    messages: &HashMap<Snowflake, ConversationMessage>,
    leaf: Snowflake,
) -> Vec<ConversationMessage> {
    let mut path = Vec::new();
    let mut cursor = Some(leaf);
    while let Some(message) = cursor.and_then(|id| messages.get(&id)) {
        path.push(message.clone());
        cursor = message.parent;
        if path.len() > messages.len() {
            break; // a cycle; can't happen in a well-formed tree
        }
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lorealm_storage::models::ChatRole;

    fn msg(id: i64, parent: Option<i64>) -> ConversationMessage {
        ConversationMessage {
            id: Snowflake(id),
            conversation_id: Snowflake(1),
            parent: parent.map(Snowflake),
            role: ChatRole::User,
            content: String::new(),
            order: 0,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn ids(path: &[ConversationMessage]) -> Vec<i64> {
        path.iter().map(|m| m.id.0).collect()
    }

    #[test]
    fn only_current_copies_are_returned() {
        let cache = MessageCache::new(1 << 20);
        let convo = Snowflake(1);
        cache.put(convo, 3, vec![msg(10, None)]);
        assert!(cache.get(convo, 3).is_some());
        assert!(cache.get(convo, 4).is_none());
    }

    #[test]
    fn writes_extend_a_current_copy() {
        let cache = MessageCache::new(1 << 20);
        let convo = Snowflake(1);
        cache.put(convo, 3, vec![msg(10, None)]);
        cache.record_write(convo, 4, Some(&msg(11, Some(10))));
        let messages = cache.get(convo, 4).unwrap();
        assert_eq!(ids(&path_to(&messages, Snowflake(11))), [10, 11]);
    }

    #[test]
    fn a_missed_write_drops_the_copy() {
        let cache = MessageCache::new(1 << 20);
        let convo = Snowflake(1);
        cache.put(convo, 3, vec![msg(10, None)]);
        // Revision 4 came from another server.
        cache.record_write(convo, 5, Some(&msg(11, Some(10))));
        assert!(cache.get(convo, 5).is_none());
    }

    #[test]
    fn writes_without_an_insert_drop_the_copy() {
        let cache = MessageCache::new(1 << 20);
        let convo = Snowflake(1);
        cache.put(convo, 3, vec![msg(10, None)]);
        cache.record_write(convo, 4, None);
        assert!(cache.get(convo, 4).is_none());
    }

    #[test]
    fn a_zero_budget_keeps_nothing() {
        let cache = MessageCache::new(0);
        let convo = Snowflake(1);
        let messages = cache.put(convo, 3, vec![msg(10, None)]);
        assert_eq!(messages.len(), 1);
        assert!(cache.get(convo, 3).is_none());
    }

    #[test]
    fn the_least_recently_used_go_first() {
        let size = message_bytes(&msg(10, None));
        let cache = MessageCache::new(2 * size);
        cache.put(Snowflake(1), 0, vec![msg(10, None)]);
        cache.put(Snowflake(2), 0, vec![msg(20, None)]);
        cache.get(Snowflake(1), 0);
        cache.put(Snowflake(3), 0, vec![msg(30, None)]);
        assert!(cache.get(Snowflake(1), 0).is_some());
        assert!(cache.get(Snowflake(2), 0).is_none());
    }

    #[test]
    fn path_follows_parents_from_the_leaf() {
        let messages: HashMap<_, _> = [
            msg(1, None),
            msg(2, Some(1)),
            msg(3, Some(1)),
            msg(4, Some(3)),
        ]
        .into_iter()
        .map(|m| (m.id, m))
        .collect();
        assert_eq!(ids(&path_to(&messages, Snowflake(4))), [1, 3, 4]);
        assert!(path_to(&messages, Snowflake(99)).is_empty());
    }
}
