#![no_std]
#![no_main]

mod battery_monitoring;
mod board;
mod safety;

use crate::battery_monitoring::{battery_led_task, battery_monitoring_task};
use crate::safety::safety_task;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::adc::AdcChannel;
use embassy_stm32::gpio::{Level, OutputOpenDrain, Pull, Speed};
use embassy_stm32::timer::Channel;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Salut moi c'est la mcu de Kiwi!");

    let mut wheels = board::init_wheels_motors(p.TIM3, p.PA6, p.PA7, p.PB0);

    wheels.m1.enable();
    wheels.m2.enable();
    wheels.m3.enable();
    spawner.spawn(battery_monitoring_task(p.ADC1, p.PC0.degrade_adc()).unwrap());
    spawner.spawn(battery_led_task(p.PG6, p.PG7).unwrap());
    spawner.spawn(safety_task(p.PG8).unwrap());

    loop {
        set_wheels_duty_cycle(&mut wheels, 100);
        Timer::after_millis(2000).await;
        set_wheels_duty_cycle(&mut wheels, 0);
        Timer::after_millis(1000).await;
    }
}

fn set_wheels_duty_cycle(wheels: &mut board::WheelsMotors, duty_cycle: u8) {
    wheels.m1.set_duty_cycle_percent(0);
    wheels.m2.set_duty_cycle_percent(duty_cycle);
    wheels.m3.set_duty_cycle_percent(duty_cycle);
}
