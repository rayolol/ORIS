#![allow(non_camel_case_types)]

use bitbybit::bitfield;
use arbitrary_int::{u2, u3, u4, u5, u7, u9, u10, u20, u23, u40};

// ─── SPI Frame Types ──────────────────────────────────────────────────────────

pub trait Register {
    const ADDRESS: u8;
}

pub trait Frame {
    fn make_frame(payload: u32, register_addr: u8) -> [u8; 5];
}

#[bitfield(u40)]
pub struct ReadFrame {
    #[bits(0..=7, r)]
    spi_status: u8,
    #[bits(8..=39, r)]
    data: u32,
}

impl ReadFrame {
    pub fn read_frame(data: u40) -> u32 {
        Self::new_with_raw_value(data).data()
    }
}

#[bitfield(u40)]
pub struct WriteFrame {
    #[bits(0..=6, rw)]
    addr: u7,
    #[bit(7, rw)]
    write_flag: bool,
    #[bits(8..=39, rw)]
    data: u32,
}

impl Frame for WriteFrame {
    fn make_frame(payload: u32, register_addr: u8) -> [u8; 5] {
        let frame = Self::new_with_raw_value(u40::new(0))
            .with_addr(u7::new(register_addr))
            .with_write_flag(true)
            .with_data(payload);

        let bytes = frame.raw_value.to_be_bytes();
        [bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]
    }
}

// ─── 0x00 – GCONF: General Configuration (R/W) ───────────────────────────────

#[bitfield(u32)]
pub(super) struct GCONF {
    /// Recalibrate x0 position and offset calibration on standstill
    #[bit(0, rw)]
    recalibrate: bool,
    /// Short standstill timeout (0 = 2^20 clocks, 1 = 2^18 clocks)
    #[bit(1, rw)]
    faststandstill: bool,
    /// Enable stealthChop voltage-PWM mode
    #[bit(2, rw)]
    en_pwm_mode: bool,
    /// Enable step-input filter for stealthChop
    #[bit(3, rw)]
    multistep_filt: bool,
    /// Invert motor direction
    #[bit(4, rw)]
    shaft: bool,
    /// DIAG0 active on driver errors
    #[bit(5, rw)]
    diag0_err: bool,
    /// DIAG0 active on overtemperature prewarning
    #[bit(6, rw)]
    diag0_otpw: bool,
    /// DIAG0 active on motor stall
    #[bit(7, rw)]
    diag0_stall: bool,
    /// DIAG1 active on motor stall
    #[bit(8, rw)]
    diag1_stall: bool,
    /// DIAG1 active on index position
    #[bit(9, rw)]
    diag1_index: bool,
    /// DIAG1 active when chopper is on
    #[bit(10, rw)]
    diag1_onstate: bool,
    /// DIAG1 toggles when steps are skipped in dcStep mode
    #[bit(11, rw)]
    diag1_steps_skipped: bool,
    /// 0 = DIAG0 open-collector, 1 = push-pull
    #[bit(12, rw)]
    diag0_int_push_pull: bool,
    /// 0 = DIAG1 open-collector, 1 = push-pull
    #[bit(13, rw)]
    diag1_pushpull: bool,
    /// Step frequency comparison hysteresis: 0 = 1/16, 1 = 1/32
    #[bit(14, rw)]
    small_hysteresis: bool,
    /// Emergency stop: DCIN halts the sequencer when high
    #[bit(15, rw)]
    stop_enable: bool,
    /// Direct mode: coil currents set via XDIRECT register
    #[bit(16, rw)]
    direct_mode: bool,
    /// Test mode – not for normal use
    #[bit(17, rw)]
    test_mode: bool,
}

// ─── 0x01 – GSTAT: Global Status Flags (R+W1C) ───────────────────────────────

#[bitfield(u32)]
pub(super) struct GSTAT {
    /// IC has been reset since last read; write 1 to clear
    #[bit(0, rw)]
    reset: bool,
    /// Driver error since last read; write 1 to clear
    #[bit(1, rw)]
    drv_err: bool,
    /// Charge-pump undervoltage; write 1 to clear
    #[bit(2, rw)]
    uv_cp: bool,
}

