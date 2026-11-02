pub mod drivers;
pub mod tasks;

use shared::messages::{LidarPoint, McuState, RobotState};
use std::sync::{Arc, RwLock};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    let robot_state = Arc::new(RwLock::new(RobotState {
        mcu: McuState::default(),
    }));

    let (gcs_tx, _) = tokio::sync::broadcast::channel::<shared::messages::RpiToGcs>(64);
    let (mcu_tx, _) = tokio::sync::broadcast::channel::<shared::messages::RpiToMcu>(64);
    let (lidar_tx, _lidar_rx) = watch::channel(Arc::new(Vec::<LidarPoint>::new()));

    tokio::spawn(tasks::lidar::lidar_task(lidar_tx, gcs_tx.clone()));
    tokio::spawn(tasks::gcs::gcs_task(
        gcs_tx.clone(),
        mcu_tx.clone(),
        robot_state.clone(),
    ));
    tokio::spawn(tasks::mcu::mcu_task(
        mcu_tx.clone(),
        gcs_tx.clone(),
        robot_state.clone(),
    ));

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
