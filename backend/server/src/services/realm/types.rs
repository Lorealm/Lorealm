use lorealm_storage::models::{Realm, RealmAgent};

pub trait TurnPolicy: Send + Sync {
    fn next<'a>(&self, realm: &Realm, agents: &'a [RealmAgent]) -> Option<&'a RealmAgent>;
}

pub struct RoundRobin;

impl TurnPolicy for RoundRobin {
    fn next<'a>(&self, realm: &Realm, agents: &'a [RealmAgent]) -> Option<&'a RealmAgent> {
        if agents.is_empty() {
            return None;
        }
        agents.get((realm.tick % agents.len() as i64) as usize)
    }
}
