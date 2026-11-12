use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// 2D robot velocity command in the body frame (`vx`, `vy` in m/s, `w` in rad/s).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Twist {
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
}

#[cfg(feature = "std")]
impl Twist {
    /// Computes the 3-wheel Kiwi inverse kinematics and scales wheel speeds if any exceeds `MAX_WHEEL_SPEED`.
    pub fn to_wheel_speeds(&self) -> [f32; 3] {
        let radius_inv = 1.0_f32 / crate::consts::WHEEL_RADIUS;

        let rot = crate::consts::WHEEL_BASE * self.w;
        let vx_comp = 3.0_f32.sqrt() / 2.0 * self.vx;
        let vy_half = 0.5 * self.vy;

        let mut speeds = [
            radius_inv * (-self.vy + rot),
            radius_inv * (vx_comp + vy_half + rot),
            radius_inv * (-vx_comp + vy_half + rot),
        ];

        let max_speed = speeds
            .iter()
            .map(|s| s.abs())
            .reduce(f32::max)
            .unwrap_or(0.0);

        if max_speed > crate::consts::MAX_WHEEL_SPEED {
            let scale = crate::consts::MAX_WHEEL_SPEED / max_speed;
            for s in &mut speeds {
                *s *= scale;
            }
        }

        speeds
    }
}
