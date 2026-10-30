use crate::channels::{UART_RX, WHEEL_SPEED_SETPOINTS};
use shared::messages::RpiToMcu;

#[embassy_executor::task]
pub async fn message_dispatcher() {
    loop {
        let message = UART_RX.receive().await;
        match message {
            RpiToMcu::WheelSpeedSetpoints(setpoints) => wheel_speed_setpoints_handler(setpoints),
        }
    }
}

pub fn wheel_speed_setpoints_handler(setpoints: [f32; 3]) {
    WHEEL_SPEED_SETPOINTS.signal(setpoints);
}
