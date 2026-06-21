#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::assert_eq;
    use esp_fan::pwm::{PwmConfig, PwmConfigError};
    use esp_hal::{gpio::Level, rmt::PulseCode};

    #[init]
    fn init() {
        let peripherals = esp_hal::init(esp_hal::Config::default());

        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

        rtt_target::rtt_init_defmt!();
    }

    #[test]
    fn pwm_config_success() {
        let _config = PwmConfig::new(80, 25, (0, 1)).unwrap();
    }

    #[test]
    fn pwm_config_invalid() {
        assert_eq!(
            PwmConfig::new(80, 25, (1, 0)).unwrap_err(),
            PwmConfigError::InvalidRange
        );
        assert_eq!(
            PwmConfig::new(0, 25, (0, 1)).unwrap_err(),
            PwmConfigError::RequiresNonZeroCarrier
        );
        assert_eq!(
            PwmConfig::new(80, 0, (0, 1)).unwrap_err(),
            PwmConfigError::RequiresNonZeroPWM
        );
    }

    #[test]
    fn to_pulse_code() {
        let config = PwmConfig::new(80, 25, (0, 15)).unwrap();

        let step = 3200 / 15;

        //lowest - should be the lowest clamped for PWM
        assert_eq!(
            config.to_pulse_code(&0)[0],
            PulseCode::new(Level::High, 5, Level::Low, 3195)
        );
        // first in range
        assert_eq!(
            config.to_pulse_code(&1)[0],
            PulseCode::new(Level::High, step, Level::Low, 3200 - step)
        );
        // second highest
        assert_eq!(
            config.to_pulse_code(&14)[0],
            PulseCode::new(Level::High, 3200 - step - 1, Level::Low, step + 1)
        );
        //max
        assert_eq!(
            config.to_pulse_code(&15)[0],
            PulseCode::new(Level::High, 3195, Level::Low, 5)
        );
        //clamped max
        assert_eq!(
            config.to_pulse_code(&999)[0],
            PulseCode::new(Level::High, 3195, Level::Low, 5)
        );
    }
}
