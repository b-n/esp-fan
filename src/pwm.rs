use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_hal::{
    Blocking,
    gpio::Level,
    peripherals::RMT,
    rmt::{Channel as RmtChannel, ConfigError, LoopMode, PulseCode, Rmt, Tx, TxChannelConfig},
    time::Rate,
};

use super::SwitchValue;

pub static PWM_CHANNEL: Channel<CriticalSectionRawMutex, SwitchValue, 2> = Channel::new();

#[derive(Debug)]
pub struct PwmConfig {
    carrier_rate_hz: u32,
    ticks: u16,
    range: (u16, u16),
    domain: (u16, u16),
}

impl PwmConfig {
    /// # Panics
    ///
    /// Will panick if the range is inverted. Values must be ascending for range
    #[must_use]
    #[allow(clippy::similar_names)]
    pub fn new(
        carrier_rate_mhz: u32,
        pwm_freq_khz: u32,
        range: (u16, u16),
        domain: (u16, u16),
    ) -> Self {
        let carrier_rate_hz = carrier_rate_mhz * 1_000_000;

        let pwm_freq_hz = pwm_freq_khz * 1_000;
        #[allow(clippy::cast_possible_truncation)]
        let ticks = (carrier_rate_hz / pwm_freq_hz) as u16;

        assert!(
            range.0 < range.1,
            "Range must be incrementing, recieved {range:?}"
        );

        info!("Tick size: {}", ticks);

        Self {
            carrier_rate_hz,
            ticks,
            range,
            domain,
        }
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
        // Clamp the input
        let clamped = (*switches).clamp(self.range.0, self.range.1);

        // deref the range/domain
        let (range_start, range_end) = self.range;
        let (domain_start, domain_end) = self.domain;

        // Get the span of both range and domain
        let range_span = range_start.abs_diff(range_end);
        let domain_span = domain_start.abs_diff(domain_end);
        // And whether the domain is inverted
        let inverse_domain = domain_start > domain_end;

        // how far is the value from the range minimum
        let value_offset = clamped - range_start;

        // do the math. Takes value_offset/range_span (e.g. position in range), and multiples by
        // domain_span for position in the domain. Is the offset by the domain_start to get the
        // true value in the domain. Math is done here to preserve integers etc.
        let domain_value = if inverse_domain {
            let offset = value_offset * domain_span / range_span;
            domain_start - offset
        } else {
            let offset = value_offset * domain_span / range_span;
            domain_start + offset
        };

        // Point of inflection between high to low
        let inflection = self.ticks * domain_value / domain_start.max(domain_end);

        let high_ticks = inflection;
        let low_ticks = self.ticks - high_ticks;
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
