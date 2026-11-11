#![no_std]
#![no_main]

mod board;
pub mod channels;
mod drivers;
mod tasks;

use defmt::{info, panic};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let b = match board::Board::init().await {
        Ok(board) => board,
        Err(e) => {
            panic!(
                "MAIN: Board initialization error: {:?}",
                defmt::Debug2Format(&e)
            );
        }
    };
    info!("Salut moi c'est la mcu de Kiwi!");

    Timer::after(embassy_time::Duration::from_millis(500)).await; // wait for the peripherals to stabilize

    spawner.spawn(
        tasks::battery_management::battery_monitoring_task(b.adc1, b.voltage_sense).unwrap(),
    );
    spawner.spawn(
        tasks::battery_management::battery_led_task(b.voltage_led_red, b.voltage_led_green)
            .unwrap(),
    );
    spawner.spawn(tasks::safety::safety_task(b.pwr_cmd).unwrap());
    spawner.spawn(tasks::wheels::wheels_control_task(b.wheel_motors, b.wheel_encoders).unwrap());
    spawner.spawn(tasks::uart::uart_task(b.uart1).unwrap());
    spawner.spawn(tasks::communication::message_dispatcher_task().unwrap());
    spawner.spawn(tasks::imu::imu_task(b.imu).unwrap());
    spawner.spawn(tasks::imu::theta_telemetry_task().unwrap());

    loop {
        Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}
