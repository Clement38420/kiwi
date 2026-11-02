use channel::Channel;
use core::cell::Cell;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_sync::{channel, watch};
use shared::messages::{ArmState, BatteryLevel};
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
const BATTERY_VOLTAGE_RECEIVERS: usize = 1;
static BATTERY_VOLTAGE: Watch<ThreadModeRawMutex, f32, BATTERY_VOLTAGE_RECEIVERS> =
    Watch::new_with(0.0);
pub fn update_battery_voltage(new_state: f32) {
    update_watch(&BATTERY_VOLTAGE, new_state);
}
pub fn subscribe_battery_voltage()
-> watch::Receiver<'static, ThreadModeRawMutex, f32, BATTERY_VOLTAGE_RECEIVERS> {
    defmt::expect!(
        BATTERY_VOLTAGE.receiver(),
        "CHANNELS: No more battery voltage receiver available."
    )
}
#[allow(clippy::missing_panics_doc)]
pub fn get_battery_voltage() -> f32 {
    BATTERY_VOLTAGE.try_get().unwrap() // BATTERY_VOLTAGE is initialized with a value, so it is safe to unwrap
}

const BATTERY_LEVEL_RECEIVERS: usize = 2;
static BATTERY_LEVEL: Watch<ThreadModeRawMutex, BatteryLevel, BATTERY_LEVEL_RECEIVERS> =
    Watch::new_with(BatteryLevel::High);
pub async fn update_battery_level(new_state: BatteryLevel) {
    if update_watch(&BATTERY_LEVEL, new_state) {
        UART_TX.send(McuToRpi::BatteryLevel(new_state)).await;
    }
}
pub fn subscribe_battery_level()
-> watch::Receiver<'static, ThreadModeRawMutex, BatteryLevel, BATTERY_LEVEL_RECEIVERS> {
    defmt::expect!(
        BATTERY_LEVEL.receiver(),
        "CHANNELS: No more battery level receiver available."
    )
}
#[allow(clippy::missing_panics_doc)]
pub fn get_battery_level() -> BatteryLevel {
    BATTERY_LEVEL.try_get().unwrap() // BATTERY_LEVEL is initialized with a value, so it is safe to unwrap
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
