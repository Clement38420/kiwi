use super::battery_management::{BATTERY_STATUS, BatteryStatus};
use defmt::error;
use embassy_futures::select::select_array;
use embassy_stm32::gpio::Output;

pub enum SafetyReason {
    BatteryCriticalVoltage,
}

async fn battery_safety_check() -> SafetyReason {
    let mut receiver = defmt::expect!(
        BATTERY_STATUS.receiver(),
        "SAFETY TASK: No more battery status receiver available."
    );

    loop {
        if receiver.changed().await == BatteryStatus::Critical {
            return SafetyReason::BatteryCriticalVoltage;
        }
    }
}

#[embassy_executor::task]
pub async fn safety_task(mut pwr_cmd: Output<'static>) {
    loop {
        let check_functions = [battery_safety_check()];
        let (reason, _index) = select_array(check_functions).await;

        match reason {
            SafetyReason::BatteryCriticalVoltage => {
                // Cut power to the system
                pwr_cmd.set_low();
                error!("SAFETY TASK: Battery critical! Cutting power to the system.");
            }
        }
    }
}
