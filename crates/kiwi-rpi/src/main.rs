pub mod drivers;
pub mod tasks;

use shared::messages::{LidarPoint, RobotPose, RobotState};
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{broadcast, watch};

#[tokio::main]
async fn main() {
    let (gcs_tx, _) = broadcast::channel::<shared::messages::RpiToGcs>(64);
    let (mcu_tx, _) = broadcast::channel::<shared::messages::RpiToMcu>(64);
    let (lidar_tx, mut lidar_rx) = watch::channel(Arc::new(Vec::<LidarPoint>::new()));
    let (robot_pose_tx, mut robot_pose_rx) = watch::channel(RobotPose::default());
    let (robot_state_tx, mut robot_state_rx) = watch::channel(RobotState::default());

    tokio::spawn(tasks::lidar::lidar_task(lidar_tx));
    tokio::spawn(tasks::gcs::gcs_task(
        gcs_tx.clone(),
        mcu_tx.clone(),
        robot_state_tx.clone(),
        robot_pose_tx.clone(),
    ));
    tokio::spawn(tasks::mcu::mcu_task(
        mcu_tx.clone(),
        gcs_tx.clone(),
        robot_state_tx.clone(),
    ));
    tokio::spawn(tasks::odometry::odometry_task(
        lidar_rx.clone(),
        robot_pose_tx.clone(),
    ));

    let gcs_pose_tx = gcs_tx.clone();
    tokio::spawn(async move {
        while robot_pose_rx.changed().await.is_ok() {
            let pose = *robot_pose_rx.borrow_and_update();
            let _ = gcs_pose_tx.send(shared::messages::RpiToGcs::RobotPose(pose));
        }
    });

    let gcs_lidar_tx = gcs_tx.clone();
    tokio::spawn(async move {
        while lidar_rx.changed().await.is_ok() {
            let scan = lidar_rx.borrow_and_update().clone();
            let _ = gcs_lidar_tx.send(shared::messages::RpiToGcs::LidarScan(scan));
        }
    });

    let gcs_robot_state_tx = gcs_tx.clone();
    tokio::spawn(async move {
        while robot_state_rx.changed().await.is_ok() {
            let state = *robot_state_rx.borrow_and_update();
            let _ = gcs_robot_state_tx.send(shared::messages::RpiToGcs::RobotState(state));
        }
    });

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
