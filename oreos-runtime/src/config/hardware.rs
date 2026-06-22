#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareConfig {
    pub enable_active_low: bool,
    pub dir_inverted: bool,
    pub pulse_width_ticks: u16,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            enable_active_low: true,
            dir_inverted: false,
            pulse_width_ticks: 1,
        }
    }
}
