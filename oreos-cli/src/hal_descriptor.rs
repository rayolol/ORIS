use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Hal {
    Esp(ChipDescriptor),
    Stm(ChipDescriptor),
}

impl Hal {
    pub fn descriptor(&self) -> (&'static HalDescriptor, &ChipDescriptor) {
        match self {
            Self::Stm(s) => (&STM32, s),
            Self::Esp(e) => (&ESP32, e),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub chip: Hal,
}

pub struct HalDescriptor {
    pub crate_name: &'static str,
    pub peripheral_pattern: &'static str,
    pub entrypoint: &'static str,
    pub packages: &'static [CratePackage],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChipDescriptor {
    pub name: String,
    pub target: String,
    pub toolchain: String,
}

pub struct CratePackage {
    pub name: &'static str,
    pub version: &'static str,
    pub features: &'static [&'static str],
    pub chip_feature: bool,
    pub default_features: bool,
}

impl CratePackage {
    pub fn features_for(&self, chip: &str) -> Vec<String> {
        let mut features: Vec<String> = self
            .features
            .iter()
            .map(|feature| (*feature).into())
            .collect();
        if self.chip_feature && !features.iter().any(|feature| feature == chip) {
            features.push(chip.into());
        }
        features
    }
}

pub const STM32: HalDescriptor = HalDescriptor {
    crate_name: "embassy_stm32",
    peripheral_pattern: "p.{name}",
    entrypoint: "embassy_executor::main",
    packages: &[
        CratePackage {
            name: "embassy-stm32",
            version: "0.6.0",
            features: &["time-driver-any", "defmt"],
            chip_feature: true,
            default_features: true,
        },
        CratePackage {
            name: "embassy-executor",
            version: "0.10.0",
            features: &[
                "executor-thread",
                "executor-interrupt",
                "platform-cortex-m",
                "defmt",
            ],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "embassy-time",
            version: "0.5.1",
            features: &["defmt"],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "embedded-hal",
            version: "1.0.0",
            features: &[],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "defmt",
            version: "1.1.1",
            features: &[],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "defmt-rtt",
            version: "1.1.0",
            features: &[],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "panic-probe",
            version: "1.0.0",
            features: &["print-defmt"],
            chip_feature: false,
            default_features: true,
        },
    ],
};

pub const ESP32: HalDescriptor = HalDescriptor {
    crate_name: "esp_hal",
    peripheral_pattern: "p.{name}",
    entrypoint: "esp_rtos::main",
    packages: &[
        CratePackage {
            name: "esp-hal",
            version: "~1.2.0",
            features: &["unstable"],
            chip_feature: true,
            default_features: true,
        },
        CratePackage {
            name: "esp-rtos",
            version: "0.4.0",
            features: &["embassy"],
            chip_feature: true,
            default_features: true,
        },
        CratePackage {
            name: "embassy-executor",
            version: "0.10.0",
            features: &[],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "embassy-time",
            version: "0.5.1",
            features: &["defmt"],
            chip_feature: false,
            default_features: true,
        },
        CratePackage {
            name: "esp-backtrace",
            version: "0.20.0",
            features: &["panic-handler", "println"],
            chip_feature: true,
            default_features: true,
        },
        CratePackage {
            name: "esp-bootloader-esp-idf",
            version: "0.6.0",
            features: &[],
            chip_feature: true,
            default_features: true,
        },
        CratePackage {
            name: "esp-println",
            version: "0.18.0",
            features: &["defmt-espflash"],
            chip_feature: true,
            default_features: true,
        },
    ],
};
