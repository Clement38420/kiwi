use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "std")]
use crate::geometry::Twist;
use crate::geometry::{PointPolar, Pose2D};
#[cfg(feature = "std")]
use crate::rpc::{RpcRequest, RpcResponse};

/// Raw LiDAR scan sample (`angle` in radians clockwise from robot forward, `distance` in meters).
///
/// Convert into [`PointPolar`] (CCW convention) via `PointPolar::from(...)`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct LidarPoint {
    pub angle: f32,
    pub distance: f32,
}

impl From<LidarPoint> for PointPolar {
    /// Converts a raw LiDAR sample (clockwise angle) into a standard CCW polar point.
    fn from(p: LidarPoint) -> Self {
        Self::new(p.distance, -p.angle)
    }
}

impl From<&LidarPoint> for PointPolar {
    fn from(p: &LidarPoint) -> Self {
        Self::from(*p)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum BatteryLevel {
    High,
    Medium,
    Low,
    Critical,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum ArmState {
    Armed,
    Disarmed,
    CannotArm,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(TS))]
pub struct McuState {
    pub battery_voltage: f32,
    pub battery_level: BatteryLevel,
    pub arm: ArmState,
}

impl Default for McuState {
    fn default() -> Self {
        McuState {
            battery_voltage: 0.0,
            battery_level: BatteryLevel::Critical,
            arm: ArmState::CannotArm,
        }
    }
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RobotState {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub mcu: McuState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum McuToRpi {
    BatteryVoltage(f32),
    BatteryLevel(BatteryLevel),
    ArmState(ArmState),
    McuState(McuState),
    Odom(Pose2D),
    ResetAck,
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum RpiToGcs {
    LidarScan(std::sync::Arc<Vec<LidarPoint>>),
    RobotState(RobotState),
    GetStateRes(RpcResponse<RobotState>),
    RobotPose(Pose2D),
}

#[cfg(feature = "std")]
impl TryFrom<McuToRpi> for RpiToGcs {
    type Error = ();

    fn try_from(value: McuToRpi) -> Result<Self, Self::Error> {
        match value {
            McuToRpi::BatteryVoltage(_)
            | McuToRpi::BatteryLevel(_)
            | McuToRpi::ArmState(_)
            | McuToRpi::McuState(_)
            | McuToRpi::Odom(_)
            | McuToRpi::ResetAck => Err(()),
        }
    }
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum GcsToRpi {
    Arm(bool),
    GetStateReq(RpcRequest<()>),
    Reset,
    Twist(Twist),
}

#[cfg(feature = "std")]
impl TryFrom<GcsToRpi> for RpiToMcu {
    type Error = ();

    fn try_from(value: GcsToRpi) -> Result<Self, Self::Error> {
        match value {
            GcsToRpi::Arm(v) => Ok(RpiToMcu::Arm(v)),
            GcsToRpi::GetStateReq(_) => Err(()),
            GcsToRpi::Reset => Ok(RpiToMcu::Reset),
            GcsToRpi::Twist(_) => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RpiToMcu {
    WheelSpeedSetpoints([f32; 3]),
    Arm(bool),
    Sync,
    Reset,
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::geometry::Point2D;

    #[test]
    fn lidar_point_to_point2d_preserves_cw_convention() {
        let lidar = LidarPoint {
            angle: core::f32::consts::FRAC_PI_2,
            distance: 2.0,
        };
        let cart = Point2D::from(PointPolar::from(lidar));
        assert!(cart.x.abs() < 1e-5);
        assert!((cart.y - (-2.0)).abs() < 1e-5);
    }
}
