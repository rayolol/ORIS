pub mod scheme;
pub mod ops;
pub mod scanner;
pub mod diff;
pub mod sync_engine;

use clap::Parser;
use anyhow::Result;
use scheme::{ControlNode, Middleware, Device, TypeName, Kernel, BusLane, Backend};



#[derive(Parser)]
#[command(name = "ORDL")]
#[command(about = "OREOS Device Description Language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    New {
        #[command(subcommand)]
        resource: Option<Resource>,

        #[arg(short, long)]
        target: Option<String>,
    },

    Show {
        #[arg(short, long)]
        device: Option<String>,
    },

    Generate {
        #[arg(short, long)]
        device: String,

        #[arg(short, long, default_value = "./generated")]
        output: String,
    },

    Sync
}

#[derive(clap::Subcommand)]
enum Resource {
    /// Create a new device
    Device,
    /// Create a new backend
    Backend,
    /// Create new middleware
    Middleware,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut node = ControlNode::load()?;

    match cli.command {
        Commands::New { resource, target } => {
            match resource {
                Some(Resource::Device) => create_device(&mut node)?,
                Some(Resource::Backend) => {
                    let target = target.ok_or_else(|| {
                        anyhow::anyhow!("--target DEVICE is required for backend creation")
                    })?;
                    create_backend(&mut node, target)?;
                }
                Some(Resource::Middleware) => {
                    let target = target.ok_or_else(|| {
                        anyhow::anyhow!("--target DEVICE is required for middleware creation")
                    })?;
                    create_middleware(&mut node, target)?;
                }
                None => eprintln!("Please specify a resource type: device, backend, or middleware"),
            }
        }

        Commands::Show { device } => {
            show_config(&node, device.as_deref())?;
        }

        Commands::Generate { device, output } => {
            generate_code(&node, &device, &output)?;
        }

        Commands::Sync => {
            let mut node = sync_config(node)?;
            node.save()?;
            println!("✓ Saved updated OREOS.toml");
        }
    }
    Ok(())
}


fn sync_config(mut node: ControlNode) -> Result<ControlNode> {
    use std::path::Path;
    use std::env;

    let target_dir = if Path::new("oreos-runtime/target").exists() {
        Path::new("oreos-runtime/target").to_path_buf()
    } else {
        let target = env::var("CARGO_TARGET_DIR")
            .unwrap_or_else(|_| "./target".to_string());
        Path::new(&target).to_path_buf()
    };

    let table = scanner::scan(&target_dir)?;
    let actions = diff::diff(&table, &node);

    sync_engine::apply(&actions, &mut node, &table)?;
    println!("✓ Synced metadata from code to OREOS.toml");

    Ok(node)
}

fn create_backend(node: &mut ControlNode, device_name: String) -> Result<()> {
    let name = ops::ask("Backend name: ")?;

    let backend = Backend {
        name: name.clone(),
        periph_access: None,
    };

    let device = node
        .find_device_mut(&device_name)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?;

    let dev_name = device.name.clone();
    device.backends.push(backend);
    node.save()?;

    println!("✓ Added backend '{}' to device '{}'", name, dev_name);

    Ok(())
}

fn create_middleware(node: &mut ControlNode, device_name: String) -> Result<()> {
    let name = ops::ask("Middleware name: ")?;

    let middleware = Middleware { name: name.clone() };

    let device = node
        .find_device_mut(&device_name)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?;

    let dev_name = device.name.clone();
    device.middleware = Some(middleware);
    node.save()?;

    println!("✓ Added middleware '{}' to device '{}'", name, dev_name);

    Ok(())
}

fn create_device(node: &mut ControlNode) -> Result<()> {
    let name = ops::ask("Device name: ")?;
    let state_name = ops::ask("State type name: ")?;
    let config_name = ops::ask("Config type name: ")?;

    let device = Device {
        name: name.clone(),
        state: TypeName { name: state_name },
        config: TypeName { name: config_name },
        middleware: None,
        kernel: Kernel {
            bus: BusLane {
                name: "main".to_string(),
                lane_type: "sync".to_string(),
                bound: "unbounded".to_string(),
            },
        },
        backends: vec![],
    };

    node.devices.push(device);
    node.save()?;

    println!("✓ Created device '{}'", name);

    Ok(())
}

fn show_config(node: &ControlNode, device_name: Option<&str>) -> Result<()> {
    match device_name {
        Some(name) => match node.find_device(name) {
            Some(device) => println!("{:#?}", device),
            None => println!("Device '{}' not found", name),
        },
        None => println!("{:#?}", node),
    }
    Ok(())
}

fn generate_code(node: &ControlNode, device_name: &str, output_dir: &str) -> Result<()> {
    let device = node
        .find_device(device_name)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?;

    let device_dir = format!("{}/{}", output_dir, device.name);
    std::fs::create_dir_all(&device_dir)?;

    // Generate device and kernel files
    let files = ops::generate(device)?;
    let templates = vec!["kernel.rs", "device.rs"];
    for (template, content) in templates.iter().zip(files.iter()) {
        let path = format!("{}/{}", device_dir, template);
        std::fs::write(&path, content)?;
        println!("✓ Generated {}", path);
    }

    // Generate backend folder with each backend instance
    if !device.backends.is_empty() {
        let backends_dir = format!("{}/backends", device_dir);
        std::fs::create_dir_all(&backends_dir)?;

        for backend in &device.backends {
            let backend_dir = format!("{}/{}", backends_dir, backend.name);
            std::fs::create_dir_all(&backend_dir)?;

            let backend_file = format!("{}/backend.rs", backend_dir);
            let backend_content = format!("// Backend: {}\n\npub struct {} {{\n    //TODO\n}}\n",
                backend.name, backend.name);
            std::fs::write(&backend_file, backend_content)?;
            println!("✓ Generated {}", backend_file);
        }
    }

    Ok(())
}
