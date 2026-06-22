use arbitrary_int::{u2, u3, u4, u5};
use super::registers::{Frame, Register};
use super::structs::*;
use super::registers as reg;
use embassy_time::{Duration, Instant, Timer};

use crate::transport::traits::{TransportHandler, RequestType};


//TODO turn this into a working crate with good docs

pub struct Tmc2209<T: TransportHandler<8>> {
    transport: T,
    tmc_id: u8,
    config: Option<Tmc2209Config>,
}

impl<T: TransportHandler<8>> Tmc2209<T> {
    pub fn new(transport: T, tmc_id: u8) -> Self {
        Self {
            tmc_id,
            transport,
            config: None,
        }
    }

    pub async fn init(&mut self, config: Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        self.apply_uart_mode(&config).await?;
        Timer::after_millis(100).await;

        defmt::info!("tmc init: gconf ok");
        self.apply_chopper(&config).await?;
        Timer::after_millis(100).await;

        defmt::info!("tmc init: chopconf ok");
        self.apply_current(&config).await?;
        Timer::after_millis(100).await;

        defmt::info!("tmc init: ihold_irun ok");
        self.apply_standstill(&config).await?;
        Timer::after_millis(100).await;

        defmt::info!("tmc init: tpowerdown ok");
        self.apply_stealthchop(&config).await?;
        Timer::after_millis(100).await;

        defmt::info!("tmc init: pwmconf ok");
        Timer::after_millis(100).await;


        if let Some(ref sg) = config.stallguard {
            self.apply_stall_detection(sg).await?;
            if let Some(ref cs) = sg.coolstep {
                self.apply_adaptive_current(cs).await?;
            }

        }
        Timer::after_millis(100).await;


        self.config = Some(config);
        Ok(())
    }

    pub async fn reconfigure(&mut self, config: Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        self.init(config).await
    }

    // ── Private register writers ──

    async fn apply_uart_mode(&mut self, config: &Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        let payload = reg::GCONF::new_with_raw_value(0)
            .with_pdn_disable(true)
            .with_mstep_reg_select(true)
            .with_en_spread_cycle(!config.stealthchop)
            .with_shaft(config.shaft_reverse)
            .with_multistep_filt(config.advanced.multistep_filt)
            .with_i_scale_analog(false)
            .with_internal_rsense(false)
            .with_index_otpw(false)
            .with_index_step(false)
            .with_test_mode(false);

        self.write_register(reg::GCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_chopper(&mut self, config: &Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        let mres = Self::mres_from_microsteps(config.microsteps)
            .ok_or(Tmc2209Error::InvalidMicrosteps)?;

        let payload = reg::CHOPCONF::new_with_raw_value(0)
            .with_mres(mres)
            .with_intpol(config.interpolation)
            .with_toff(u4::new(config.advanced.toff))
            .with_hstrt(u3::new(config.advanced.hstrt))
            .with_hend(u4::new(config.advanced.hend))
            .with_tbl(u2::new(config.advanced.tbl));

        self.write_register(reg::CHOPCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_current(&mut self, config: &Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        let irun = Self::ma_to_cs(config.run_current_ma, config.sense_resistor_mohm)
            .ok_or(Tmc2209Error::CurrentOutOfRange)?;
        let ihold = Self::ma_to_cs(config.hold_current_ma, config.sense_resistor_mohm)
            .ok_or(Tmc2209Error::CurrentOutOfRange)?;

        let payload = reg::IHOLD_IRUN::new_with_raw_value(0)
            .with_irun(u5::new(irun))
            .with_ihold(u5::new(ihold))
            .with_iholddelay(u4::new(config.advanced.iholddelay));

        self.write_register(reg::IHOLD_IRUN::ADDRESS, payload.raw_value()).await
    }

    async fn apply_standstill(&mut self, config: &Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        self.write_register(reg::TPOWERDOWN::ADDRESS, config.advanced.tpowerdown as u32).await
    }

    async fn apply_stealthchop(&mut self, config: &Tmc2209Config) -> Result<(), Tmc2209Error<T::Error>> {
        if let Some(threshold) = config.stealthchop_threshold {
            self.write_register(reg::TPWMTHRS::ADDRESS, threshold).await?;
        }

        let payload = reg::PWMCONF::new_with_raw_value(0)
            .with_pwm_autoscale(config.advanced.pwm_autoscale)
            .with_pwm_autograd(config.advanced.pwm_autograd)
            .with_pwm_freq(u2::new(config.advanced.pwm_freq))
            .with_pwm_reg(u4::new(config.advanced.pwm_reg))
            .with_pwm_ofs(config.advanced.pwm_ofs)
            .with_pwm_grad(config.advanced.pwm_grad)
            .with_freewheel(u2::new(config.advanced.freewheel as u8));

        self.write_register(reg::PWMCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_stall_detection(&mut self, sg: &StallGuardConfig) -> Result<(), Tmc2209Error<T::Error>> {
        self.write_register(reg::SGTHRS::ADDRESS, sg.threshold as u32).await?;
        self.write_register(reg::TCOOLTHRS::ADDRESS, sg.velocity_threshold).await
    }

    async fn apply_adaptive_current(&mut self, cs: &CoolStepConfig) -> Result<(), Tmc2209Error<T::Error>> {
        let payload = reg::COOLCONF::new_with_raw_value(0)
            .with_semin(u4::new(cs.semin))
            .with_semax(u4::new(cs.semax))
            .with_seup(u2::new(cs.seup))
            .with_sedn(u2::new(cs.sedn))
            .with_seimin(cs.quarter_current);

        self.write_register(reg::COOLCONF::ADDRESS, payload.raw_value()).await
    }

    // ── Generic register IO ──

    async fn write_register(&mut self, address: u8, data: u32) -> Result<(), Tmc2209Error<T::Error>> {
        let frame = reg::Writeframe::make_frame(data, self.tmc_id, address);
        let _reply = self.transport.transfer(frame, RequestType::Write).await.map_err(Tmc2209Error::Transport)?;
        Ok(())
    }
    //TODO make something with the reply 
    async fn read_register(&mut self, address: u8) -> Result<u32, Tmc2209Error<T::Error>> {
        let frame = reg::ReadFrame::make_frame(0, self.tmc_id, address);
        let result = self.transport.transfer(frame, RequestType::Read).await.map_err(Tmc2209Error::Transport)?;
        let reply = result.ok_or(Tmc2209Error::NoReply)?;
        if reg::compute_crc8(&reply[0..7]) != reply[7] {
            return Err(Tmc2209Error::CrcMismatch);
        }
        let parsed = reg::ReadReplyFrame::read_frame(u64::from_le_bytes(reply));
        Ok(parsed)
    }

    // ── Public runtime methods ──

    pub async fn set_run_current_ma(&mut self, ma: u16) -> Result<(), Tmc2209Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2209Error::InvalidParameter)?;
        config.run_current_ma = ma;
        let config = config.clone();
        self.apply_uart_mode(&config).await?;
        self.apply_current(&config).await
    }

    pub async fn set_microsteps(&mut self, microsteps: u16) -> Result<(), Tmc2209Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2209Error::InvalidParameter)?;
        config.microsteps = microsteps;
        let config = config.clone();
        self.apply_uart_mode(&config).await?;
        self.apply_chopper(&config).await
    }

    pub async fn reverse_shaft(&mut self, reversed: bool) -> Result<(), Tmc2209Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2209Error::InvalidParameter)?;
        config.shaft_reverse = reversed;
        let config = config.clone();
        self.apply_uart_mode(&config).await
    }

