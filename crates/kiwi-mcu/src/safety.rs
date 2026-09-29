use defmt::error;
use embassy_stm32::Peri;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::PG8;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

pub static SAFETY_CHANNEL: Channel<ThreadModeRawMutex, SafetyReason, 1> = Channel::new();

pub enum SafetyReason {
    BatteryCritical,
}

#[embassy_executor::task]
pub async fn safety_task(power_cmd_pin: Peri<'static, PG8>) {
    let mut power_cmd = Output::new(power_cmd_pin, Level::High, Speed::Low);

    loop {
        let reason = SAFETY_CHANNEL.receiver().receive().await;
        match reason {
            SafetyReason::BatteryCritical => {
                // Cut power to the system
                power_cmd.set_low();
                error!("Safety task: Battery critical! Cutting power to the system.");
            }
        }
    }
}
