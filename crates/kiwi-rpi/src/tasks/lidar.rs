use crate::channels::Base;
use crate::drivers;
use shared::messages::LidarPoint;
use std::sync::Arc;

const LIDAR_PORT: &str = "/dev/ttyUSB0";
const LIDAR_BAUD_RATE: u32 = 512_000;

impl From<drivers::lidar::DataPoint> for LidarPoint {
    fn from(pt: drivers::lidar::DataPoint) -> Self {
        Self {
            angle: pt.angle,
            distance: pt.distance,
        }
    }
}

pub async fn lidar_task(base: Base) {
    loop {
        let mut lidar: drivers::lidar::Lidar = loop {
            let lidar_port_builder = tokio_serial::new(LIDAR_PORT, LIDAR_BAUD_RATE)
                .parity(tokio_serial::Parity::None)
                .stop_bits(tokio_serial::StopBits::One)
                .data_bits(tokio_serial::DataBits::Eight);

            match drivers::lidar::Lidar::new(lidar_port_builder) {
                Ok(lidar) => break lidar,
                Err(e) => {
                    eprintln!(
                        "LIDAR: Failed to open serial port on {LIDAR_PORT}: {e}, retrying in 5 seconds..."
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        };

        // Start the LIDAR scan; if the serial port fails, reopen it from scratch
        if let Err(e) = lidar.start_scan().await {
            eprintln!("LIDAR: Error starting the scan: {e}, retrying in 5 seconds...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

        loop {
            match lidar.read_scan_full_revolution().await {
                Ok(points) => {
                    let scan: Arc<Vec<LidarPoint>> =
                        Arc::new(points.into_iter().map(Into::into).collect());
                    base.lidar.send_replace(scan);
                }
                Err(e @ drivers::lidar::LidarError::CorruptPacket) => {
                    eprintln!("LIDAR: {e}");
                }
                Err(e) => {
                    eprintln!("LIDAR: Serial connection lost ({e}), reconnecting in 5 seconds...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    break;
                }
            }
        }
    }
}
