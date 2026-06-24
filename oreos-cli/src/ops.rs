use std::io::{self, Write};
use std::collections::HashMap;
use anyhow::Result;

use crate::scheme::{ControlNode, Device, Middleware};
use crate::templates::{ backend_template, middleware_template, kernel_template, device_template};

pub fn ask(query: &str) -> io::Result<String> {
    print!("{}", query);
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().to_string())
}

pub fn generate(device: &Device) -> Result<HashMap<String, String>> {

    let mut files = HashMap::new();
    let name = device.name.as_str();
    let state = device.state.name.as_str();
    let config = device.config.name.as_str();
    let bus = device.kernel.bus.name.as_str();
    let kernel = device.kernel.name.as_str();
    let backends: Vec<&str> = device.backends.iter().map(|b| b.name.as_str()).collect();
    let middlewares: Vec<&str> = device.middleware.iter().map(|m| m.name.as_str()).collect();

    for backend in backends.iter() {
        files.insert(format!("{}_backend.rs", backend), backend_template(backend));
    }

    for middleware in middlewares.iter() {
        files.insert(format!("{}_middleware.rs", middleware), middleware_template(middleware));
    }

    files.insert("kernel.rs".to_string(), kernel_template(kernel, state, config, bus));
    files.insert("device.rs".to_string(), device_template(name, state, kernel, config, &middlewares.as_slice(), &backends.as_slice()));


    Ok(files)
}

pub fn sync(_config_json: &str, node: &mut ControlNode) -> Result<()> {
    node.save()?;
    Ok(())
}