// ─── 0x02 – IFCNT: SPI Interface Transmission Counter (R) ────────────────────

#[bitfield(u32)]
pub(super) struct IFCNT {
    /// Increments with each valid SPI write; wraps at 255
    #[bits(0..=7, r)]
    ifcnt: u8,
}

// ─── 0x04 – IOIN: Input Pin Readback (R) ─────────────────────────────────────

#[bitfield(u32)]
pub(super) struct IOIN {
    #[bit(0, r)]
    refl_step: bool,
    #[bit(1, r)]
    refr_dir: bool,
    #[bit(2, r)]
    encb_dcen_cfg4: bool,
    #[bit(3, r)]
    enca_dcin_cfg5: bool,
    /// Hardware enable pin – low = driver enabled
    #[bit(4, r)]
    drv_enn: bool,
    #[bit(5, r)]
    encn_dco_cfg6: bool,
    /// SD_MODE pin level
    #[bit(6, r)]
    sd_mode: bool,
    #[bit(7, r)]
    swcomp_in: bool,
    /// Chip version; reads 0x30 for TMC2160
    #[bits(24..=31, r)]
    version: u8,
}

// ─── 0x06 – OTP_PROG: OTP Programming (W) ────────────────────────────────────

#[bitfield(u32)]
pub(super) struct OTP_PROG {
    /// Select OTP bit to program (0–7)
    #[bits(0..=2, w)]
    otpbit: u3,
    /// Select OTP byte to program (0–2)
    #[bits(4..=5, w)]
    otpbyte: u2,
    /// Magic value – must be 0xBD to unlock OTP write
    #[bits(8..=15, w)]
    otpmagic: u8,
}

// ─── 0x07 – OTP_READ: OTP Memory Readback (R) ────────────────────────────────

#[bitfield(u32)]
pub(super) struct OTP_READ {
    /// OTP byte 0 (bits 0-4 = FCLKTRIM, bits 5-6 = OTTRIM)
    #[bits(0..=7, r)]
    otp_byte0: u8,
    /// OTP byte 1
    #[bits(8..=15, r)]
    otp_byte1: u8,
    /// OTP byte 2
    #[bits(16..=23, r)]
    otp_byte2: u8,
}

// ─── 0x08 – FACTORY_CONF: Factory Configuration (R/W) ────────────────────────

#[bitfield(u32)]
pub(super) struct FACTORY_CONF {
    /// Internal oscillator frequency trim (0–31)
    #[bits(0..=4, rw)]
    fclktrim: u5,
    /// Overtemperature threshold trim (0–3)
    #[bits(8..=9, rw)]
    ottrim: u2,
}

// ─── 0x09 – SHORT_CONF: Short-Circuit Detection Configuration (R/W) ──────────

#[bitfield(u32)]
pub(super) struct SHORT_CONF {
    /// Short-to-VS detector sensitivity (1–15; higher = less sensitive). Default: 6
    #[bits(0..=3, rw)]
    s2vs_level: u4,
    /// Short-to-GND detector sensitivity (2–15; higher = less sensitive). Default: 6
    #[bits(8..=11, rw)]
    s2gnd_level: u4,
    /// Spike filter bandwidth: 0=100 ns, 1=1 µs, 2=2 µs, 3=3 µs
    #[bits(16..=17, rw)]
    shortfilter: u2,
    /// Short detection delay: 0 = 750 ns, 1 = 1500 ns
    #[bit(18, rw)]
    shortdelay: bool,
}

// ─── 0x0A – DRV_CONF: Driver Configuration (R/W) ─────────────────────────────

