
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static REGISTRY: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));


pub (crate) fn store(namespace: &str, name: &str, token: String) {
    REGISTRY.lock().unwrap().insert(format!("{}::{}", namespace, name), token);
}

pub (crate) fn fetch(namespace: &str, name: &str) -> Option<String>{
    REGISTRY.lock().unwrap().get(&format!("{}::{}", namespace, name)).cloned()
}