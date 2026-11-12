use crate::channels::{POSE, subscribe_wheel_speeds};
use embassy_time::Duration;
use micromath::F32Ext;
use shared::consts::WHEEL_RADIUS;

const SQRT_3: f32 = 1.732_050_8;
const WHEEL_RADIUS_DIV_SQRT_3: f32 = WHEEL_RADIUS / SQRT_3;
const WHEEL_RADIUS_DIV_3: f32 = WHEEL_RADIUS / 3.0;

#[embassy_executor::task]
pub async fn odometry_task() {
    let mut wheel_speeds = subscribe_wheel_speeds();

    let mut last_time = embassy_time::Instant::now();
    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(1));
    loop {
        ticker.next().await;
        let speeds = wheel_speeds.try_get().unwrap();

        let vx = WHEEL_RADIUS_DIV_SQRT_3 * (speeds[1] - speeds[2]);
        let vy = WHEEL_RADIUS_DIV_3 * (-2.0 * speeds[0] + speeds[1] + speeds[2]);

        let now = embassy_time::Instant::now();
        let dt = now - last_time;
        last_time = now;

        POSE.sender().send_modify(|pose| {
            if let Some(pose) = pose {
                let (sin_t, cos_t) = pose.theta.sin_cos();
                let dt_s = dt.as_micros() as f32 * 1e-6;
                let dx = vx * dt_s;
                let dy = vy * dt_s;

                pose.x += dx * cos_t - dy * sin_t;
                pose.y += dx * sin_t + dy * cos_t;
            }
        });
    }
}