#[bitfield(u32)]
pub(super) struct DRVCONF {
    /// Break-before-make delay (0–24 ns range)
    #[bits(0..=4, rw)]
    bbmtime: u5,
    /// Digital break-before-make in clock cycles (0–15)
    #[bits(8..=11, rw)]
    bbmclks: u4,
    /// Overtemperature level: 0=150°C, 1=143°C, 2=136°C, 3=120°C
    #[bits(16..=17, rw)]
    otselect: u2,
    /// Gate driver strength: 0=weak, 1=weak+TC, 2=medium, 3=strong
    #[bits(18..=19, rw)]
    drvstrength: u2,
    /// Current sense filter: 0=100 ns, 1=200 ns, 2=300 ns, 3=400 ns
    #[bits(20..=21, rw)]
    filt_isense: u2,
}

// ─── 0x0B – GLOBALSCALER: Motor Current Global Scaler (W) ────────────────────

#[bitfield(u32)]
pub(super) struct GLOBALSCALER {
    /// Global current scale: 0 = full scale (256/256), 32–255 = (value/256) × I_max
    #[bits(0..=7, w)]
    globalscaler: u8,
}

// ─── 0x0C – OFFSET_READ: Current Offset Calibration Readback (R) ─────────────

#[bitfield(u32)]
pub(super) struct OFFSETREAD {
    /// Signed offset for motor phase B
    #[bits(0..=7, r)]
    phase_b: u8,
    /// Signed offset for motor phase A
    #[bits(8..=15, r)]
    phase_a: u8,
}

// ─── 0x10 – IHOLDIRUN: Driver Current Control (W) ────────────────────────────

#[bitfield(u32)]
pub(super) struct IHOLDIRUN {
    /// Standstill current (0–31)
    #[bits(0..=4, rw)]
    ihold: u5,
    /// Motor run current (0–31)
    #[bits(8..=12, rw)]
    irun: u5,
    /// Delay from IRUN to IHOLD after standstill (0–15)
    #[bits(16..=19, rw)]
    iholddelay: u4,
}

// ─── 0x11 – TPOWERDOWN: Power-Down Delay (W) ─────────────────────────────────

#[bitfield(u32)]
pub(super) struct TPOWERDOWN {
    /// Delay before motor current powers down (0–255; step = 2^18 clocks)
    #[bits(0..=7, w)]
    tpowerdown: u8,
}

// ─── 0x12 – TSTEP: Measured Step Period (R) ──────────────────────────────────

#[bitfield(u32)]
pub(super) struct TSTEP {
    /// Measured period between two 1/256 microsteps (in clock cycles)
    #[bits(0..=19, r)]
    tstep: u20,
}

// ─── 0x13 – TPWMTHRS: stealthChop Upper Velocity Threshold (W) ───────────────

#[bitfield(u32)]
pub(super) struct TPWMTHRS {
    /// TSTEP value above which stealthChop is disabled
    #[bits(0..=19, w)]
    tpwmthrs: u20,
}

// ─── 0x14 – TCOOLTHRS: coolStep / stallGuard Velocity Threshold (W) ──────────

#[bitfield(u32)]
pub(super) struct TCOOLTHRS {
    /// TSTEP below which coolStep and stallGuard are enabled
    #[bits(0..=19, w)]
    tcoolthrs: u20,
}

// ─── 0x15 – THIGH: High-Velocity Mode Threshold (W) ──────────────────────────

#[bitfield(u32)]
pub(super) struct THIGH {
    /// TSTEP below which fullstep and high-velocity chopper become active
    #[bits(0..=19, w)]
    thigh: u20,
}

// ─── 0x2D – XDIRECT: Direct Coil-Current Control (R/W) ───────────────────────

#[bitfield(u32)]
pub(super) struct XDIRECT {
    /// Coil A current – signed 9-bit (two's complement)
    #[bits(0..=8, rw)]
    cur_a: u9,
    /// Coil B current – signed 9-bit (two's complement)
    #[bits(16..=24, rw)]
    cur_b: u9,
}

// ─── 0x33 – VDCMIN: DC Step Minimum Velocity (W) ─────────────────────────────

#[bitfield(u32)]
pub(super) struct VDCMIN {
    /// Minimum velocity for DC step mode (0 = disabled)
    #[bits(0..=22, w)]
    vdcmin: u23,
}

