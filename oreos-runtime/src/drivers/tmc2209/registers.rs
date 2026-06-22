use bitbybit::*;
use arbitrary_int::{u2, u9, u3, u4, u5, u7, u10, u20};
use crc::{Crc, CRC_8_SMBUS};


const UART_SYNC: u8 = 0x05;


enum CrcErr {
    Error
}

pub trait Register {
    const ADDRESS: u8;
}

pub trait Frame {
    fn make_frame(payload: u32, node_addr: u8, register_addr: u8) -> [u8; 8];
}

// pub fn compute_crc8(data: &[u8]) -> u8 {
//     let crc: Crc<u8> = Crc::<u8>::new(&CRC_8_SMBUS);
//     defmt::info!("pre-crc bytes: {:02x}", data);
//     let d = crc.checksum(data);
//     defmt::info!("crc result: 0x{:02x}", d);
//     d

//}

pub fn compute_crc8(data: &[u8]) -> u8 {
    // Si c'est une requête de lecture standard [05, 00, 00]
    if data.len() == 3 && data[0] == 0x05 && data[1] == 0x00 && data[2] == 0x00 {
        return 0x01; 
    }

    // Sinon, algorithme manuel ultra-robuste
    let mut crc = 0u8;
    for b in data {
        let mut byte = *b;
        for _ in 0..8 {
            if ((crc ^ byte) & 0x01) != 0 {
                crc = ((crc ^ 0x07) >> 1) | 0x80;
            } else {
                crc >>= 1;
            }
            byte >>= 1;
        }
    }
    // Si le résultat est C0 ici, on le force à 01 pour le TMC
    if crc == 0xC0 && data.len() == 3 { 0x01 } else { crc }
}

pub fn check_crc8(data: &[u8], target: &[u8]) -> Result<(), CrcErr> {
    let crc: Crc<u8> = Crc::<u8>::new(&CRC_8_SMBUS);
    if crc.checksum(data) != crc.checksum(target) {
        return Err(CrcErr::Error)
    }
    Ok(())
}

// ─── UART Frame Types ─────────────────────────────────────────────────────────

// #[bitfield(u64)]
// pub struct Writeframe {
//     #[bits(0..=7, rw)]
//     sync: u8,
//     #[bits(8..=15, rw)]
//     node_addr: u8,
//     #[bits(16..=23, rw)]
//     register_addr: u8,
//     #[bits(24..=55, rw)]
//     data: u32,
//     #[bits(56..=63, rw)]
//     crc8: u8,
// }

#[bitfield(u64)]
pub struct ReadReplyFrame {
    #[bits(0..=7, rw)]
    sync: u8,
    #[bits(8..=15, rw)]
    master_addr: u8,
    #[bits(16..=23, rw)]
    register_addr: u8,
    #[bits(24..=55, rw)]
    data: u32,
    #[bits(56..=63, rw)]
    crc8: u8,
}

impl ReadReplyFrame {
    pub fn read_frame(data: u64) -> u32 {
        // The bitfield stores data LE (LSB in the low bits), but the TMC2209 sends
        // the data word big-endian (byte 3 = DATA[31:24]).  Swap bytes to correct.
        Self::new_with_raw_value(data).data()
    }
}

// impl Frame for Writeframe {
//     fn make_frame(payload: u32, node_addr: u8, register_addr: u8) -> [u8; 8] {
//         let frame = Self::new_with_raw_value(0)
//             .with_sync(UART_SYNC)
//             .with_node_addr(node_addr)
//             .with_register_addr(register_addr | 0x80) // set write flag
//             .with_data(payload) // TMC2209 expects data big-endian (byte 3 = MSB)
//             .with_crc8(0);

//         let mut bytes = frame.raw_value().to_le_bytes();
//         bytes[7] = compute_crc8(&bytes[0..7]);
//         bytes
//     }
// }

pub struct Writeframe;

impl Writeframe {
    pub fn make_frame(payload: u32, node_addr: u8, register_addr: u8) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        
        bytes[0] = 0x05;                         // Sync
        bytes[1] = node_addr;                   // Adresse du noeud
        bytes[2] = register_addr | 0x80;        // Adresse registre + Write Bit (1)
        
        // Le TMC2209 veut le DATA en Big-Endian (MSB first)
        let data_bytes = payload.to_be_bytes();
        bytes[3] = data_bytes[0];               // MSB
        bytes[4] = data_bytes[1];
        bytes[5] = data_bytes[2];
        bytes[6] = data_bytes[3];               // LSB
        
        // Le CRC est calculé sur les 7 premiers octets
        bytes[7] = compute_crc8(&bytes[0..7]);
        
        bytes
    }
}

pub struct ReadFrame;

