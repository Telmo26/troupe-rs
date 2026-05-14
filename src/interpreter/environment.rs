use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;

use crate::interpreter::runtime_value::RuntimeValue;


pub type EnvPtr = Arc<Environment>;

#[derive(Debug, Default, Clone)]
pub struct Environment {
    parent: Option<EnvPtr>,
    bindings: DashMap<String, RuntimeValue>
}

impl Environment {
    pub fn new_child(parent: EnvPtr) -> EnvPtr {
        Arc::new(Environment {
            parent: Some(parent),
            bindings: DashMap::new(),
        })
    }

    pub fn get(&self, key: &str) -> Option<RuntimeValue> {
        if let Some(v) = self.bindings.get(key) {
            return Some(v.clone());
        }

        match &self.parent {
            Some(parent) => parent.get(key),
            None => None,
        }
    }

    pub fn insert(&self, key: String, value: RuntimeValue) -> Option<RuntimeValue> {
        self.bindings.insert(key, value)
    }

    pub fn extend(&self, bindings: HashMap<String, RuntimeValue>) {
        bindings.into_iter().for_each(|(name, value)| {
            self.bindings.insert(name, value);
        });
    }
}