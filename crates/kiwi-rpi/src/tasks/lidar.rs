use crate::drivers;
use std::sync::Arc;
use tokio::sync::watch;

const LIDAR_PORT: &str = "/dev/ttyUSB0";
const LIDAR_BAUD_RATE: u32 = 512_000;

pub async fn lidar_task(tx: watch::Sender<Arc<Vec<drivers::lidar::DataPoint>>>) {
    let mut lidar: drivers::lidar::Lidar = loop {
        let lidar_port_builder = tokio_serial::new(LIDAR_PORT, LIDAR_BAUD_RATE)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .data_bits(tokio_serial::DataBits::Eight);

        match drivers::lidar::Lidar::new(lidar_port_builder) {
            Ok(lidar) => break lidar,
            Err(e) => {
                eprintln!("LIDAR: Failed to open serial port on {LIDAR_PORT}: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    };

    // Start the LIDAR scan until a successful start is achieved
    loop {
        match lidar.start_scan().await {
            Ok(()) => {
                break;
            }
            Err(e) => {
                eprintln!("LIDAR: Error starting the scan: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    loop {
        match lidar.read_scan_full_revolution().await {
            Ok(points) => {
                let _ = tx.send(Arc::new(points));
            }
            Err(e) => {
                eprintln!("LIDAR: Error reading scan packet: {e}");
            }
        }
    }
}
