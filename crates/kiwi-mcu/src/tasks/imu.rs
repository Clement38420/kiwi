use crate::channels::{THETA, UART_TX};
use crate::{channels, drivers};
use core::f32::consts::{PI, TAU};
use defmt::info;
use embassy_time::{Duration, Instant};
use shared::messages::McuToRpi;

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

            THETA.sender().send_modify(|theta| {
                if let Some(theta) = theta {
                    *theta += f32::midpoint(w, prev_w) * dt;
                    if *theta > PI {
                        *theta -= TAU;
                    } else if *theta < -PI {
                        *theta += TAU;
                    }
                }
            });

            prev_w = w;
        }
    }
}

#[embassy_executor::task]
pub async fn theta_telemetry_task() {
    let mut rx = channels::subscribe_theta();
    let mut ticker = embassy_time::Ticker::every(Duration::from_hz(50));

    loop {
        ticker.next().await;
        if let Some(theta) = rx.try_changed() {
            let _ = UART_TX.try_send(McuToRpi::Theta(theta));
        }
    }
}
