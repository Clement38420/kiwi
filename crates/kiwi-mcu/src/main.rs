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
    let p = embassy_stm32::init(Default::default());
    info!("Salut moi c'est la mcu de Kiwi!");

    let b = match board::Board::init(p) {
        Ok(board) => board,
        Err(e) => match e {
            board::BoardInitError::Uart(e) => {
                panic!(
                    "MAIN: Failed to initialize UART: {:?}",
                    defmt::Debug2Format(&e)
                );
            }
        },
    };

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
    spawner.spawn(tasks::communication::message_dispatcher().unwrap());

    loop {
        Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}
