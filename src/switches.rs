use core::marker::PhantomData;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Async,
    analog::adc::{
        Adc, AdcCalCurve, AdcCalScheme, AdcChannel, AdcConfig, AdcHasCurveCal, AdcHasLineCal,
        AdcPin, Attenuation, CalibrationAccess,
    },
    gpio::AnalogPin,
    peripherals::{ADC1, GPIO0},
};

use super::SwitchValue;

pub static SWITCH_CHANGE_CHANNEL: Channel<CriticalSectionRawMutex, SwitchValue, 2> = Channel::new();
const SAMPLES: usize = 8;

// SwitchesConfig is used to generate the pin and adc
#[derive(Debug)]
pub struct SwitchesConfig<PIN, ADCX, CS> {
    pin: PIN,
    adc: ADCX,
    _phantom: PhantomData<CS>,
}

impl<PIN, ADCX, CS> SwitchesConfig<PIN, ADCX, CS>
where
    PIN: AdcChannel + AnalogPin,
    ADCX: CalibrationAccess + AdcHasCurveCal + AdcHasLineCal + 'static,
    CS: AdcCalScheme<ADCX>,
{
    pub fn new(pin: PIN, adc: ADCX) -> Self {
        Self {
            pin,
            adc,
            _phantom: PhantomData,
        }
    }

    pub fn adc(self) -> (AdcPin<PIN, ADCX, CS>, Adc<'static, ADCX, Async>) {
        let mut adc1_config = AdcConfig::new();

        let pin = adc1_config.enable_pin_with_cal::<PIN, CS>(self.pin, Attenuation::_11dB);
        let adc = Adc::new(self.adc, adc1_config).into_async();

        (pin, adc)
    }
}

// Task loop. Needs to have concrete types due to embassy_executor
#[embassy_executor::task]
pub async fn switch_listener_task(
    mut pin: AdcPin<GPIO0<'static>, ADC1<'static>, AdcCalCurve<ADC1<'static>>>,
    mut adc: Adc<'static, ADC1<'static>, Async>,
) {
    let mut last: SwitchValue = 0;
    let mut buf: [f32; SAMPLES] = [0.0; SAMPLES];

    loop {
        let reading: u16 = adc.read_oneshot(&mut pin).await;
        buf.rotate_right(1);
        buf[0] = f32::from(reading);

        let (mean, stddev) = stats(buf);

        if stddev < 20.0 {
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            let switches_value: SwitchValue = adc_to_u8(mean as u16);
            if last != switches_value {
                SWITCH_CHANGE_CHANNEL.send(switches_value).await;
                last = switches_value;
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}

#[must_use]
pub const fn adc_to_u8(adc_value: u16) -> SwitchValue {
    match adc_value {
        0..160 => 0,
        160..454 => 1,
        454..699 => 2,
        699..957 => 3,
        957..1180 => 4,
        1180..1322 => 5,
        1322..1445 => 6,
        1445..1580 => 7,
        1580..1705 => 8,
        1705..1784 => 9,
        1784..1849 => 10,
        1849..1930 => 11,
        1930..2005 => 12,
        2005..2057 => 13,
        2057..2107 => 14,
        _ => 15,
    }
}

#[must_use]
pub fn stats(values: [f32; SAMPLES]) -> (f32, f32) {
    #[allow(clippy::cast_precision_loss)]
    let samples = SAMPLES as f32;
    let sum: f32 = values.iter().sum();
    let mean = sum / samples;
    let diffs: f32 = values
        .iter()
        .fold(0.0, |acc, v| acc + (v - mean) * (v - mean));

    (mean, libm::sqrtf(diffs / samples))
}
