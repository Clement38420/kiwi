use crate::channels::{Base, Motion};
use shared::geometry::Twist;
use shared::messages::{ControlMode, RpiToMcu};
use tokio::sync::broadcast;

pub async fn motion_task(motion: Motion, base: Base, mcu_tx: broadcast::Sender<RpiToMcu>) {
    let pose_cmd_rx = motion.pose_cmd.subscribe();
    let pose_rx = base.pose.subscribe();
    let state_rx = base.state.subscribe();

    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));
    loop {
        ticker.tick().await;

        if state_rx.borrow().mode == ControlMode::Manual {
            continue;
        }

        let robot_pose_cmd = *pose_cmd_rx.borrow();
        let robot_pose = *pose_rx.borrow();
        let err = robot_pose_cmd - robot_pose;

        let wheel_speeds = Twist {
            vx: err.x * 1.0,
            vy: err.y * 1.0,
            w: err.theta * 1.0,
        }
        .rotated(-robot_pose.theta)
        .to_wheel_speeds();

        let _ = mcu_tx.send(RpiToMcu::WheelSpeedSetpoints(wheel_speeds));
    }
}