// ─── 0x60–0x67 – MSLUT[0..7]: Microstep Look-Up Table (W) ───────────────────

pub(super) struct MSLUT0(pub u32);
pub(super) struct MSLUT1(pub u32);
pub(super) struct MSLUT2(pub u32);
pub(super) struct MSLUT3(pub u32);
pub(super) struct MSLUT4(pub u32);
pub(super) struct MSLUT5(pub u32);
pub(super) struct MSLUT6(pub u32);
pub(super) struct MSLUT7(pub u32);

// ─── 0x68 – MSLUTSEL: Microstep Table Segment Widths (W) ─────────────────────

#[bitfield(u32)]
pub(super) struct MSLUTSEL {
    /// Width of LUT segment 0 (0=±1, 1=±2, 2=±4, 3=±8)
    #[bits(0..=1, w)]
    w0: u2,
    #[bits(2..=3, w)]
    w1: u2,
    #[bits(4..=5, w)]
    w2: u2,
    #[bits(6..=7, w)]
    w3: u2,
    /// Boundary between segment 0 and 1 (microstep counter value 0–255)
    #[bits(8..=15, w)]
    x1: u8,
    #[bits(16..=23, w)]
    x2: u8,
    #[bits(24..=31, w)]
    x3: u8,
}

// ─── 0x69 – MSLUTSTART: Microstep Table Start Values (W) ─────────────────────

#[bitfield(u32)]
pub(super) struct MSLUTSTART {
    /// Initial current at microstep counter = 0
    #[bits(0..=7, w)]
    start_sin: u8,
    /// Initial current at microstep counter = 256 (90°)
    #[bits(16..=23, w)]
    start_sin90: u8,
}

// ─── 0x6A – MSCNT: Microstep Counter (R) ────────────────────────────────────

#[bitfield(u32)]
pub(super) struct MSCNT {
    /// Current microstep table position (0–1023)
    #[bits(0..=9, r)]
    mscnt: u10,
}

// ─── 0x6B – MSCURACT: Microstep Current Values (R) ──────────────────────────

#[bitfield(u32)]
pub(super) struct MSCURACT {
    /// Actual current for motor phase A – signed 9-bit
    #[bits(0..=8, r)]
    cur_a: u9,
    /// Actual current for motor phase B – signed 9-bit
    #[bits(16..=24, r)]
    cur_b: u9,
}

// ─── 0x6C – CHOPCONF: Chopper and Driver Configuration (R/W) ─────────────────

#[bitfield(u32)]
pub(super) struct CHOPCONF {
    /// Off time (0 = driver disabled; 1–15)
    #[bits(0..=3, rw)]
    toff: u4,
    /// Hysteresis start (CHM=0) or fast-decay time TFD[2:0] (CHM=1)
    #[bits(4..=6, rw)]
    hstrt: u3,
    /// Hysteresis end (CHM=0) or sine-wave offset (CHM=1)
    #[bits(7..=10, rw)]
    hend: u4,
    /// Fast-decay time MSB TFD[3] (CHM=1 only)
    #[bit(11, rw)]
    fd3: bool,
    /// Disable current comparator for fast-decay termination (CHM=1 only)
    #[bit(12, rw)]
    disfdcc: bool,
    /// Enable random TOFF modulation to spread EMI
    #[bit(13, rw)]
    rndtf: bool,
    /// Chopper mode: 0 = spreadCycle, 1 = classic constant off-time
    #[bit(14, rw)]
    chm: bool,
    /// Comparator blank time: 0=16, 1=24, 2=36, 3=54 clocks
    #[bits(15..=16, rw)]
    tbl: u2,
    /// Enable fullstep switching at high velocities (VHIGHFS)
    #[bit(17, rw)]
    vhighfs: bool,
    /// Enable high-velocity chopper mode (VHIGHCHM)
    #[bit(18, rw)]
    vhighchm: bool,
    /// Passive fast-decay time (0 = off; 1–15)
    #[bits(19..=22, rw)]
    tpfd: u4,
    /// Microstep resolution: 0=256, 1=128, 2=64, 3=32, 4=16, 5=8, 6=4, 7=2, 8=fullstep
    #[bits(24..=27, rw)]
    mres: u4,
    /// Enable step interpolation to 256 microsteps
    #[bit(28, rw)]
    intpol: bool,
    /// Enable double-edge step pulses
    #[bit(29, rw)]
    dedge: bool,
    /// Disable short-to-GND protection
    #[bit(30, rw)]
    diss2g: bool,
    /// Disable short-to-VS protection
    #[bit(31, rw)]
    diss2vs: bool,
}

