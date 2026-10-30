use channel::Channel;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_sync::{channel, watch};
use shared::messages::BatteryState;
use shared::messages::{McuToRpi, RpiToMcu};

const BATTERY_STATE_RECEIVERS: usize = 2;
static BATTERY_STATE: Watch<ThreadModeRawMutex, BatteryState, BATTERY_STATE_RECEIVERS> =
    Watch::new();
pub fn update_battery_state(new_state: BatteryState) {
    BATTERY_STATE.sender().send_if_modified(|current| {
        if *current != Some(new_state) {
            *current = Some(new_state);
            true
        } else {
            false
        }
    });
}
pub fn subscribe_battery_state()
-> watch::Receiver<'static, ThreadModeRawMutex, BatteryState, BATTERY_STATE_RECEIVERS> {
    defmt::expect!(
        BATTERY_STATE.receiver(),
        "CHANNELS: No more battery state receiver available."
    )
}

pub static UART_RX: Channel<ThreadModeRawMutex, RpiToMcu, 4> = Channel::new();

pub static UART_TX: Channel<ThreadModeRawMutex, McuToRpi, 4> = Channel::new();

pub static WHEEL_SPEED_SETPOINTS: Signal<ThreadModeRawMutex, [f32; 3]> = Signal::new();
