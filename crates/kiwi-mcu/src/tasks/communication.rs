use crate::channels::{
    ARM_CMD, POSE, RESET, RESET_DONE, UART_RX, UART_TX, WHEEL_SPEED_SETPOINTS, get_full_state,
};
use embassy_futures::select::{Either, select};
use shared::messages::{McuToRpi, RpiToMcu};

#[embassy_executor::task]
pub async fn message_dispatcher_task() {
    send_sync().await;

    loop {
        match select(UART_RX.receive(), RESET_DONE.wait()).await {
            Either::First(message) => match message {
                RpiToMcu::WheelSpeedSetpoints(setpoints) => WHEEL_SPEED_SETPOINTS.signal(setpoints),
                RpiToMcu::Arm(arm) => ARM_CMD.signal(arm),
                RpiToMcu::Sync => send_sync().await,
                RpiToMcu::Reset => {
                    RESET_DONE.reset();
                    RESET.signal(());
                }
            },
            Either::Second(()) => {
                UART_TX.send(McuToRpi::ResetAck).await;
            }
        }
    }
}

async fn send_sync() {
    UART_TX.send(McuToRpi::McuState(get_full_state())).await;
    UART_TX
        .send(McuToRpi::Odom(POSE.try_get().unwrap_or_default()))
        .await;
}
