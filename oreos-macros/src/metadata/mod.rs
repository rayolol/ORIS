use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub uuid: u32,
    pub name: String,
    pub source: LineInfo,
    pub payload: ComponentPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceRefs {
    pub state: String,
    pub config: String,
    pub kernel_bus: String,
    pub backends: Vec<String>,
    pub middleware: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BusLaneInfo {
    pub name: String,
    pub lane_type: String,
    pub bound: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LineInfo {
    pub file: String,
    pub line: u32,
}

impl Metadata {
    pub fn new(name: String, source: LineInfo, payload: ComponentPayload) -> Self {
        let crate_name = std::env::var("CARGO_CRATE_NAME").unwrap_or_else(|_| "unknown".to_string());
        let uuid = stable_uuid(&crate_name, &name);
        Metadata { uuid, name, source, payload }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = get_file_meta(&self.name)?;
        let json = serde_json::to_string_pretty(self)?;
        match std::fs::write(&path, &json) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Failed to write metadata to {}: {}", path, e);
                Err(e)
            }
        }
    }
}

pub fn stable_uuid(crate_name: &str, item_name: &str) -> u32 {
    let input = format!("{}::{}", crate_name, item_name);
    simplehash::fnv1a_32(input.as_bytes())
}

pub fn get_file_meta(file_name: &str) -> Result<String, std::io::Error> {
    let crate_name = std::env::var("CARGO_CRATE_NAME").unwrap_or_else(|_| "unknown".to_string());
    let mut current = std::env::current_dir()?;
    let mut found_root = false;

    for _ in 0..10 {
        if current.join("Cargo.lock").exists() || current.join("Cargo.toml").exists() {
            if current.join("target").exists() {
                found_root = true;
                break;
            }
        }
        if !current.pop() {
            break;
        }
    }

    let metadata_dir = if found_root {
        current.join("target/.oreos/tmp")
    } else {
        std::path::PathBuf::from(".oreos/tmp")
    };

    std::fs::create_dir_all(&metadata_dir)?;
    let file_path = metadata_dir.join(format!("{}_{}-{}.json", crate_name, file_name, stable_uuid(&crate_name, file_name)));
    Ok(file_path.to_string_lossy().to_string())
}
