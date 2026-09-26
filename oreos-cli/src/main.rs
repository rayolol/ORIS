pub mod casing;
pub mod diff;
pub mod ops;
pub mod scanner;
pub mod scheme;
pub mod sync_engine;
pub mod templates;

use anyhow::Result;
use clap::Parser;
use scheme::{Backend, BusLane, ControlNode, Device, Kernel, Middleware, TypeName};

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
        from: Option<String>,
    },

    Show {
        #[arg(short, long)]
        device: Option<String>,
    },

    Generate {
        #[arg(short, long)]
        device: String,

        /// Generate only this backend instead of regenerating the whole device
        #[arg(short, long)]
        backend: Option<String>,

        #[arg(short, long, default_value = "./src")]
        output: String,
    },

    Sync,
}

#[derive(clap::Subcommand)]
enum Resource {
    /// Create a new device
    Device,
    /// Create a new backend
    Backend,
    /// Create new middleware
    Middleware,
    /// Create a new runtime
    Runtime,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut node = ControlNode::load()?;

    match cli.command {
        Commands::New { resource, from } => match resource {
            Some(Resource::Device) => create_device(&mut node)?,
            Some(Resource::Backend) => {
                let target = from.ok_or_else(|| {
                    anyhow::anyhow!("--target DEVICE is required for backend creation")
                })?;
                create_backend(&mut node, target)?;
            }
            Some(Resource::Middleware) => {
                let target = from.ok_or_else(|| {
                    anyhow::anyhow!("--target DEVICE is required for middleware creation")
                })?;
                create_middleware(&mut node, target)?;
            }
            Some(Resource::Runtime) => create_runtime()?,
            None => eprintln!("Please specify a resource type: device, backend, or middleware"),
        },

        Commands::Show { device } => {
            show_config(&node, device.as_deref())?;
        }

        Commands::Generate {
            device,
            backend,
            output,
        } => {
            if let Some(backend) = backend {
                generate_backend_code(&node, &device, &backend, &output)?;
            } else {
                generate_code(&node, &device, &output)?;
            }
        }

        Commands::Sync => {
            let node = sync_config(node)?;
            node.save()?;
            println!("✓ Saved updated OREOS.toml");
        }
    }
    Ok(())
}

fn create_runtime() -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let src_dir = std::env::current_dir()?.join("src");
    let main_path = src_dir.join("main.rs");

    std::fs::create_dir_all(&src_dir)?;

    if main_path.exists() {
        let answer = ops::ask("src/main.rs aleady exists, Overwrite it? [y/N]: ")?;

        if !answer.eq_ignore_ascii_case("y") && !answer.eq_ignore_ascii_case("yes") {
            println!("Cancelled; existing src/main.rs was not changed.");
            return Ok(());
        }
    }

    let code = templates::runtime_base_template::create_base_runtime();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&main_path)
        .map_err(|error| {
            anyhow::anyhow!(
                "could not create {} without overwriting it: {}",
                main_path.display(),
                error
            )
        })?;
    file.write_all(code.as_bytes())?;

    println!("Created {}", main_path.display());
    Ok(())
}

fn sync_config(mut node: ControlNode) -> Result<ControlNode> {
    use std::env;
    use std::path::Path;

    let target_dir = if Path::new("oreos-runtime/target").exists() {
        Path::new("oreos-runtime/target").to_path_buf()
    } else {
        let target = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "./target".to_string());
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
    device.middleware.push(middleware);
    node.save()?;

    println!("Added middleware '{}' to device '{}'", name, dev_name);

    Ok(())
}

fn create_device(node: &mut ControlNode) -> Result<()> {
    let name = ops::ask("Device name: ")?;
    let state_name = ops::ask("State type name: ")?;
    let config_name = ops::ask("Config type name: ")?;
    let bus_name = ops::ask("bus name")?;

    let device = Device {
        name: name.clone(),
        state: TypeName { name: state_name },
        config: TypeName { name: config_name },
        middleware: vec![],
        kernel: Kernel {
            name: format!("{}_kernel", name),
            bus: BusLane {
                name: bus_name.to_string(),
                lane_type: "sync".to_string(),
                bound: "unbounded".to_string(),
            },
        },
        backends: vec![],
    };

    node.devices.push(device);
    node.save()?;

    println!("Created device '{}'", name);

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

    let module_name = casing::to_snake_case(&device.name);
    let device_dir = format!("{}/{}", output_dir, module_name);
    std::fs::create_dir_all(&device_dir)?;

    let files = ops::generate(device)?;
    let mut module_declarations = String::new();

    for (name, content) in files.iter() {
        let path = format!("{}/{}", device_dir, name);
        std::fs::write(&path, content)?;
        println!("Generated {}", path);

        let module_name = name.trim_end_matches(".rs");
        module_declarations.push_str(&format!("pub mod {};\n", module_name));
    }

    let mod_path = format!("{}/mod.rs", device_dir);
    std::fs::write(&mod_path, module_declarations)?;
    println!("Generated {}", mod_path);

    Ok(())
}

fn generate_backend_code(
    node: &ControlNode,
    device_name: &str,
    backend_name: &str,
    output_dir: &str,
) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let device = node
        .find_device(device_name)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?;

    let backend = device
        .backends
        .iter()
        .find(|backend| backend.name == backend_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Backend '{}' is not registered on device '{}'",
                backend_name,
                device_name
            )
        })?;

    let device_dir = std::path::Path::new(output_dir).join(casing::to_snake_case(&device.name));
    std::fs::create_dir_all(&device_dir)?;

    let filename = format!("{}.rs", casing::snake_with_suffix(&backend.name, "backend"));
    let path = device_dir.join(filename);
    // FIXME: IO access is not yet implemented for backends, so we pass None for now. Once it is implemented, we can pass the correct IO access type here, coming from backend.periph_access.
    let content = templates::backend_template(&backend.name, None);

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow::anyhow!("Refusing to overwrite existing backend {}", path.display())
            } else {
                anyhow::Error::new(error).context(format!("Failed to create {}", path.display()))
            }
        })?;

    file.write_all(content.as_bytes())?;
    println!("Generated {}", path.display());

    Ok(())
}
