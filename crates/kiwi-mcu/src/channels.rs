use channel::Channel;
use core::cell::Cell;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_sync::{channel, watch};
use shared::messages::{ArmState, BatteryState};
use shared::messages::{McuToRpi, RpiToMcu};

fn update_watch<T: PartialEq + Copy, const N: usize>(
    watch: &Watch<ThreadModeRawMutex, T, N>,
    new_state: T,
) -> bool {
    let changed = Cell::new(false);
    watch.sender().send_if_modified(|current| {
        if *current == Some(new_state) {
            false
        } else {
            *current = Some(new_state);
            changed.set(true);
            true
        }
    });
    changed.get()
}

const BATTERY_STATE_RECEIVERS: usize = 2;
static BATTERY_STATE: Watch<ThreadModeRawMutex, BatteryState, BATTERY_STATE_RECEIVERS> =
    Watch::new();
pub fn update_battery_state(new_state: BatteryState) {
    update_watch(&BATTERY_STATE, new_state);
}
pub fn subscribe_battery_state()
-> watch::Receiver<'static, ThreadModeRawMutex, BatteryState, BATTERY_STATE_RECEIVERS> {
    defmt::expect!(
        BATTERY_STATE.receiver(),
        "CHANNELS: No more battery state receiver available."
    )
}

pub static ARM_CMD: Signal<ThreadModeRawMutex, bool> = Signal::new();
const ARM_STATE_RECEIVERS: usize = 1;
static ARM_STATE: Watch<ThreadModeRawMutex, ArmState, ARM_STATE_RECEIVERS> =
    Watch::new_with(ArmState::Disarmed);
pub async fn update_arm_state(new_state: ArmState) {
    if update_watch(&ARM_STATE, new_state) {
        UART_TX.send(McuToRpi::ArmState(new_state)).await;
    }
}
pub fn subscribe_arm_state()
-> watch::Receiver<'static, ThreadModeRawMutex, ArmState, ARM_STATE_RECEIVERS> {
    defmt::expect!(
        ARM_STATE.receiver(),
        "CHANNELS: No more arm state receiver available."
    )
}
#[allow(clippy::missing_panics_doc)]
pub fn get_arm_state() -> ArmState {
    ARM_STATE.try_get().unwrap() // ARM_STATE is initialized with a value, so it is safe to unwrap
}

pub static UART_RX: Channel<ThreadModeRawMutex, RpiToMcu, 4> = Channel::new();

pub static UART_TX: Channel<ThreadModeRawMutex, McuToRpi, 4> = Channel::new();

pub static WHEEL_SPEED_SETPOINTS: Signal<ThreadModeRawMutex, [f32; 3]> = Signal::new();
