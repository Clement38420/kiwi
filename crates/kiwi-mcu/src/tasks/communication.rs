use crate::channels::{
    ARM_CMD, UART_RX, UART_TX, WHEEL_SPEED_SETPOINTS, get_arm_state, get_battery_level,
    get_battery_voltage,
};
use shared::messages::{McuToRpi, RobotState, RpiToMcu};
use shared::rpc::{RpcRequest, RpcResponse};

#[embassy_executor::task]
pub async fn message_dispatcher() {
    loop {
        let message = UART_RX.receive().await;
        match message {
            RpiToMcu::WheelSpeedSetpoints(setpoints) => wheel_speed_setpoints_handler(setpoints),
            RpiToMcu::Arm(arm) => arm_handler(arm),
            RpiToMcu::GetStateReq(req) => get_state_handler(req).await,
        }
    }
}

pub fn wheel_speed_setpoints_handler(setpoints: [f32; 3]) {
    WHEEL_SPEED_SETPOINTS.signal(setpoints);
}

pub fn arm_handler(arm: bool) {
    ARM_CMD.signal(arm);
}

pub async fn get_state_handler(req: RpcRequest<()>) {
    UART_TX
        .send(McuToRpi::GetStateRes(RpcResponse::new(
            &req,
            RobotState {
                battery_voltage: get_battery_voltage(),
                battery_level: get_battery_level(),
                arm: get_arm_state(),
            },
        )))
        .await;
}
