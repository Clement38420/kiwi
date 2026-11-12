use crate::channels;
use crate::channels::UART_TX;
use embassy_time::Duration;
use shared::messages::McuToRpi;

#[embassy_executor::task]
pub async fn send_pose_task() {
    let mut rx = channels::subscribe_pose();
    let mut ticker = embassy_time::Ticker::every(Duration::from_hz(50));
    loop {
        ticker.next().await;
        if let Some(pose) = rx.try_changed() {
            let _ = UART_TX.try_send(McuToRpi::Odom(pose));
        }
    }
}
