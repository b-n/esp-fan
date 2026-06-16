#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use esp_hal::{
    clock::CpuClock,
    gpio::{
        DriveMode, Level, Output, OutputConfig, OutputSignal, Pull, interconnect::PeripheralOutput,
    },
    rmt::TxChannelCreator,
    timer::timg::TimerGroup,
};
use panic_rtt_target as _;

use esp_fan::{
    pwm::{PWM_CHANNEL, PwmConfig, pwm_task},
    switches::{SWITCH_CHANGE_CHANNEL, SwitchesConfig, switch_listener_task},
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c3 -o unstable-hal -o embassy -o probe-rs -o defmt -o panic-rtt-target -o embedded-test
    rtt_target::rtt_init_defmt!();

    // Configure the boot
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Configure the main timer for looping
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    info!("Embassy initialized!");

    // Setup Switch
    let switches_config = SwitchesConfig::new(peripherals.GPIO0, peripherals.ADC1);
    let (pin, adc) = switches_config.adc();
    // Spawn listener task
    spawner.spawn(switch_listener_task(pin, adc).unwrap());

    // Setup PWM outputs
    let pwm_config = PwmConfig::new(80, 25, (0, 15), (1, 16));
    let rmt_channel = pwm_config
        .rmt(peripherals.RMT)
        .unwrap()
        .channel0
        .configure_tx(&pwm_config.tx_config())
        .unwrap();

    // Connect all the GPIOs up to the PWM channel (controlled via RMT)
    let out_config = OutputConfig::default()
        .with_drive_mode(DriveMode::PushPull)
        .with_pull(Pull::Down);
    Output::new(peripherals.GPIO9, Level::Low, out_config)
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    Output::new(peripherals.GPIO10, Level::Low, out_config)
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    Output::new(peripherals.GPIO20, Level::Low, out_config)
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    Output::new(peripherals.GPIO21, Level::Low, out_config)
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);

    // Spawn async tasks
    spawner.spawn(pwm_task(pwm_config, rmt_channel).unwrap());

    // main loop
    loop {
        let switches_value = SWITCH_CHANGE_CHANNEL.receive().await;
        PWM_CHANNEL.send(switches_value).await;
        info!("{}", switches_value);
    }
}
