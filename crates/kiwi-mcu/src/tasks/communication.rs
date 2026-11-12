use crate::channels::{ARM_CMD, POSE, UART_RX, UART_TX, WHEEL_SPEED_SETPOINTS, get_full_state};
use shared::geometry::Pose2D;
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
                POSE.sender().send(Pose2D::default());
                UART_TX.send(McuToRpi::Odom(Pose2D::default())).await;
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
