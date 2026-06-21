use defmt::info;
use embassy_net::Runner;
use embassy_time::{Duration, Timer};
use esp_radio::wifi::{Interface, WifiController, WifiError};

/// This task takes the wifi controller, will attempt keep the connection alive
#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    info!("[WiFi] start connection task");

    loop {
        info!("[WiFi] About to connect...");

        match controller.connect_async().await {
            Ok(connected_info) => {
                info!(
                    "[WiFi] Connected. AP: {}, Ch: {}, Auth: {}",
                    connected_info.ssid.as_str(),
                    connected_info.channel,
                    connected_info.authmode
                );

                // wait until we're no longer connected
                let disconnect_info = controller.wait_for_disconnect_async().await.ok();
                if let Some(i) = disconnect_info {
                    info!(
                        "[WiFi] Disconnected. AP: {}, Reason: {}",
                        i.ssid.as_str(),
                        i.reason
                    );
                } else {
                    info!("[WiFi] Disconnected. {}", disconnect_info);
                }
            }
            Err(e) => {
                if let WifiError::Disconnected(i) = e {
                    info!(
                        "[WiFi] Failed to connect: AP: {}, Reason: {}",
                        i.ssid.as_str(),
                        i.reason
                    );
                } else {
                    info!("[WiFi] Failed to connect: {:?}", e);
                }
            }
        }

        Timer::after(Duration::from_millis(5000)).await;
    }
}

// keep the net tasks running
#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}
