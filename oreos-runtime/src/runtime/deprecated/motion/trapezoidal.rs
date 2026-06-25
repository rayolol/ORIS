use libm::sqrtf;

#[derive(Debug, Clone)]
pub(crate) enum MotionPlanners {
    Trapezoidal(Trapezoidal),
    Home(Home),
}

impl MotionPlanners {
    pub fn plan(&mut self, config: &KernelConfig, state: &KernelState, target: f32) {
        match self {
            Self::Trapezoidal(t) => {
                t.plan_absolute_move(
                    target,
                    state.current_pos_rad,
                    config.max_vel_rad_s as f32,
                    config.max_accel_rad_s2 as f32,
                );
            }
            Self::Home(h) => {
                h.plan_absolute_move(config.max_vel_rad_s as f32);
            }
        }
    }

    pub fn update(&mut self, dt: u32, state: &KernelState) -> MotionSetpoint {
        match self {
            Self::Trapezoidal(t) => t.update(dt),
            Self::Home(h) => h.update(dt, state.stalled),
        }
    }
}

pub trait MotionPlanner {
    fn update(&mut self, dt: u32) -> MotionSetpoint;
    fn plan_absolute_move(&mut self, target: f32, current: f32, max_vel: f32, max_acc: f32);
}

// _____Homing Sequence_________

#[derive(Debug, Clone)]
enum HomeState {
    Searching1,
    Backoff1,
    Searching2,
    Backoff2,
    Comparing,
    MovingToRest,
    Done,
}

#[derive(Debug, Clone)]
pub struct Home {
    state: HomeState,
    active: bool,
    direction: i32,
    current_vel: f32,
    current_rad: f32,
    delta_pos: f32,
    stalled: bool,
    stall1_pos: f32,
    stall2_pos: f32,
    max_home_vel: f32,
}

impl Home {
    fn plan_absolute_move(&mut self, max_vel: f32) {
        self.active = true;
    }

    fn update(&mut self, dt: u32, stalled: bool) -> MotionSetpoint {
        match self.state {
            HomeState::Searching1 => {
                if stalled {
                    self.current_vel = -self.current_vel;
                    self.stall1_pos = self.current_rad;
                    self.state = HomeState::Backoff1;
                }
                return MotionSetpoint {
                    vel_rad_s: self.current_vel,
                    done: false,
                };
            }
            HomeState::Backoff1 => {
                if self.current_rad == self.current_rad - (self.delta_pos / 2.0) {
                    self.state = HomeState::Searching2;
                    self.current_vel = -self.current_vel;
                }
                return MotionSetpoint {
                    vel_rad_s: self.current_vel,
                    done: false,
                };
            }
            HomeState::Backoff2 => {
                if self.current_rad == self.current_rad - (self.delta_pos / 2.0) {
                    self.state = HomeState::Comparing;
                }
                return MotionSetpoint {
                    vel_rad_s: 0.0,
                    done: false,
                };
            }
            HomeState::Searching2 => {
                if stalled {
                    self.current_vel = -self.current_vel;
                    self.stall2_pos = self.current_rad;
                    self.state = HomeState::Backoff2;
                }
                return MotionSetpoint {
                    vel_rad_s: self.current_vel,
                    done: false,
                };
            }
            HomeState::Comparing => {
                if ((self.stall1_pos - self.stall2_pos).abs() > 2.0) {}
                return MotionSetpoint {
                    vel_rad_s: 0.0,
                    done: true,
                };
            }
            HomeState::Done => {
                return MotionSetpoint {
                    vel_rad_s: 0.0,
                    done: true,
                };
            }
            HomeState::MovingToRest => {
                return MotionSetpoint {
                    vel_rad_s: 0.0,
                    done: true,
                };
            }
            _ => {
                return MotionSetpoint {
                    vel_rad_s: 0.0,
                    done: true,
                };
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionSetpoint {
    pub vel_rad_s: f32,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Trapezoidal {
    active: bool,
    delta_rad: f32,
    current_rad: f32,
    distance: f32,
    max_vel_rad_s: f32,
    max_accel_rad_s2: f32,
    direction: i32,
    t_vel_us: u32,
    delta_t_us: u32,
    t_accel_us: u32,
    elapsed_us: u32,
    v_peak: f32,
}

impl Trapezoidal {
    pub fn new() -> Self {
        Self {
            active: false,
            delta_rad: 0.0,
            current_rad: 0.0,
            distance: 0.0,
            max_vel_rad_s: 0.0,
            max_accel_rad_s2: 0.0,
            direction: 1,
            t_vel_us: 0,
            delta_t_us: 0,
            t_accel_us: 0,
            elapsed_us: 0,
            v_peak: 0.0,
        }
    }

    pub fn plan_absolute_move(
        &mut self,
        target_rad: f32,
        current_rad: f32,
        max_vel_rad_s: f32,
        max_accel_rad_s2: f32,
    ) {
        self.current_rad = current_rad;
        self.max_vel_rad_s = max_vel_rad_s.abs();
        self.max_accel_rad_s2 = max_accel_rad_s2.abs();
        self.elapsed_us = 0;

        self.delta_rad = target_rad - self.current_rad;
        self.direction = if self.delta_rad >= 0.0 { 1 } else { -1 };
        self.distance = self.delta_rad.abs();

        if self.distance == 0.0 || self.max_vel_rad_s <= 0.0 || self.max_accel_rad_s2 <= 0.0 {
            self.clear();
            return;
        }

        let accel = self.max_accel_rad_s2;
        let vel = self.max_vel_rad_s;
        let dist = self.distance;

        let t_accel_s = vel / accel;
        let d_accel = 0.5 * accel * t_accel_s * t_accel_s;

        if 2.0 * d_accel > dist {
            self.v_peak = sqrtf(dist * accel);
            let t_accel_s = self.v_peak / accel;
            self.t_accel_us = (t_accel_s * 1_000_000.0) as u32;
            self.t_vel_us = 0;
        } else {
            self.v_peak = vel;
            self.t_accel_us = (t_accel_s * 1_000_000.0) as u32;
            let t_vel_s = (dist - 2.0 * d_accel) / vel;
            self.t_vel_us = (t_vel_s * 1_000_000.0) as u32;
        }

        self.delta_t_us = 2 * self.t_accel_us + self.t_vel_us;
        self.active = true;
    }

    pub fn update(&mut self, dt: u32) -> MotionSetpoint {
        if !self.active {
            return MotionSetpoint {
                vel_rad_s: 0.0,
                done: true,
            };
        }

        self.elapsed_us += dt;
        let t = self.elapsed_us;

        let velocity = if t < self.t_accel_us {
            let t_s = t as f32 / 1_000_000.0;
            self.max_accel_rad_s2 * t_s
        } else if t < self.t_accel_us + self.t_vel_us {
            self.v_peak
        } else if t < self.delta_t_us {
            let t_dec = t - (self.t_accel_us + self.t_vel_us);
            let t_dec_s = t_dec as f32 / 1_000_000.0;
            self.v_peak - self.max_accel_rad_s2 * t_dec_s
        } else {
            self.current_rad += self.delta_rad;
            self.clear();
            return MotionSetpoint {
                vel_rad_s: 0.0,
                done: true,
            };
        };

        MotionSetpoint {
            vel_rad_s: self.direction as f32 * velocity,
            done: false,
        }
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.delta_rad = 0.0;
        self.distance = 0.0;
        self.delta_t_us = 0;
        self.t_accel_us = 0;
        self.t_vel_us = 0;
        self.elapsed_us = 0;
        self.v_peak = 0.0;
    }
}
