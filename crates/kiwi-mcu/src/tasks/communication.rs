use crate::channels::{ARM_CMD, UART_RX, UART_TX, WHEEL_SPEED_SETPOINTS, get_full_state};
use defmt::info;
use shared::messages::{McuToRpi, RpiToMcu};

#[embassy_executor::task]
pub async fn message_dispatcher() {
    loop {
        let message = UART_RX.receive().await;
        match message {
            RpiToMcu::WheelSpeedSetpoints(setpoints) => WHEEL_SPEED_SETPOINTS.signal(setpoints),
            RpiToMcu::Arm(arm) => ARM_CMD.signal(arm),
            RpiToMcu::SyncState => {
                info!("Sync state");
                UART_TX.send(McuToRpi::McuState(get_full_state())).await;
            }
        }
    }
}