// ─── 0x6D – COOLCONF: coolStep and stallGuard2 Configuration (W) ─────────────

#[bitfield(u32)]
pub(super) struct COOLCONF {
    /// Minimum stallGuard2 value to enable smart current reduction (0 = disabled)
    #[bits(0..=3, rw)]
    semin: u4,
    /// Current increment step size: 0=1, 1=2, 2=4, 3=8
    #[bits(5..=6, rw)]
    seup: u2,
    /// stallGuard2 hysteresis upper threshold
    #[bits(8..=11, rw)]
    semax: u4,
    /// Current decrement speed: 0=32, 1=8, 2=2, 3=1 steps per SG measurement
    #[bits(13..=14, rw)]
    sedn: u2,
    /// Minimum current: 0 = IHOLD/2, 1 = IHOLD
    #[bit(15, rw)]
    seimin: bool,
    /// stallGuard2 threshold – signed 7-bit (two's complement, −64 to +63)
    #[bits(16..=22, rw)]
    sgt: u7,
    /// Enable stallGuard2 filter (0 = standard, 1 = filtered over 4 cycles)
    #[bit(24, rw)]
    sfilt: bool,
}

// ─── 0x6E – DCCTRL: DC Step Control (W) ──────────────────────────────────────

#[bitfield(u32)]
pub(super) struct DCCTRL {
    /// Upper PWM on-time limit for DC step mode (clock cycles)
    #[bits(0..=9, rw)]
    dc_time: u10,
    /// stallGuard2 threshold for DC step validation (0 = disabled)
    #[bits(16..=23, rw)]
    dc_sg: u8,
}

// ─── 0x6F – DRV_STATUS: Driver Status Flags (R) ──────────────────────────────

#[bitfield(u32)]
pub(super) struct DRV_STATUS {
    /// stallGuard2 result (0 = high load / stall; 1023 = no load)
    #[bits(0..=9, r)]
    sg_result: u10,
    /// Short to supply detected on phase A
    #[bit(10, r)]
    s2vsa: bool,
    /// Short to supply detected on phase B
    #[bit(11, r)]
    s2vsb: bool,
    /// stealthChop mode currently active
    #[bit(12, r)]
    stealth: bool,
    /// Fullstep active
    #[bit(13, r)]
    fsactive: bool,
    /// Actual motor current (CS value, 0–31)
    #[bits(16..=20, r)]
    cs_actual: u5,
    /// stallGuard2 stall detected
    #[bit(24, r)]
    sg_status: bool,
    /// Overtemperature shutdown active
    #[bit(25, r)]
    ot: bool,
    /// Overtemperature prewarning active
    #[bit(26, r)]
    otpw: bool,
    /// Short to GND detected on phase A
    #[bit(27, r)]
    s2ga: bool,
    /// Short to GND detected on phase B
    #[bit(28, r)]
    s2gb: bool,
    /// Open-load indicator phase A
    #[bit(29, r)]
    ola: bool,
    /// Open-load indicator phase B
    #[bit(30, r)]
    olb: bool,
    /// Standstill indicator
    #[bit(31, r)]
    stst: bool,
}

// ─── 0x70 – PWMCONF: Voltage PWM Mode Configuration (R/W) ────────────────────