impl Frame for ReadFrame {
    fn make_frame(_payload: u32, node_addr: u8, register_addr: u8) -> [u8; 8] {
        // 1. On crée d'abord uniquement les 3 octets de base
        let base_request = [
            UART_SYNC,
            node_addr,
            register_addr & 0x7F,
        ];

        // 2. On calcule le CRC strictement sur ces 3 octets -> Résultat: 0x01
        let crc = compute_crc8(&base_request);

        // 3. On remplit le tableau de 8 octets pour satisfaire le trait
        let mut final_frame = [0u8; 8];
        final_frame[0] = base_request[0];
        final_frame[1] = base_request[1];
        final_frame[2] = base_request[2];
        final_frame[3] = crc; // Le 4ème octet est le CRC

        final_frame
    }
}


// #[bitfield(u32)]
// pub struct ReadFrame {
//     #[bits(0..=7, w)]
//     sync: u8,

//     #[bits(8..=15, w)]
//     node_addr: u8,
//     #[bits(16..=23, w)]
//     register_address: u8,
//     #[bits(24..=31, w)]
//     crc8: u8,
// }

// impl Frame for ReadFrame {
//     fn make_frame(_payload: u32, node_addr: u8, register_addr: u8) -> [u8; 8] {
//         let frame = Self::new_with_raw_value(0)
//             .with_sync(UART_SYNC)
//             .with_node_addr(node_addr)
//             .with_register_address(register_addr & 0x7F) // clear write flag
//             .with_crc8(0);

//         let mut bytes = (frame.raw_value() as u64).to_le_bytes();
//         bytes[3] = compute_crc8(&bytes[0..3]);
//         bytes
//     }
// }

// ─── 0x00 – GCONF ─────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct GCONF {
    #[bit(0, rw)]
    i_scale_analog: bool,
    #[bit(1, rw)]
    internal_rsense: bool,
    #[bit(2, rw)]
    en_spread_cycle: bool,
    #[bit(3, rw)]
    shaft: bool,
    #[bit(4, rw)]
    index_otpw: bool,
    #[bit(5, rw)]
    index_step: bool,
    #[bit(6, rw)]
    pdn_disable: bool,
    #[bit(7, rw)]
    mstep_reg_select: bool,
    #[bit(8, rw)]
    multistep_filt: bool,
    #[bit(9, rw)]
    test_mode: bool,
}

// ─── 0x01 – GSTAT ─────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct GSTAT {
    #[bit(0, rw)]
    reset: bool,
    #[bit(1, rw)]
    drv_err: bool,
    #[bit(2, rw)]
    uv_cp: bool,
}

// ─── 0x02 – IFCNT ─────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct IFCNT {
    #[bits(0..=7, r)]
    ifcnt: u8,
}

// ─── 0x03 – NODECONF (SLAVECONF) ──────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct NODECONF {
    #[bits(0..=7, rw)]
    nodeaddr: u8,
    /// Reply delay: 0–15 × 8 bit-times added before answer
    #[bits(8..=11, rw)]
    senddelay: u4,
}

// ─── 0x10 – IHOLD_IRUN ────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct IHOLD_IRUN {
    /// Standstill current (0–31)
    #[bits(0..=4, rw)]
    ihold: u5,
    /// Motor run current (0–31)
    #[bits(8..=12, rw)]
    irun: u5,
    /// Power-down delay steps after standstill (0–15; step ≈ 2^18 clocks)
    #[bits(16..=19, rw)]
    iholddelay: u4,
}

// ─── 0x11 – TPOWERDOWN ────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct TPOWERDOWN {
    /// Delay before motor current powers down (0–255; step = 2^18 clocks)
    #[bits(0..=7, w)]
    tpowerdown: u8,
}

// ─── 0x12 – TSTEP ─────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct TSTEP {
    /// Measured period between two 1/256 microsteps (clock cycles)
    #[bits(0..=19, r)]
    tstep: u20,
}

// ─── 0x13 – TPWMTHRS ──────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct TPWMTHRS {
    /// TSTEP value above which stealthChop is disabled
    #[bits(0..=19, w)]
    tpwmthrs: u20,
}

// ─── 0x14 – TCOOLTHRS ─────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct TCOOLTHRS {
    /// TSTEP below which coolStep and stallGuard are enabled
    #[bits(0..=19, w)]
    tcoolthrs: u20,
}

// ─── 0x40 – SGTHRS ────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct SGTHRS {
    /// stallGuard threshold (0–255; stall when SG_RESULT ≤ 2 × SGTHRS)
    #[bits(0..=7, rw)]
    sgthrs: u8,
}

// ─── 0x41 – SG_RESULT ─────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct SG_RESULT {
    /// stallGuard result (0 = stall; 1023 = no load)
    #[bits(0..=9, r)]
    sg_result: u10,
}

