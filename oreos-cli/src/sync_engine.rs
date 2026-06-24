use std::collections::HashMap;
use anyhow::Result;
use crate::scanner::{Metadata, ComponentPayload};
use crate::scheme::{ControlNode, Device, Kernel, BusLane, Backend, Middleware, TypeName, UnresolvedComponent};
use crate::diff::SyncAction;

fn resolve_bus(
    name: &str,
    table: &HashMap<String, Metadata>,
) -> Option<BusLane> {
    let meta = table.get(name)?;
    let ComponentPayload::Bus { lanes } = &meta.payload else { return None };
    let first = lanes.first()?;
    Some(BusLane {
        name: name.to_string(),
        lane_type: first.lane_type.clone(),
        bound: first.bound.clone(),
    })
}

pub fn resolve_device(
    meta: &Metadata,
    table: &HashMap<String, Metadata>,
) -> Option<Device> {
    let ComponentPayload::Device { refs } = &meta.payload else { return None };
    Some(Device {
        name: meta.name.clone(),
        state: TypeName { name: refs.state.clone() },
        config: TypeName { name: refs.config.clone() },
        kernel: Kernel {name: refs.kernel_bus.clone(), bus: resolve_bus(&refs.kernel_bus, table)? },
        backends: refs.backends.iter().map(|b| Backend { name: b.clone(), periph_access: None }).collect(),
        middleware: refs.middleware.iter().map(|m| Middleware {name: m.clone()}).collect()
    })
}

fn kind_str(payload: &ComponentPayload) -> String {
    match payload {
        ComponentPayload::Device { .. } => "device",
        ComponentPayload::Bus { .. } => "bus",
        ComponentPayload::State => "state",
        ComponentPayload::Config => "config",
        ComponentPayload::Kernel { .. } => "kernel",
        ComponentPayload::Middleware => "middleware",
        ComponentPayload::Command => "command",
        ComponentPayload::Create { .. } => "create",
    }.to_string()
}

pub fn apply(
    actions: &[SyncAction],
    config: &mut ControlNode,
    table: &HashMap<String, Metadata>,
) -> Result<()> {
    for action in actions {
        match action {
            SyncAction::AddToToml(meta) => {
                if let Some(device) = resolve_device(meta, table) {
                    config.devices.push(device);
                    println!("Added device '{}'", meta.name);
                }
            }
            SyncAction::WarnDangling(meta) => {
                config.unresolved.push(UnresolvedComponent {
                    uuid: meta.uuid,
                    name: meta.name.clone (),
                    kind: kind_str(&meta.payload),
                    source: format!("{}:{}", meta.source.file, meta.source.line),
                });
                println!("Registered '{}' as unresolved (dangling)", meta.name);
            }
            SyncAction::WarnDeleteBlocked(name) => {
                println!("{} — remove from code first", name);
            }
            _ => {}
        }
    }
    Ok(())
}
