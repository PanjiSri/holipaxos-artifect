use super::KVStore;
use crate::replicant::kvstore::kv::{Command, KeyValue};
use rocksdb::{DB, Options};

pub struct RocksDBKVStore {
    db: DB
}

impl RocksDBKVStore {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        RocksDBKVStore{
            db: DB::open(&opts, path).unwrap(),
        }
    }
}

impl KVStore for RocksDBKVStore {
    fn execute(&mut self, command: Command) -> (String, i64) {
        match command {
            Command::Put(KeyValue { key, value, client_id }) => {
                self.put(&key, &value);
                ("".to_string(), client_id)
            }
            Command::Delete(KeyValue { key, value: _, client_id }) => {
                self.del(&key);
                ("".to_string(), client_id)
            }
            Command::Get(KeyValue { key, value: _, client_id }) => {
                (self.get(&key), client_id)
            },
        }
    }

    fn get(&self, key: &str) -> String {
        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => String::from_utf8(value).unwrap_or("".to_string()),
            Ok(None) => "".to_string(),
            Err(_) => "".to_string(),
        }
    }

    fn put(&mut self, key: &str, value: &str) -> bool {
        match self.db.put(key.as_bytes(), value.as_bytes()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn del(&mut self, key: &str) -> bool {
        match self.db.delete(key.as_bytes()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}