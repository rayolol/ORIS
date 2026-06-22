// API-facing structs

#[derive(Clone)]
pub struct Tmc2160Config {
    pub run_current_ma: u16,
    pub hold_current_ma: u16,
    pub sense_resistor_mohm: u16,
    /// Global current scaler: 0 = full scale (256/256), 32–255 = (value/256) × I_max
    pub global_scaler: u8,
    pub microsteps: u16,
    pub interpolation: bool,
    pub shaft_reverse: bool,
    pub stealthchop: bool,
    pub stealthchop_threshold: Option<u32>,
    pub stallguard: Option<StallGuardConfig>,
    pub advanced: AdvancedConfig,
}

impl Default for Tmc2160Config {
    fn default() -> Self {
        Self {
            run_current_ma: 800,
            hold_current_ma: 200,
            sense_resistor_mohm: 110,
            global_scaler: 0,
            microsteps: 16,
            interpolation: true,
            shaft_reverse: false,
            stealthchop: true,
            stealthchop_threshold: None,
            stallguard: None,
            advanced: AdvancedConfig::default(),
        }
    }
}

impl Tmc2160Config {
    pub fn with_run_current_ma(mut self, ma: u16) -> Self {
        self.run_current_ma = ma;
        self
    }

    pub fn with_hold_current_ma(mut self, ma: u16) -> Self {
        self.hold_current_ma = ma;
        self
    }

    pub fn with_sense_resistor_mohm(mut self, mohm: u16) -> Self {
        self.sense_resistor_mohm = mohm;
        self
    }

    pub fn with_global_scaler(mut self, scaler: u8) -> Self {
        self.global_scaler = scaler;
        self
    }

    pub fn with_microsteps(mut self, microsteps: u16) -> Self {
        self.microsteps = microsteps;
        self
    }

    pub fn with_interpolation(mut self, enabled: bool) -> Self {
        self.interpolation = enabled;
        self
    }

    pub fn with_shaft_reverse(mut self, reversed: bool) -> Self {
        self.shaft_reverse = reversed;
        self
    }

    pub fn with_stealthchop(mut self, enabled: bool) -> Self {
        self.stealthchop = enabled;
        self
    }

    pub fn with_stealthchop_threshold(mut self, threshold: u32) -> Self {
        self.stealthchop_threshold = Some(threshold);
        self
    }

    pub fn with_stallguard(mut self, config: StallGuardConfig) -> Self {
        self.stallguard = Some(config);
        self
    }

    pub fn with_advanced(mut self, config: AdvancedConfig) -> Self {
        self.advanced = config;
        self
    }
}

#[derive(Clone)]
pub struct StallGuardConfig {
    /// stallGuard2 threshold (−64 to +63; higher = less sensitive)
    pub threshold: i8,
    /// TSTEP value below which stallGuard and coolStep are enabled
    pub velocity_threshold: u32,
    pub coolstep: Option<CoolStepConfig>,
}

impl Default for StallGuardConfig {
    fn default() -> Self {
        Self {
            threshold: 10,
            velocity_threshold: 0,
            coolstep: None,
        }
    }
}

impl StallGuardConfig {
    pub fn with_threshold(mut self, threshold: i8) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_velocity_threshold(mut self, tstep: u32) -> Self {
        self.velocity_threshold = tstep;
        self
    }

    pub fn with_coolstep(mut self, config: CoolStepConfig) -> Self {
        self.coolstep = Some(config);
        self
    }
}

#[derive(Clone)]
pub struct CoolStepConfig {
    pub semin: u8,
    pub semax: u8,
    pub seup: u8,
    pub sedn: u8,
    pub quarter_current: bool,
}

impl Default for CoolStepConfig {
    fn default() -> Self {
        Self {
            semin: 5,
            semax: 2,
            seup: 0,
            sedn: 0,
            quarter_current: false,
        }
    }
}

#[derive(Clone)]
pub struct AdvancedConfig {
    // Chopper
    pub toff: u8,
    pub hstrt: u8,
    pub hend: u8,
    pub tbl: u8,
    pub chm: bool, // false = spreadCycle, true = classic constant off-time
    // Timing
    pub iholddelay: u8,
    pub tpowerdown: u8,
    // PWM (stealthChop)
    pub freewheel: FreewheelMode,
    pub pwm_freq: u8,
    pub pwm_autoscale: bool,
    pub pwm_autograd: bool,
    pub pwm_reg: u8,
    pub pwm_lim: u8,
    pub pwm_ofs: u8,
    pub pwm_grad: u8,
    // Driver
    pub drvstrength: u8,
    pub bbmtime: u8,
    pub bbmclks: u8,
    pub filt_isense: u8,
    // Short-circuit detection
    pub s2vs_level: u8,
    pub s2gnd_level: u8,
    pub multistep_filt: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            toff: 3,
            hstrt: 4,
            hend: 1,
            tbl: 2,
            chm: false,
            iholddelay: 8,
            tpowerdown: 20,
            freewheel: FreewheelMode::Normal,
            pwm_freq: 1,
            pwm_autoscale: true,
            pwm_autograd: true,
            pwm_reg: 8,
            pwm_lim: 12,
            pwm_ofs: 36,
            pwm_grad: 14,
            drvstrength: 2,
            bbmtime: 0,
            bbmclks: 4,
            filt_isense: 0,
            s2vs_level: 6,
            s2gnd_level: 6,
            multistep_filt: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FreewheelMode {
    Normal = 0,
    Freewheel = 1,
    CoilShortLS = 2,
    CoilShortHS = 3,
}

pub enum Tmc2160Error<E> {
    Transport(E),
    NoReply,
    InvalidMicrosteps,
    CurrentOutOfRange,
    InvalidParameter,
}

#[derive(Clone, Copy, Default)]
pub struct DriverStatus {
    pub standstill: bool,
    pub stealth_active: bool,
    pub fullstep_active: bool,
    pub cs_actual: u8,
    pub sg_result: u16,
    pub stall_detected: bool,
    pub over_temp_prewarning: bool,
    pub over_temp_shutdown: bool,
    pub short_to_supply_a: bool,
    pub short_to_supply_b: bool,
    pub short_to_gnd_a: bool,
    pub short_to_gnd_b: bool,
    pub open_load_a: bool,
    pub open_load_b: bool,
}

impl DriverStatus {
    pub fn from_raw(reg: super::registers::DRV_STATUS) -> Self {
        Self {
            standstill: reg.stst(),
            stealth_active: reg.stealth(),
            fullstep_active: reg.fsactive(),
            cs_actual: reg.cs_actual().into(),
            sg_result: reg.sg_result().into(),
            stall_detected: reg.sg_status(),
            over_temp_prewarning: reg.otpw(),
            over_temp_shutdown: reg.ot(),
            short_to_supply_a: reg.s2vsa(),
            short_to_supply_b: reg.s2vsb(),
            short_to_gnd_a: reg.s2ga(),
            short_to_gnd_b: reg.s2gb(),
            open_load_a: reg.ola(),
            open_load_b: reg.olb(),
        }
    }
}

pub struct GlobalStatus {
    pub reset: bool,
    pub driver_error: bool,
    pub undervoltage: bool,
}
