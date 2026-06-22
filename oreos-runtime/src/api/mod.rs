pub enum ActuatorCommand {
    Enable,
    Disable,
    SetVelocity(f32),
    SetPosition(f32),
    SetTorque(f32),
}

#[derive(Clone, Copy)]
pub struct MotorState {
    pub velocity_rad_s: f32,
    pub position_rad: f32,
}

#[derive(Clone, Copy)]
pub struct SensorState {
    pub raw: f32,
}
