use std::collections::{HashMap, BTreeSet};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TrustMap {
    map: HashMap<String, BTreeSet<String>>
}

impl TrustMap {
    pub fn from_json(json: &str) -> serde_json::Result<TrustMap> {
        let values: Vec<TrustEntry> = serde_json::from_str(json)?;
        let map = values.into_iter()
            .map(|te| {
                let levels = te.level.split(",")
                    .map(|l| l.trim().to_owned())
                    .collect();
                (te.id, levels)
            })
            .collect();
        Ok(TrustMap {
            map
        })
    }

    pub fn get(&self, id: &str) -> Option<&BTreeSet<String>> {
        self.map.get(id)
    }
}

#[derive(Deserialize)]
struct TrustEntry {
    id: String,
    level: String
}