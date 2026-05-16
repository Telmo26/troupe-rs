use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TrustMap {
    map: HashMap<String, String>
}

impl TrustMap {
    pub fn from_json(json: &str) -> serde_json::Result<TrustMap> {
        let values: Vec<TrustEntry> = serde_json::from_str(json)?;
        let map = values.into_iter()
            .map(|te| (te.id, te.level))
            .collect();
        Ok(TrustMap {
            map
        })
    }
}

#[derive(Deserialize)]
struct TrustEntry {
    id: String,
    level: String
}