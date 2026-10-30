pub mod drivers;
pub mod tasks;

use shared::messages::LidarPoint;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    let (gcs_tx, _) = tokio::sync::broadcast::channel::<shared::messages::RpiToGcs>(64);
    let (lidar_tx, _lidar_rx) = watch::channel(Arc::new(Vec::<LidarPoint>::new()));

    tokio::spawn(tasks::lidar::lidar_task(lidar_tx, gcs_tx.clone()));
    tokio::spawn(tasks::gcs::gcs_task(gcs_tx.clone()));
    tokio::spawn(tasks::mcu::mcu_task(gcs_tx.clone()));

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