#[bitfield(u32)]
pub(super) struct PWMCONF {
    /// PWM amplitude offset (0–255)
    #[bits(0..=7, rw)]
    pwm_ofs: u8,
    /// PWM amplitude gradient (0–255)
    #[bits(8..=15, rw)]
    pwm_grad: u8,
    /// PWM frequency: 0=2/1024, 1=2/683, 2=2/512, 3=2/410 (× f_clk)
    #[bits(16..=17, rw)]
    pwm_freq: u2,
    /// Enable automatic current amplitude scaling
    #[bit(18, rw)]
    pwm_autoscale: bool,
    /// Enable automatic gradient adaptation
    #[bit(19, rw)]
    pwm_autograd: bool,
    /// Standstill freewheeling: 0=normal, 1=freewheel, 2=short LS, 3=short HS
    #[bits(20..=21, rw)]
    freewheel: u2,
    /// Regulation loop gradient (1–15)
    #[bits(24..=27, rw)]
    pwm_reg: u4,
    /// Maximum PWM amplitude during autoscale (0–15)
    #[bits(28..=31, rw)]
    pwm_lim: u4,
}

// ─── 0x71 – PWM_SCALE: PWM Scale Readback (R) ────────────────────────────────

#[bitfield(u32)]
pub(super) struct PWM_SCALE {
    /// Accumulated PWM amplitude scaling value (0–255)
    #[bits(0..=7, r)]
    pwm_scale_sum: u8,
    /// Automatically determined PWM amplitude – signed 9-bit
    #[bits(16..=24, r)]
    pwm_scale_auto: u9,
}

// ─── 0x72 – PWM_AUTO: Automatic PWM Values (R) ───────────────────────────────

#[bitfield(u32)]
pub(super) struct PWM_AUTO {
    /// Auto-determined offset value
    #[bits(0..=7, r)]
    pwm_ofs_auto: u8,
    /// Auto-determined gradient value
    #[bits(16..=23, r)]
    pwm_grad_auto: u8,
}

// ─── 0x73 – LOST_STEPS: Lost Step Counter (R) ────────────────────────────────

#[bitfield(u32)]
pub(super) struct LOST_STEPS {
    /// Number of input steps skipped in dcStep mode (cleared on read)
    #[bits(0..=19, r)]
    lost_steps: u20,
}

// ─── Register Address Map ─────────────────────────────────────────────────────

macro_rules! register {
    ($($name:ident => $addr:expr),* $(,)?) => {
        $(impl Register for $name { const ADDRESS: u8 = $addr; })*
    };
}

register! {
    GCONF        => 0x00,
    GSTAT        => 0x01,
    IFCNT        => 0x02,
    IOIN         => 0x04,
    OTP_PROG     => 0x06,
    OTP_READ     => 0x07,
    FACTORY_CONF => 0x08,
    SHORT_CONF   => 0x09,
    DRVCONF      => 0x0A,
    GLOBALSCALER => 0x0B,
    OFFSETREAD   => 0x0C,
    IHOLDIRUN    => 0x10,
    TPOWERDOWN   => 0x11,
    TSTEP        => 0x12,
    TPWMTHRS     => 0x13,
    TCOOLTHRS    => 0x14,
    THIGH        => 0x15,
    XDIRECT      => 0x2D,
    VDCMIN       => 0x33,
    MSLUT0       => 0x60,
    MSLUT1       => 0x61,
    MSLUT2       => 0x62,
    MSLUT3       => 0x63,
    MSLUT4       => 0x64,
    MSLUT5       => 0x65,
    MSLUT6       => 0x66,
    MSLUT7       => 0x67,
    MSLUTSEL     => 0x68,
    MSLUTSTART   => 0x69,
    MSCNT        => 0x6A,
    MSCURACT     => 0x6B,
    CHOPCONF     => 0x6C,
    COOLCONF     => 0x6D,
    DCCTRL       => 0x6E,
    DRV_STATUS   => 0x6F,
    PWMCONF      => 0x70,
    PWM_SCALE    => 0x71,
    PWM_AUTO     => 0x72,
    LOST_STEPS   => 0x73,
}
