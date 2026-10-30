use crate::channels;
use crate::channels::{ARM_CMD, get_arm_state, update_arm_state};
use defmt::{error, warn};
use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use shared::messages::{ArmState, BatteryState};

async fn arm(pwr_cmd: &mut Output<'static>) {
    pwr_cmd.set_high();
    update_arm_state(ArmState::Armed).await;
}

async fn disarm(pwr_cmd: &mut Output<'static>) {
    pwr_cmd.set_low();
    update_arm_state(ArmState::Disarmed).await;
}

#[embassy_executor::task]
pub async fn safety_task(mut pwr_cmd: Output<'static>) {
    let mut battery_rx = channels::subscribe_battery_state();

    loop {
        match select(battery_rx.changed(), ARM_CMD.wait()).await {
            Either::First(state) => {
                if state == BatteryState::Critical {
                    pwr_cmd.set_low();
                    update_arm_state(ArmState::CannotArm).await;
                    error!("SAFETY TASK: Battery critical! Cutting power to the system.");
                } else {
                    update_arm_state(ArmState::Disarmed).await;
                }
            }

            Either::Second(arm_cmd) => {
                if arm_cmd {
                    if get_arm_state() == ArmState::CannotArm {
                        warn!("SAFETY TASK: Cannot arm robot while battery is critical!");
                    } else {
                        arm(&mut pwr_cmd).await;
                    }
                } else {
                    disarm(&mut pwr_cmd).await;
                }
            }
        }
    }
}
