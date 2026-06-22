use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;
use serde_json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub uuid: u32,
    pub name: String,
    pub source: LineInfo,
    pub payload: ComponentPayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineInfo {
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ComponentPayload {
    Device { refs: DeviceRefs },
    Bus { lanes: Vec<BusLaneInfo> },
    State,
    Config,
    Kernel { bus: String },
    Middleware,
    Command,
    Create { parent: Option<String> },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceRefs {
    pub state: String,
    pub config: String,
    pub kernel_bus: String,
    pub backends: Vec<String>,
    pub middleware: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BusLaneInfo {
    pub name: String,
    pub lane_type: String,
    pub bound: String,
}

pub fn scan(target_dir: &Path) -> Result<HashMap<String, Metadata>> {
    let tmp = target_dir.join(".oreos/tmp");
    let mut table = HashMap::new();

    if !tmp.exists() {
        return Ok(table);
    }

    for entry in std::fs::read_dir(&tmp)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let json = std::fs::read_to_string(&path)?;
            match serde_json::from_str::<Metadata>(&json) {
                Ok(meta) => {
                    table.insert(meta.name.clone(), meta);
                }
                Err(e) => {
                    eprintln!("Failed to parse metadata from {}: {}", path.display(), e);
                }
            }
        }
    }
    Ok(table)
}

pub fn claimed_names(table: &HashMap<String, Metadata>) -> std::collections::HashSet<String> {
    let mut claimed = std::collections::HashSet::new();
    for meta in table.values() {
        if let ComponentPayload::Device { refs } = &meta.payload {
            claimed.insert(refs.state.clone());
            claimed.insert(refs.config.clone());
            claimed.insert(refs.kernel_bus.clone());
            claimed.extend(refs.backends.clone());
            if let Some(m) = &refs.middleware {
                claimed.insert(m.clone());
            }
        }
    }
    claimed
}
