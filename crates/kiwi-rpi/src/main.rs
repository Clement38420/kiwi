pub mod channels;
pub mod drivers;
pub mod tasks;

use channels::Channels;
use shared::geometry::Pose2D;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() {
    let channels = Channels::default();

    tokio::spawn(tasks::lidar::lidar_task(channels.base.clone()));
    tokio::spawn(tasks::gcs::gcs_task(
        channels.comm.clone(),
        channels.base.clone(),
    ));
    tokio::spawn(tasks::mcu::mcu_task(
        channels.comm.clone(),
        channels.base.clone(),
    ));
    tokio::spawn(tasks::odometry::odometry_task(channels.base.clone()));
    tokio::spawn(tasks::motion::motion_task(
        channels.motion.clone(),
        channels.base.clone(),
        channels.comm.mcu.clone(),
    ));
    tokio::spawn(channels::gcs_relay_task(channels.clone()));


    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
