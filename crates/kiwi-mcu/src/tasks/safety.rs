use crate::channels;
use defmt::error;
use embassy_futures::select::select_array;
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Receiver;
use shared::messages::BatteryStatus;

pub enum SafetyReason {
    BatteryCriticalVoltage,
}

async fn battery_safety_check(
    receiver: &mut Receiver<'static, ThreadModeRawMutex, BatteryStatus, 2>,
) -> SafetyReason {
    loop {
        if receiver.changed().await == BatteryStatus::Critical {
            return SafetyReason::BatteryCriticalVoltage;
        }
    }
}

#[embassy_executor::task]
pub async fn safety_task(mut pwr_cmd: Output<'static>) {
    let mut receiver = channels::subscribe_battery_status();

    loop {
        let check_functions = [battery_safety_check(&mut receiver)];
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
