use crate::channels::{POSE, RESET, RESET_DONE};
use crate::drivers;
use core::f32::consts::{PI, TAU};
use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Instant};
use shared::geometry::Pose2D;

const GYRO_DEADBAND: f32 = 0.2f32.to_radians();

async fn calibrate_imu(imu: &mut drivers::mpu6050::Mpu6050<'static>) {
    info!("Starting IMU calibration...");
    loop {
        match imu.calibrate(None).await {
            Ok(()) => {
                info!("IMU calibrated successfully");
                break;
            }
            Err(e) => {
                info!(
                    "Error calibrating IMU: {:?}, retrying in 1 second...",
                    defmt::Debug2Format(&e)
                );
                embassy_time::Timer::after(Duration::from_secs(1)).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn imu_task(mut imu: drivers::mpu6050::Mpu6050<'static>) {
    calibrate_imu(&mut imu).await;
    RESET.reset();

    let mut last_time = Instant::now();
    let mut prev_w = 0.0;

    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(1));
    loop {
        match select(ticker.next(), RESET.wait()).await {
            Either::First(()) => {
                if let Ok(w) = imu.read_z_gyro().await {
                    let w = if w.abs() < GYRO_DEADBAND { 0.0 } else { w };
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
            Either::Second(()) => {
                calibrate_imu(&mut imu).await;
                RESET.reset();

                POSE.sender().send(Pose2D::default());
                prev_w = 0.0;
                last_time = Instant::now();
                ticker.reset();
                RESET_DONE.signal(());
            }
        }
    }
}
