use channel::Channel;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_sync::{channel, watch};
use shared::messages::BatteryStatus;
use shared::messages::{McuToRpi, RpiToMcu};

const BATTERY_STATUS_RECEIVERS: usize = 2;
static BATTERY_STATUS: Watch<ThreadModeRawMutex, BatteryStatus, BATTERY_STATUS_RECEIVERS> =
    Watch::new();
pub fn update_battery_status(new_status: BatteryStatus) {
    BATTERY_STATUS.sender().send_if_modified(|current| {
        if *current != Some(new_status) {
            *current = Some(new_status);
            true
        } else {
            false
        }
    });
}
pub fn subscribe_battery_status()
-> watch::Receiver<'static, ThreadModeRawMutex, BatteryStatus, BATTERY_STATUS_RECEIVERS> {
    defmt::expect!(
        BATTERY_STATUS.receiver(),
        "CHANNELS: No more battery status receiver available."
    )
}

pub static UART_RX: Channel<ThreadModeRawMutex, RpiToMcu, 4> = Channel::new();

pub static UART_TX: Channel<ThreadModeRawMutex, McuToRpi, 4> = Channel::new();

pub static WHEEL_SPEED_SETPOINTS: Signal<ThreadModeRawMutex, [f32; 3]> = Signal::new();
