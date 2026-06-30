#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, DhcpConfig, StackResources};
use esp_hal::{
    clock::CpuClock,
    gpio::{OutputSignal, interconnect::PeripheralOutput},
    rmt::TxChannelCreator,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{
    Config as WifiConfig, ControllerConfig as WifiControllerConfig, Interface as WifiInterface,
    WifiController, sta::StationConfig,
};
use panic_rtt_target as _;
use static_cell::StaticCell;

use esp_fan::{
    http::{HttpServer, run_http_server},
    pwm::{PWM_CHANNEL, PwmConfig, pwm_task},
    switches::{SWITCH_CHANGE_CHANNEL, SwitchesConfig, switch_listener_task},
    wifi::{connection, net_task},
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

static EMBASSY_STACK: StaticCell<StackResources<3>> = StaticCell::new();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    //// ESP setup
    rtt_target::rtt_init_defmt!();
    // Configure the esp32 chip (with an allocator)
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);
    // Configure the main timer for looping
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    info!("Embassy initialized!");

    //// WiFi
    // Station
    let station_config = WifiConfig::Station(
        StationConfig::default()
            .with_ssid(WIFI_SSID)
            .with_password(WIFI_PASSWORD.into()),
    );
    // Controller
    info!("[WiFi] Starting controller");
    let wifi_interface = WifiInterface::station();
    let wifi_controller = WifiController::new(
        peripherals.WIFI,
        WifiControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");
    info!(
        "[WiFi] controller started and configured to connect to station {}",
        WIFI_SSID
    );
    // Networking stack setup
    let net_config = NetConfig::dhcpv4(DhcpConfig::default());
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    let stack_resources = EMBASSY_STACK.init(StackResources::<3>::new());
    let (stack, runner) = embassy_net::new(wifi_interface, net_config, stack_resources, seed);
    // Pass the wifi controller and net handling to background tasks
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(connection(wifi_controller, stack).unwrap());

    //// HTTP Server
    // Start http server
    let http_server = HttpServer::default();
    spawner.spawn(run_http_server(stack, http_server).unwrap());

    //// Switch ADC Setup
    // Setup Switch ADC listener
    let switches_config = SwitchesConfig::new(peripherals.GPIO0, peripherals.ADC1);
    let (pin, adc) = switches_config.adc();
    // Spawn listener task
    spawner.spawn(switch_listener_task(pin, adc).unwrap());

    //// Output PWM Setup
    // Setup PWM outputs
    let pwm_config = PwmConfig::new(80, 25, (0, 15)).unwrap();
    let rmt_channel = pwm_config
        .rmt(peripherals.RMT)
        .unwrap()
        .channel0
        .configure_tx(&pwm_config.tx_config())
        .unwrap();
    // Connect all the GPIOs up to the PWM channel (controlled via RMT)
    // Note: These GPIOs need inline resistors, using OutputConfig with pullup/down does not
    // connect the internal gpio resistors by the looks sadly.
    peripherals
        .GPIO9
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    peripherals
        .GPIO10
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    peripherals
        .GPIO20
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    peripherals
        .GPIO21
        .connect_peripheral_to_output(OutputSignal::RMT_SIG_0);
    // Spawn pwm task
    spawner.spawn(pwm_task(pwm_config, rmt_channel).unwrap());

    //// Main loop
    // - Route switches events to the PWM output
    loop {
        let switches_value = SWITCH_CHANGE_CHANNEL.receive().await;
        PWM_CHANNEL.send(switches_value).await;
        info!("Switches value: {}", switches_value);
    }
}