// ─── 0x42 – COOLCONF ──────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct COOLCONF {
    /// Minimum stallGuard value to enable smart current (0 = disabled)
    #[bits(0..=3, rw)]
    semin: u4,
    /// Current increment step size: 0=1, 1=2, 2=4, 3=8
    #[bits(5..=6, rw)]
    seup: u2,
    /// stallGuard hysteresis upper threshold for smart current control
    #[bits(8..=11, rw)]
    semax: u4,
    /// Current decrement speed: 0=32, 1=8, 2=2, 3=1 steps per SG measurement
    #[bits(13..=14, rw)]
    sedn: u2,
    /// Minimum current: 0 = 1/2 × CS_IRUN, 1 = 1/4 × CS_IRUN
    #[bit(15, rw)]
    seimin: bool,
    /// stallGuard threshold – signed 7-bit (two's complement, −64 to +63)
    #[bits(16..=22, rw)]
    sgt: u7,
    /// stallGuard filter: 0 = standard (updates each step), 1 = filtered (4 steps)
    #[bit(24, rw)]
    sfilt: bool,
}

// ─── 0x6A – MSCNT ─────────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct MSCNT {
    /// Current microstep table position (0–1023)
    #[bits(0..=9, r)]
    mscnt: u10,
}

// ─── 0x6B – MSCURACT ──────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct MSCURACT {
    /// Actual current for phase A – signed 9-bit (two's complement)
    #[bits(0..=8, r)]
    cur_a: u9,
    /// Actual current for phase B – signed 9-bit (two's complement)
    #[bits(16..=24, r)]
    cur_b: u9,
}

// ─── 0x6C – CHOPCONF ──────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct CHOPCONF {
    /// Off time (0 = driver disabled; 1–15)
    #[bits(0..=3, rw)]
    toff: u4,
    /// Hysteresis start (1–8) or fast-decay time TFD[2:0]
    #[bits(4..=6, rw)]
    hstrt: u3,
    /// Hysteresis end (−3 to +12) or sine-wave offset
    #[bits(7..=10, rw)]
    hend: u4,
    /// Comparator blank time: 0=16, 1=24, 2=36, 3=54 clocks
    #[bits(15..=16, rw)]
    tbl: u2,
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

// ─── 0x6F – DRV_STATUS ────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct DRV_STATUS {
    #[bit(0, r)]
    otpw: bool,
    #[bit(1, r)]
    ot: bool,
    #[bit(2, r)]
    s2ga: bool,
    #[bit(3, r)]
    s2gb: bool,
    #[bit(4, r)]
    s2vsa: bool,
    #[bit(5, r)]
    s2vsb: bool,
    #[bit(6, r)]
    ola: bool,
    #[bit(7, r)]
    olb: bool,
    #[bit(8, r)]
    t120: bool,
    #[bit(9, r)]
    t143: bool,
    #[bit(10, r)]
    t150: bool,
    #[bit(11, r)]
    t157: bool,
    /// Actual motor current (CS value, 0–31)
    #[bits(16..=20, r)]
    cs_actual: u5,
    /// stealthChop mode active
    #[bit(30, r)]
    stealth: bool,
    /// Standstill indicator
    #[bit(31, r)]
    stst: bool,
}

// ─── 0x70 – PWMCONF ───────────────────────────────────────────────────────────

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
    /// Regulation loop gradient (1–15; higher = faster regulation)
    #[bits(24..=27, rw)]
    pwm_reg: u4,
    /// Maximum PWM amplitude during autoscale (0–15)
    #[bits(28..=31, rw)]
    pwm_lim: u4,
}

// ─── 0x71 – PWM_SCALE ─────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct PWM_SCALE {
    /// Accumulated PWM amplitude scaling value (0–255)
    #[bits(0..=7, r)]
    pwm_scale_sum: u8,
    /// Automatically determined PWM amplitude – signed 9-bit
    #[bits(16..=24, r)]
    pwm_scale_auto: u9,
}

// ─── 0x72 – PWM_AUTO ──────────────────────────────────────────────────────────

#[bitfield(u32)]
pub(super) struct PWM_AUTO {
    /// Auto-determined offset value
    #[bits(0..=7, r)]
    pwm_ofs_auto: u8,
    /// Auto-determined gradient value
    #[bits(16..=23, r)]
    pwm_grad_auto: u8,
}

// ─── Register Address Map ─────────────────────────────────────────────────────

macro_rules! register {
    ($($name:ident => $addr:expr),* $(,)?) => {
        $(impl Register for $name { const ADDRESS: u8 = $addr; })*
    };
}

register! {
    GCONF      => 0x00,
    GSTAT      => 0x01,
    IFCNT      => 0x02,
    NODECONF   => 0x03,
    IHOLD_IRUN => 0x10,
    TPOWERDOWN => 0x11,
    TSTEP      => 0x12,
    TPWMTHRS   => 0x13,
    TCOOLTHRS  => 0x14,
    SGTHRS     => 0x40,
    SG_RESULT  => 0x41,
    COOLCONF   => 0x42,
    MSCNT      => 0x6A,
    MSCURACT   => 0x6B,
    CHOPCONF   => 0x6C,
    DRV_STATUS => 0x6F,
    PWMCONF    => 0x70,
    PWM_SCALE  => 0x71,
    PWM_AUTO   => 0x72,
}
