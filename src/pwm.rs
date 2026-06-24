use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_hal::{
    Blocking,
    gpio::Level,
    peripherals::RMT,
    rmt::{Channel as RmtChannel, ConfigError, LoopMode, PulseCode, Rmt, Tx, TxChannelConfig},
    time::Rate,
};
use utils::maths::RangeDomainMapper;

use super::SwitchValue;

pub static PWM_CHANNEL: Channel<CriticalSectionRawMutex, SwitchValue, 2> = Channel::new();

static PWM_BUFFER: u16 = 5;

#[derive(Debug, Eq, PartialEq, defmt::Format)]
pub enum PwmConfigError {
    InvalidRange,
    RequiresNonZeroCarrier,
    RequiresNonZeroPWM,
}

#[derive(Debug)]
pub struct PwmConfig {
    carrier_rate_hz: u32,
    mapper: RangeDomainMapper<u32>,
    ticks_per_pwm: u16,
}

impl PwmConfig {
    /// # Errors
    ///
    /// Returns `PwmConfigError`:
    /// - `InvalidRange` - when range goes from high to low
    #[allow(clippy::similar_names)]
    pub fn new(
        carrier_rate_mhz: u32,
        pwm_freq_khz: u32,
        range: (u16, u16),
    ) -> Result<Self, PwmConfigError> {
        if range.0 > range.1 {
            return Err(PwmConfigError::InvalidRange);
        }
        if carrier_rate_mhz == 0 {
            return Err(PwmConfigError::RequiresNonZeroCarrier);
        }
        if pwm_freq_khz == 0 {
            return Err(PwmConfigError::RequiresNonZeroPWM);
        }

        let carrier_rate_hz = carrier_rate_mhz * 1_000_000;
        let pwm_freq_hz = pwm_freq_khz * 1_000;

        #[allow(clippy::cast_possible_truncation)]
        let ticks_per_pwm = (carrier_rate_hz / pwm_freq_hz) as u16;
        info!("[PWM] Ticks per PWM: {}", ticks_per_pwm);

        let mapper = RangeDomainMapper::new(
            (u32::from(range.0), u32::from(range.1)),
            (0, u32::from(ticks_per_pwm)),
        );

        Ok(Self {
            carrier_rate_hz,
            mapper,
            ticks_per_pwm,
        })
    }

    /// # Errors
    ///
    /// Returns `rmt::ConfigError` if invalid peripheral/carrier rate
    pub fn rmt(
        &self,
        peripherals_rmt: RMT<'static>,
    ) -> Result<Rmt<'static, Blocking>, ConfigError> {
        Rmt::new(peripherals_rmt, Rate::from_hz(self.carrier_rate_hz))
    }

    #[must_use]
    pub fn tx_config(&self) -> TxChannelConfig {
        TxChannelConfig::default().with_clk_divider(1)
    }

    #[must_use]
    pub fn to_pulse_code(&self, switches: &SwitchValue) -> [PulseCode; 2] {
        // Explanation of PWM_BUFFER: Who knows why, but if the inflection = 0 || ticks_per_pwm,
        // then the pulse code ends up being the opposite of what you'd think it should be. This
        // happens even if the inflection is at 1/ticks_per_pwm - 1.
        #[allow(clippy::cast_possible_truncation)]
        let inflection = (self.mapper.value(&u32::from(*switches)) as u16)
            .clamp(PWM_BUFFER, self.ticks_per_pwm - PWM_BUFFER);

        // If inflection == 0, for some reason PulseCode is always set to high
        let high_ticks: u16 = inflection;
        let low_ticks = self.ticks_per_pwm - high_ticks;
        info!("[PWM] high ticks: {}, low ticks: {}", high_ticks, low_ticks);
        [
            PulseCode::new(Level::High, high_ticks, Level::Low, low_ticks),
            PulseCode::end_marker(),
        ]
    }
}

#[embassy_executor::task]
pub async fn pwm_task(config: PwmConfig, channel: RmtChannel<'static, Blocking, Tx>) {
    let pwm: SwitchValue = 0;
    let mut transaction = channel
        .transmit_continuously(&config.to_pulse_code(&pwm), LoopMode::Infinite)
        .unwrap();

    loop {
        let switches = PWM_CHANNEL.receive().await;
        let returned_channel = transaction.stop().unwrap();

        transaction = returned_channel
            .transmit_continuously(&config.to_pulse_code(&switches), LoopMode::Infinite)
            .unwrap();
    }
}
