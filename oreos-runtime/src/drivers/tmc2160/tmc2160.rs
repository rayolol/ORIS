use arbitrary_int::{u2, u3, u4, u5, u7};
use crate::transport::traits::{TransportHandler, RequestType};
use super::registers::{self as reg, Frame, Register};
use super::structs::*;




pub struct Tmc2160<T: TransportHandler<5>> {
    transport: T,
    config: Option<Tmc2160Config>,
}

impl<T: TransportHandler<5>> Tmc2160<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            config: None,
        }
    }

    pub async fn init(&mut self, config: Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        self.apply_short_conf(&config).await?;
        self.apply_driver_conf(&config).await?;
        self.apply_global_scaler(&config).await?;
        self.apply_gconf(&config).await?;
        self.apply_chopper(&config).await?;
        self.apply_current(&config).await?;
        self.apply_standstill(&config).await?;
        self.apply_stealthchop(&config).await?;

        if let Some(ref sg) = config.stallguard {
            self.apply_stall_detection(sg).await?;
            if let Some(ref cs) = sg.coolstep {
                self.apply_adaptive_current(cs).await?;
            }
        }

        self.config = Some(config);
        Ok(())
    }

    pub async fn reconfigure(&mut self, config: Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        self.init(config).await
    }

    // ── Private register writers ──

    async fn apply_gconf(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        let payload = reg::GCONF::new_with_raw_value(0)
            .with_en_pwm_mode(config.stealthchop)
            .with_shaft(config.shaft_reverse)
            .with_multistep_filt(config.advanced.multistep_filt);

        self.write_register(reg::GCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_short_conf(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        let payload = reg::SHORT_CONF::new_with_raw_value(0)
            .with_s2vs_level(u4::new(config.advanced.s2vs_level))
            .with_s2gnd_level(u4::new(config.advanced.s2gnd_level));

        self.write_register(reg::SHORT_CONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_driver_conf(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        let payload = reg::DRVCONF::new_with_raw_value(0)
            .with_bbmtime(u5::new(config.advanced.bbmtime))
            .with_bbmclks(u4::new(config.advanced.bbmclks))
            .with_drvstrength(u2::new(config.advanced.drvstrength))
            .with_filt_isense(u2::new(config.advanced.filt_isense));

        self.write_register(reg::DRVCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_global_scaler(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        self.write_register(reg::GLOBALSCALER::ADDRESS, config.global_scaler as u32).await
    }

    async fn apply_chopper(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        let mres = Self::mres_from_microsteps(config.microsteps)
            .ok_or(Tmc2160Error::InvalidMicrosteps)?;

        let payload = reg::CHOPCONF::new_with_raw_value(0)
            .with_mres(mres)
            .with_intpol(config.interpolation)
            .with_toff(u4::new(config.advanced.toff))
            .with_hstrt(u3::new(config.advanced.hstrt))
            .with_hend(u4::new(config.advanced.hend))
            .with_tbl(u2::new(config.advanced.tbl))
            .with_chm(config.advanced.chm);

        self.write_register(reg::CHOPCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_current(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        let irun = Self::ma_to_cs(config.run_current_ma, config.sense_resistor_mohm, config.global_scaler)
            .ok_or(Tmc2160Error::CurrentOutOfRange)?;
        let ihold = Self::ma_to_cs(config.hold_current_ma, config.sense_resistor_mohm, config.global_scaler)
            .ok_or(Tmc2160Error::CurrentOutOfRange)?;

        let payload = reg::IHOLDIRUN::new_with_raw_value(0)
            .with_irun(u5::new(irun))
            .with_ihold(u5::new(ihold))
            .with_iholddelay(u4::new(config.advanced.iholddelay));

        self.write_register(reg::IHOLDIRUN::ADDRESS, payload.raw_value()).await
    }

    async fn apply_standstill(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        self.write_register(reg::TPOWERDOWN::ADDRESS, config.advanced.tpowerdown as u32).await
    }

    async fn apply_stealthchop(&mut self, config: &Tmc2160Config) -> Result<(), Tmc2160Error<T::Error>> {
        if let Some(threshold) = config.stealthchop_threshold {
            self.write_register(reg::TPWMTHRS::ADDRESS, threshold).await?;
        }

        let payload = reg::PWMCONF::new_with_raw_value(0)
            .with_pwm_autoscale(config.advanced.pwm_autoscale)
            .with_pwm_autograd(config.advanced.pwm_autograd)
            .with_pwm_freq(u2::new(config.advanced.pwm_freq))
            .with_pwm_reg(u4::new(config.advanced.pwm_reg))
            .with_pwm_lim(u4::new(config.advanced.pwm_lim))
            .with_pwm_ofs(config.advanced.pwm_ofs)
            .with_pwm_grad(config.advanced.pwm_grad)
            .with_freewheel(u2::new(config.advanced.freewheel as u8));

        self.write_register(reg::PWMCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_stall_detection(&mut self, sg: &StallGuardConfig) -> Result<(), Tmc2160Error<T::Error>> {
        self.write_register(reg::TCOOLTHRS::ADDRESS, sg.velocity_threshold).await?;

        // sgt is signed 7-bit — cast to u8 and mask to 7 bits (two's complement)
        let sgt_raw = u7::new((sg.threshold as u8) & 0x7F);
        let payload = reg::COOLCONF::new_with_raw_value(0).with_sgt(sgt_raw);

        self.write_register(reg::COOLCONF::ADDRESS, payload.raw_value()).await
    }

    async fn apply_adaptive_current(&mut self, cs: &CoolStepConfig) -> Result<(), Tmc2160Error<T::Error>> {
        // Read current COOLCONF (sgt may already be set) then merge
        let current = self.read_register(reg::COOLCONF::ADDRESS).await?;

        let payload = reg::COOLCONF::new_with_raw_value(current)
            .with_semin(u4::new(cs.semin))
            .with_semax(u4::new(cs.semax))
            .with_seup(u2::new(cs.seup))
            .with_sedn(u2::new(cs.sedn))
            .with_seimin(cs.quarter_current);

        self.write_register(reg::COOLCONF::ADDRESS, payload.raw_value()).await
    }

    // ── Generic register IO ──

    async fn write_register(&mut self, address: u8, data: u32) -> Result<(), Tmc2160Error<T::Error>> {
        let frame = reg::WriteFrame::make_frame(data, address);
        let _reply = self.transport.transfer(frame, RequestType::Write).await.map_err(Tmc2160Error::Transport)?;
        Ok(())
    }

    // TMC2160 SPI read: send read request, then a dummy frame to clock out the data
    async fn read_register(&mut self, address: u8) -> Result<u32, Tmc2160Error<T::Error>> {
        // Step 1: send read request (write_flag = 0)
        let read_req: [u8; 5] = [address & 0x7F, 0, 0, 0, 0];
        self.transport.transfer(read_req, RequestType::Read).await.map_err(Tmc2160Error::Transport)?;

        // Step 2: send dummy frame — response now contains the requested register data
        let dummy: [u8; 5] = [0, 0, 0, 0, 0];
        let result = self.transport.transfer(dummy, RequestType::Read).await.map_err(Tmc2160Error::Transport)?;
        let reply = result.ok_or(Tmc2160Error::NoReply)?;

        let raw = u32::from_be_bytes([reply[1], reply[2], reply[3], reply[4]]);
        Ok(raw)
    }

    // ── Public runtime methods ──

    pub async fn set_run_current_ma(&mut self, ma: u16) -> Result<(), Tmc2160Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2160Error::InvalidParameter)?;
        config.run_current_ma = ma;
        let config = config.clone();
        self.apply_current(&config).await
    }

    pub async fn set_microsteps(&mut self, microsteps: u16) -> Result<(), Tmc2160Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2160Error::InvalidParameter)?;
        config.microsteps = microsteps;
        let config = config.clone();
        self.apply_chopper(&config).await
    }

    pub async fn reverse_shaft(&mut self, reversed: bool) -> Result<(), Tmc2160Error<T::Error>> {
        let config = self.config.as_mut().ok_or(Tmc2160Error::InvalidParameter)?;
        config.shaft_reverse = reversed;
        let config = config.clone();
        self.apply_gconf(&config).await
    }

    pub async fn status(&mut self) -> Result<DriverStatus, Tmc2160Error<T::Error>> {
        let raw = self.read_register(reg::DRV_STATUS::ADDRESS).await?;
        let register_data = reg::DRV_STATUS::new_with_raw_value(raw);
        Ok(DriverStatus::from_raw(register_data))
    }

    pub async fn stall_result(&mut self) -> Result<u16, Tmc2160Error<T::Error>> {
        let raw = self.read_register(reg::DRV_STATUS::ADDRESS).await?;
        Ok((raw & 0x3FF) as u16)
    }

    pub async fn heartbeat(&mut self) -> Result<u8, Tmc2160Error<T::Error>> {
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

    fn ma_to_cs(current_ma: u16, sense_resistor_mohm: u16, global_scaler: u8) -> Option<u8> {
        // CS = (I_rms * 32 * sqrt(2) * (R_sense + 0.02)) / (V_fs * global_scale) - 1
        // V_fs = 0.325V, global_scale = (global_scaler == 0 ? 256 : global_scaler) / 256
        let scale = if global_scaler == 0 { 256u32 } else { global_scaler as u32 };
        let numerator = (current_ma as u32) * 32 * 1414 * (sense_resistor_mohm as u32 + 20) * 256;
        let denominator = 325 * 1000 * 1000 * scale;
        let cs = (numerator / denominator).saturating_sub(1);
        if cs > 31 { None } else { Some(cs as u8) }
    }
}