    pub async fn status(&mut self) -> Result<DriverStatus, Tmc2209Error<T::Error>> {
        let raw = self.read_register(reg::DRV_STATUS::ADDRESS).await?;
        let register_data = reg::DRV_STATUS::new_with_raw_value(raw);
        Ok(DriverStatus::from_raw(register_data))
    }

    pub async fn stall_result(&mut self) -> Result<u16, Tmc2209Error<T::Error>> {
        let raw = self.read_register(reg::SG_RESULT::ADDRESS).await?;
        Ok((raw & 0x3FF) as u16)
    }

    pub async fn heartbeat(&mut self) -> Result<u8, Tmc2209Error<T::Error>> {
        let raw = self.read_register(reg::IFCNT::ADDRESS).await?;
        Ok(raw as u8)
    }

    // ── Helpers ──

    fn mres_from_microsteps(microsteps: u16) -> Option<u4> {
        match microsteps {
            256 => Some(u4::new(0)),
            128 => Some(u4::new(1)),
            64  => Some(u4::new(2)),
            32  => Some(u4::new(3)),
            16  => Some(u4::new(4)),
            8   => Some(u4::new(5)),
            4   => Some(u4::new(6)),
            2   => Some(u4::new(7)),
            1   => Some(u4::new(8)),
            _   => None,
        }
    }

    fn ma_to_cs(current_ma: u16, sense_resistor_mohm: u16) -> Option<u8> {
        // CS = (I_rms * 32 * 1.41421 * (R_sense + 0.02)) / V_fs - 1
        // V_fs = 0.325V (vsense=0) or 0.180V (vsense=1)
        // Simplified integer math version (u64 to avoid overflow at typical values):
        let numerator = (current_ma as u64) * 32 * 1414 * (sense_resistor_mohm as u64 + 20);
        let denominator: u64 = 325 * 1000 * 1000;
        let cs = (numerator / denominator).saturating_sub(1);
        if cs > 31 { None } else { Some(cs as u8) }
    }
}
