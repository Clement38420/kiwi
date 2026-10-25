pub mod drivers;
pub mod tasks;

use shared::messages::LidarPoint;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::time;

#[tokio::main]
async fn main() {
    let (lidar_tx, lidar_rx) = watch::channel(Arc::new(Vec::<LidarPoint>::new()));

    tokio::spawn(tasks::lidar::lidar_task(lidar_tx));
    tokio::spawn(tasks::gcs::gcs_task(lidar_rx));

    loop {
        time::sleep(time::Duration::from_secs(1)).await;
    }
}
