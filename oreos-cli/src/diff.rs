use std::collections::HashMap;
use crate::scanner::{Metadata, ComponentPayload};
use crate::scheme::ControlNode;

#[derive(Debug, Clone)]
pub enum SyncAction {
    AddToToml(Metadata),
    WarnDeleteBlocked(String),
    UpdateRefs(Metadata),
    WarnDangling(Metadata),
    NoOp,
}

pub fn diff(table: &HashMap<String, Metadata>, config: &ControlNode) -> Vec<SyncAction> {
    let mut actions = vec![];
    let claimed = super::scanner::claimed_names(table);

    // Pass 1 — in code not in TOML
    for meta in table.values() {
        if matches!(meta.payload, ComponentPayload::Device { .. }) {
            if !config.devices.iter().any(|d| d.name == meta.name) {
                actions.push(SyncAction::AddToToml(meta.clone()));
            }
        }
    }

    // Pass 2 — dangling (in code, unclaimed)
    for meta in table.values() {
        if !matches!(meta.payload, ComponentPayload::Device { .. }) {
            if !claimed.contains(&meta.name) {
                actions.push(SyncAction::WarnDangling(meta.clone()));
            }
        }
    }

    // Pass 3 — in TOML not in code (blocked delete)
    for device in &config.devices {
        if !table.contains_key(&device.name) {
            actions.push(SyncAction::WarnDeleteBlocked(device.name.clone()));
        }
    }

    actions
}
