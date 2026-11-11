use crate::channels::{ARM_CMD, THETA, UART_RX, UART_TX, WHEEL_SPEED_SETPOINTS, get_full_state};
use shared::messages::{McuToRpi, RpiToMcu};

#[embassy_executor::task]
pub async fn message_dispatcher_task() {
    send_sync().await;

    loop {
        let message = UART_RX.receive().await;
        match message {
            RpiToMcu::WheelSpeedSetpoints(setpoints) => WHEEL_SPEED_SETPOINTS.signal(setpoints),
            RpiToMcu::Arm(arm) => ARM_CMD.signal(arm),
            RpiToMcu::Sync => send_sync().await,
            RpiToMcu::Reset => {
                THETA.sender().send(0.0);
            }
        }
    }
}

async fn send_sync() {
    UART_TX.send(McuToRpi::McuState(get_full_state())).await;
    UART_TX
        .send(McuToRpi::Theta(THETA.try_get().unwrap_or(0.0)))
        .await;
}
