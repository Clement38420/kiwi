use crate::channels::POSE;
use crate::drivers;
use core::f32::consts::{PI, TAU};
use defmt::info;
use embassy_time::{Duration, Instant};

#[embassy_executor::task]
pub async fn imu_task(mut imu: drivers::mpu6050::Mpu6050<'static>) {
    loop {
        match imu.calibrate(None).await {
            Ok(()) => break,
            Err(e) => {
                info!(
                    "Error calibrating IMU: {:?}, retrying in 1 second...",
                    defmt::Debug2Format(&e)
                );
                embassy_time::Timer::after(Duration::from_secs(1)).await;
            }
        }
    }

    let mut last_time = Instant::now();
    let mut prev_w = 0.0;

    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(1));
    loop {
        ticker.next().await;

        if let Ok(w) = imu.read_z_gyro().await {
            let now = Instant::now();
            let dt = (now - last_time).as_micros() as f32 * 1e-6;
            last_time = now;

            POSE.sender().send_modify(|pose| {
                if let Some(pose) = pose {
                    pose.theta += f32::midpoint(w, prev_w) * dt;
                    if pose.theta > PI {
                        pose.theta -= TAU;
                    } else if pose.theta < -PI {
                        pose.theta += TAU;
                    }
                }
            });
            prev_w = w;
        }
    }
}
