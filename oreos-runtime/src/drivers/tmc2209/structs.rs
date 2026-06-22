
//API facing structs

#[derive(Clone)]
pub struct Tmc2209Config {
    pub run_current_ma: u16,
    pub hold_current_ma: u16,
    pub sense_resistor_mohm: u16,
    pub microsteps: u16,
    pub interpolation: bool,
    pub shaft_reverse: bool,
    pub stealthchop: bool,
    pub stealthchop_threshold: Option<u32>,
    pub stallguard: Option<StallGuardConfig>,
    pub advanced: AdvancedConfig,
}

impl Default for Tmc2209Config {
    fn default() -> Self {
        Self {
            run_current_ma: 800,
            hold_current_ma: 400,
            sense_resistor_mohm: 110,
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

impl Tmc2209Config {
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
    pub threshold: u8,
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
    pub fn with_threshold(mut self, threshold: u8) -> Self {
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
    pub toff: u8,
    pub hstrt: u8,
    pub hend: u8,
    pub tbl: u8,
    pub iholddelay: u8,
    pub tpowerdown: u8,
    pub freewheel: FreewheelMode,
    pub pwm_freq: u8,
    pub pwm_autoscale: bool,
    pub pwm_autograd: bool,
    pub pwm_reg: u8,
    pub pwm_ofs: u8,
    pub pwm_grad: u8,
    pub multistep_filt: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            toff: 3,
            hstrt: 4,
            hend: 1,
            tbl: 2,
            iholddelay: 8,
            tpowerdown: 20,
            freewheel: FreewheelMode::Normal,
            pwm_freq: 1,
            pwm_autoscale: true,
            pwm_autograd: true,
            pwm_reg: 8,
            pwm_ofs: 36,
            pwm_grad: 14,
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

pub enum Tmc2209Error<E> {
    Transport(E),
    NoReply,
    CrcMismatch,
    InvalidMicrosteps,
    CurrentOutOfRange,
    InvalidParameter,
}

#[derive(Clone, Copy, Default)]
pub struct DriverStatus {
    pub standstill: bool,
    pub stealth_active: bool,
    pub cs_actual: u8,
    pub over_temp_prewarning: bool,
    pub over_temp_shutdown: bool,
    pub temp_flag_120: bool,
    pub temp_flag_143: bool,
    pub temp_flag_150: bool,
    pub temp_flag_157: bool,
    pub short_to_gnd_a: bool,
    pub short_to_gnd_b: bool,
    pub short_low_a: bool,
    pub short_low_b: bool,
    pub open_load_a: bool,
    pub open_load_b: bool,
}

impl DriverStatus {
    pub fn from_raw(reg: super::registers::DRV_STATUS) -> Self {
        Self {
            stealth_active: reg.stealth(),
            cs_actual: reg.cs_actual().into(),
            over_temp_prewarning: reg.otpw(),
            standstill: reg.stst(),
            temp_flag_120: reg.t120(),
            temp_flag_143: reg.t143(),
            temp_flag_150: reg.t150(),
            temp_flag_157: reg.t157(),
            short_low_a: reg.s2vsa(),
            short_low_b: reg.s2vsb(),
            short_to_gnd_a: reg.s2ga(),
            short_to_gnd_b: reg.s2gb(),
            open_load_a: reg.ola(),
            open_load_b: reg.olb(),
            over_temp_shutdown: reg.ot(),
        }
    }
}

pub struct GlobalStatus {
    pub reset: bool,
    pub driver_error: bool,
    pub undervoltage: bool,
}
