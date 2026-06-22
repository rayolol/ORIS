use std::io::{self, Write};
use tera::{Tera, Context};
use anyhow::Result;

use crate::scheme::{Device, ControlNode};

pub fn ask(query: &str) -> io::Result<String> {
    print!("{}", query);
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().to_string())
}

pub fn generate(device: &Device) -> Result<Vec<String>> {
    let tera = Tera::new("oreos-cli/src/templates/**/*.tera")?;
    let mut files = vec![];

    let mut ctx = Context::new();
    ctx.insert("device", device);

    files.push(tera.render("backend.rs.tera", &ctx)?);
    files.push(tera.render("kernel.rs.tera", &ctx)?);
    files.push(tera.render("device.rs.tera", &ctx)?);

    Ok(files)
}

pub fn sync(_config_json: &str, node: &mut ControlNode) -> Result<()> {
    node.save()?;
    Ok(())
}