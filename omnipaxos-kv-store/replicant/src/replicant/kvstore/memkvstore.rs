use super::KVStore;
use std::collections::HashMap;
use crate::replicant::kvstore::kv::{Command, KeyValue};

pub struct MemKVStore {
    map: HashMap<String, String>,
}

impl MemKVStore {
    pub fn new() -> Self {
        MemKVStore {
            map: HashMap::new(),
        }
    }
}

impl KVStore for MemKVStore {
    fn execute(&mut self, command: Command) -> (String, i64) {
        match command {
            Command::Put(KeyValue { key, value, client_id }) => {
                self.put(&key, &value);
                ("".parse().unwrap(), client_id)
            }
            Command::Delete(KeyValue { key, value, client_id }) => {
                self.del(&key);
                ("".parse().unwrap(), client_id)
            }
            Command::Get(KeyValue { key, value, client_id }) => {
                (self.get(key.as_str()), client_id)
            },
        }
    }

    fn get(&self, key: &str) -> String {
        match self.map.get(key) {
            Some(value) => return value.clone(),
            _ => return "".parse().unwrap(),
        }
    }

    fn put(&mut self, key: &str, value: &str) -> bool {
        self.map.insert(key.to_string(), value.to_string());
        true
    }

    fn del(&mut self, key: &str) -> bool {
        self.map.remove(key).is_some()
    }
}
