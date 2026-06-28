use defmt::{debug, info};
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};
use esp_radio::wifi::{Interface, WifiController, WifiError, scan::ScanConfig};

/// This task takes the wifi controller, will attempt keep the connection alive
#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>, stack: Stack<'static>) {
    info!("[WiFi] start connection task");

    loop {
        info!("[WiFi] About to connect...");

        let scan_config = ScanConfig::default().with_max(10);
        let scan_result = controller.scan_async(&scan_config).await.unwrap();
        info!("[WiFi] Found {} APs whilst scanning", scan_result.len());
        for ap in scan_result {
            debug!(
                "[WiFi] AP: {}. Ch: {}, Str: {}db, Auth: {}",
                ap.ssid.as_str(),
                ap.channel,
                ap.signal_strength,
                ap.auth_method
            );
        }

        match controller.connect_async().await {
            Ok(connected_info) => {
                info!(
                    "[WiFi] Connected. AP: {}, Ch: {}, Auth: {}",
                    connected_info.ssid.as_str(),
                    connected_info.channel,
                    connected_info.authmode
                );

                stack.wait_config_up().await;
                if let Some(config) = stack.config_v4() {
                    info!("[WiFi] Got IP: {}", config.address);
                }

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

        Timer::after(Duration::from_millis(2000)).await;
    }
}

// keep the net tasks running
#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}
