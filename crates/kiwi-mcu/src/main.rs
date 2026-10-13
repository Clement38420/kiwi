#![no_std]
#![no_main]

mod battery_management;
mod board;
mod drivers;
pub mod pid;
mod safety;

use crate::battery_management::{battery_led_task, battery_monitoring_task};
use crate::safety::safety_task;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Salut moi c'est la mcu de Kiwi!");

    let mut b = board::Board::init(p);

    for motor in b.wheel_motors.iter_mut() {
        motor.set_pwm_speed(0.0)
    }

    let adc1 = board::ADC1.get_or_init(|| Mutex::new(b.adc1));

    spawner.spawn(battery_monitoring_task(adc1, b.voltage_sense).unwrap());
    spawner.spawn(battery_led_task(b.voltage_led_red, b.voltage_led_green).unwrap());
    spawner.spawn(safety_task(b.pwr_cmd).unwrap());

    loop {
        Timer::after(embassy_time::Duration::from_millis(1000)).await;
    }
}
