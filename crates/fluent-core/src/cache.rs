use crate::types::Response;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sled::Db;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct StoredResponse(Response);

pub struct RequestCache {
    db: Db,
}

impl RequestCache {
    pub fn new(path: &Path) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn get(&self, key: &str) -> Result<Option<Response>> {
        if let Some(v) = self.db.get(key)? {
            let resp: StoredResponse = serde_json::from_slice(&v)?;
            Ok(Some(resp.0))
        } else {
            Ok(None)
        }
    }

    pub fn insert(&self, key: &str, response: &Response) -> Result<()> {
        let data = serde_json::to_vec(&StoredResponse(response.clone()))?;
        self.db.insert(key, data)?;
        Ok(())
    }
}

pub fn cache_key(payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}
