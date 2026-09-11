#![no_std]
#![no_main]

mod board;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Salut moi c'est la mcu de Kiwi!");

    let mut wheels = board::init_wheels_motors(p.TIM3, p.PA6, p.PA7, p.PB0);

    wheels.m1.enable();
    wheels.m2.enable();
    wheels.m3.enable();

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
