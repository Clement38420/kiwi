pub mod drivers;
pub mod tasks;

use std::sync::Arc;
use tokio::sync::watch;
use tokio::time;

#[tokio::main]
async fn main() {
    let (lidar_tx, _lidar_rx) = watch::channel(Arc::new(Vec::<drivers::lidar::DataPoint>::new()));

    tokio::spawn(tasks::lidar::lidar_task(lidar_tx));
    tokio::spawn(tasks::gcs::gcs_task());

    loop {
        time::sleep(time::Duration::from_secs(1)).await;
    }
}
