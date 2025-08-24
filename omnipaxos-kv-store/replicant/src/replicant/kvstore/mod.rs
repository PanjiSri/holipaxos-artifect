use serde_json::Value as json;
use thiserror::Error;
use crate::replicant::kvstore::kv::Command;

use crate::replicant::kvstore::memkvstore::MemKVStore;
use crate::replicant::kvstore::rocksdbstore::RocksDBKVStore;

pub mod memkvstore;
pub mod kv;
pub mod rocksdbstore;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KVStoreError {
    #[error("key not found")]
    NotFoundError,
    #[error("put failed")]
    PutFailedError,
}

pub trait KVStore {
    fn execute(&mut self, command: Command) -> (String, i64);
    fn get(&self, key: &str) -> String;
    fn put(&mut self, key: &str, value: &str) -> bool;
    fn del(&mut self, key: &str) -> bool;
}

pub fn create_store(config: &json) -> Box<dyn KVStore + Sync + Send> {
    let store_type = config["store"].as_str().unwrap();
    if store_type == "rocksdb" {
        return Box::new(RocksDBKVStore::new(
            config["db_path"].as_str().unwrap()));
    } else if store_type == "mem" {
        return Box::new(MemKVStore::new());
    } else {
        assert!(false);
    }
    return Box::new(MemKVStore::new());
}
