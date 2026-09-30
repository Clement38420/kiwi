use crate::battery_monitoring::{BATTERY_STATUS, BatteryStatus};
use defmt::error;
use embassy_futures::select::select_array;
use embassy_stm32::Peri;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::PG8;

pub enum SafetyReason {
    BatteryCriticalVoltage,
}

async fn battery_safety_check() -> SafetyReason {
    let mut receiver = BATTERY_STATUS.receiver().unwrap();

    loop {
        if receiver.changed().await == BatteryStatus::Critical {
            return SafetyReason::BatteryCriticalVoltage;
        }
    }
}

#[embassy_executor::task]
pub async fn safety_task(power_cmd_pin: Peri<'static, PG8>) {
    let mut power_cmd = Output::new(power_cmd_pin, Level::High, Speed::Low);

    loop {
        let check_functions = [battery_safety_check()];
        let (reason, _index) = select_array(check_functions).await;

        match reason {
            SafetyReason::BatteryCriticalVoltage => {
                // Cut power to the system
                power_cmd.set_low();
                error!("Safety task: Battery critical! Cutting power to the system.");
            }
        }
    }
}
