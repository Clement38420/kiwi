use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel;
use shared::messages::RpiToMcu;

pub static MESSAGE_CHANNEL: channel::Channel<ThreadModeRawMutex, RpiToMcu, 4> =
    channel::Channel::new();

#[embassy_executor::task]
pub async fn message_dispatcher() {
    loop {
        let message = MESSAGE_CHANNEL.receive().await;
        match message {
            RpiToMcu::WheelSpeedSetpoints(setpoints) => wheel_speed_setpoints_handler(setpoints),
        }
    }
}

pub fn wheel_speed_setpoints_handler(setpoints: [f32; 3]) {
    crate::tasks::wheels::WHEEL_SPEED_SETPOINTS.signal(setpoints);
}
