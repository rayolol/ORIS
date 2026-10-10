use crate::hal_descriptor::ProjectConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

pub const OREOS_FILE_PATH: &str = "OREOS.toml";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OreosConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectConfig>,

    #[serde(flatten)]
    pub control_node: ControlNode,
}

impl OreosConfig {
    pub fn load() -> Result<Self> {
        if !Path::new(OREOS_FILE_PATH).exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(OREOS_FILE_PATH)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self) -> Result<()> {
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(OREOS_FILE_PATH, toml)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ControlNode {
    pub name: String,
    pub devices: Vec<Device>,
    #[serde(default)]
    pub unresolved: Vec<UnresolvedComponent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnresolvedComponent {
    pub uuid: u32,
    pub name: String,
    pub kind: String,
    pub source: String,
}

impl ControlNode {
    pub fn load() -> Result<Self> {
        if !Path::new(OREOS_FILE_PATH).exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(OREOS_FILE_PATH)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self) -> Result<()> {
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(OREOS_FILE_PATH, toml)?;
        Ok(())
    }

    pub fn find_device_mut(&mut self, name: &str) -> Option<&mut Device> {
        self.devices.iter_mut().find(|d| d.name == name)
    }

    pub fn find_device(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub name: String,
    pub state: TypeName,
    pub config: TypeName,
    pub middleware: Vec<Middleware>,
    pub kernel: Kernel,
    pub backends: Vec<Backend>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Middleware {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Kernel {
    pub name: String,
    pub bus: BusLane,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BusLane {
    pub name: String,
    pub lane_type: String,
    pub bound: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Backend {
    pub name: String,
    pub periph_access: Option<PeripheralAccess>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeripheralAccess {
    pub name: String,
    pub field: Vec<PeripheralPin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PeripheralPinType {
    Pwm,
    Analog,
    Digital,
    Transport,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeripheralPin {
    pub name: String,
    pub kind: PeripheralPinType,
    pub pins: Vec<PinId>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Hash, Deserialize, Clone)]
#[serde(transparent)]
pub struct PinId(pub String);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeName {
    pub name: String,
}
