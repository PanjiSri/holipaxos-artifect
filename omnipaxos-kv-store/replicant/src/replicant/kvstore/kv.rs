use std::collections::HashMap;
use omnipaxos::macros::Entry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub client_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Entry)]
pub enum Command {
    Put(KeyValue),
    Delete(KeyValue),
    Get(KeyValue),
}

/*impl Entry for Command {
    type Snapshot = KVSnapshot;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KVSnapshot {
    snapshotted: HashMap<String, String>,
    deleted_keys: Vec<String>,
}

impl Snapshot<Command> for KVSnapshot {
    fn create(entries: &[Command]) -> Self {
        let mut snapshotted = HashMap::new();
        let mut deleted_keys: Vec<String> = Vec::new();
        for e in entries {
            match e {
                Command::Put(KeyValue { key, value , client_id}) => {
                    snapshotted.insert(key.clone(), value.clone());
                }
                Command::Delete(KeyValue { key, value , client_id}) => {
                    if snapshotted.remove(key).is_none() {
                        // key was not in the snapshot
                        deleted_keys.push(key.clone());
                    }
                }
                Command::Get(_) => (),
            }
        }
        // remove keys that were put back
        deleted_keys.retain(|k| !snapshotted.contains_key(k));
        Self {
            snapshotted,
            deleted_keys,
        }
    }

    fn merge(&mut self, delta: Self) {
        for (k, v) in delta.snapshotted {
            self.snapshotted.insert(k, v);
        }
        for k in delta.deleted_keys {
            self.snapshotted.remove(&k);
        }
        self.deleted_keys.clear();
    }

    fn use_snapshots() -> bool {
        true
    }
}*/
