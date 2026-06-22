#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActuatorConfig {
    pub id: u8,
    pub name: &'static str,

    pub steps_per_motor_rev: u32,
    pub microsteps: u16,

    pub gear_ratio_num: u32,
    pub gear_ratio_den: u32,

    pub min_pos_steps: i32,
    pub max_pos_steps: i32,

    pub max_vel_steps_s: u32,
    pub max_accel_steps_s2: u32,
}

impl crate::hal::Config for ActuatorConfig {}

impl Default for ActuatorConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: "actuator",
            steps_per_motor_rev: 200,
            microsteps: 16,
            gear_ratio_num: 1,
            gear_ratio_den: 1,
            min_pos_steps: 0,
            max_pos_steps: 0,
            max_vel_steps_s: 0,
            max_accel_steps_s2: 0,
        }
    }
}
