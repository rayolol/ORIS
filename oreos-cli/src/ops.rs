use anyhow::Result;
use std::io::{self, Write};

use crate::casing::snake_with_suffix;
use crate::scheme::ControlNode;
use crate::scheme::Device;
use crate::templates::{backend_template, device_template, kernel_template, middleware_template};

pub fn ask(query: &str) -> io::Result<String> {
    print!("{}", query);
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().to_string())
}

/// Returns generated `(filename, contents)` pairs in a stable, deterministic order
/// (kernel, device, then backends/middleware in TOML order) so re-running `generate`
/// produces identical `mod.rs` output and log ordering every time.
pub fn generate(device: &Device) -> Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    let name = device.name.as_str();
    let state = device.state.name.as_str();
    let config = device.config.name.as_str();
    let bus = device.kernel.bus.name.as_str();
    let kernel = device.kernel.name.as_str();
    let backends: Vec<&str> = device.backends.iter().map(|b| b.name.as_str()).collect();
    let middlewares: Vec<&str> = device.middleware.iter().map(|m| m.name.as_str()).collect();

    files.push((
        "kernel.rs".to_string(),
        kernel_template(kernel, state, config, bus, name),
    ));
    files.push((
        "device.rs".to_string(),
        device_template(
            name,
            state,
            kernel,
            config,
            &middlewares.as_slice(),
            &backends.as_slice(),
        ),
    ));

    for backend in backends.iter() {
        let filename = format!("{}.rs", snake_with_suffix(backend, "backend"));
        files.push((filename, backend_template(backend)));
    }

    for middleware in middlewares.iter() {
        let filename = format!("{}.rs", snake_with_suffix(middleware, "middleware"));
        files.push((filename, middleware_template(middleware, name)));
    }

    Ok(files)
}

pub fn sync(_config_json: &str, node: &mut ControlNode) -> Result<()> {
    node.save()?;
    Ok(())
}
