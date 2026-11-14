use shared::geometry::Pose2D;
use shared::messages::{LidarPoint, LidarScan, RobotState, RpiToGcs, RpiToMcu};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};

#[derive(Clone)]
pub struct Base {
    pub lidar: watch::Sender<Arc<Vec<LidarPoint>>>,
    pub pose: watch::Sender<Pose2D>,
    pub odom: watch::Sender<Pose2D>,
    pub state: watch::Sender<RobotState>,
    pub reset: broadcast::Sender<()>,
}

impl Default for Base {
    fn default() -> Self {
        let (reset, _) = broadcast::channel(16);
        Self {
            lidar: watch::Sender::new(Arc::new(Vec::new())),
            pose: watch::Sender::new(Pose2D::default()),
            odom: watch::Sender::new(Pose2D::default()),
            state: watch::Sender::new(RobotState::default()),
            reset,
        }
    }
}

#[derive(Clone)]
pub struct Communication {
    pub gcs: broadcast::Sender<RpiToGcs>,
    pub mcu: broadcast::Sender<RpiToMcu>,
}

impl Default for Communication {
    fn default() -> Self {
        let (gcs, _) = broadcast::channel(64);
        let (mcu, _) = broadcast::channel(64);
        Self { gcs, mcu }
    }
}

#[derive(Clone)]
pub struct Motion {
    pub pose_cmd: watch::Sender<Pose2D>,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            pose_cmd: watch::Sender::new(Pose2D::default()),
        }
    }
}

#[derive(Clone, Default)]
pub struct Channels {
    pub base: Base,
    pub comm: Communication,
    pub motion: Motion,
}

pub async fn gcs_relay_task(channels: Channels) {
    let mut robot_pose_rx = channels.base.pose.subscribe();
    let mut lidar_rx = channels.base.lidar.subscribe();
    let mut robot_state_rx = channels.base.state.subscribe();

    loop {
        tokio::select! {
            Ok(()) = robot_pose_rx.changed() => {
                let pose = *robot_pose_rx.borrow_and_update();
                let _ = channels.comm.gcs.send(RpiToGcs::RobotPose(pose));
            }
            Ok(()) = lidar_rx.changed() => {
                let scan = lidar_rx.borrow_and_update().clone();
                let _ = channels.comm.gcs.send(RpiToGcs::LidarScan(LidarScan {
                    points: scan,
                    pose: *channels.base.pose.borrow(),
                }));
            }
            Ok(()) = robot_state_rx.changed() => {
                let state = *robot_state_rx.borrow_and_update();
                let _ = channels.comm.gcs.send(RpiToGcs::RobotState(state));
            }
            else => break,
        }
    }
}
