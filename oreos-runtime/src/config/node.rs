#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeConfig {
    pub node_id: u8,
    pub name: &'static str,
    pub control_hz: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            name: "mcn-node",
            control_hz: 10_000,
        }
    }
}
