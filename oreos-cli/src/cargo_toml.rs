use crate::hal_descriptor::{Hal, HalDescriptor};
use anyhow::{bail, Context, Result};
use std::{fmt::format, path::Path, process::Command};
use toml_edit::{Array, DocumentMut, Item, Value};

pub fn create_cargo_project(path: &Path, chip: &Hal) -> Result<()> {
    let status = Command::new("cargo")
        .args(["new", "--bin", "--edition", "2024"])
        .arg("path")
        .status()
        .context("could not start cargo")?;

    if !status.success() {
        bail!("cargo new failed: {status}")
    }

    let (hal_desc, chip_desc) = chip.descriptor();

    // generates .cargo folder
    let target = &chip_desc.target;
    let cargo_folder = path.join(".cargo");
    let file_path = cargo_folder.join("config.toml");
    std::fs::write(&file_path, format!("[build]\ntarget = \"{target}\"\n",))?;

    // generates rust toolchain
    let channel = &chip_desc.toolchain;
    let rust_toolchain_file = path.join("rust-toolchain.toml");
    std::fs::write(
        &rust_toolchain_file,
        format!("[toolchain]\nchannel = \"{channel}\"\n",),
    )?;

    let contents = std::fs::read_to_string("Cargo.toml")?;
    let mut doc = contents.parse::<DocumentMut>()?;

    let dependencies = doc
        .get_mut("dependencies")
        .and_then(Item::as_table_mut)
        .context("missing [dependencies] table")?;

    for dep in hal_desc.packages {
        let mut inline_table = toml_edit::InlineTable::new();
        inline_table.insert("version", Value::from(dep.version));
        let features: toml_edit::Array = dep.features_for(&chip_desc.name).into_iter().collect();
        inline_table.insert("features", Value::Array(features));
        inline_table.insert("default-features", Value::from(dep.default_features));

        dependencies.insert(dep.name, Item::Value(Value::InlineTable(inline_table)));
    }

    // dependencies.insert("oreos-runtime", item)

    std::fs::write(path, doc.to_string())?;

    Ok(())
}
