use crate::channels;
use crate::channels::UART_TX;
use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use shared::messages;
use shared::messages::BatteryState;

const BATTERY_VOLTAGE_RATIO: f32 = 0.0365; // (3.3V / 1024 * 11.3)

struct StateThresholds {
    exit_down: Option<(u16, BatteryState)>,
    exit_up: Option<(u16, BatteryState)>,
}
fn threshold(state: BatteryState) -> StateThresholds {
    match state {
        BatteryState::High => StateThresholds {
            exit_up: None,
            exit_down: Some((533, BatteryState::Medium)), // = 19.2V
        },
        BatteryState::Medium => StateThresholds {
            exit_up: Some((541, BatteryState::High)),  // = 19.5V
            exit_down: Some((516, BatteryState::Low)), // = 18.6V
        },
        BatteryState::Low => StateThresholds {
            exit_up: Some((524, BatteryState::Medium)),     // = 18.9V
            exit_down: Some((483, BatteryState::Critical)), // = 17.4V
        },
        BatteryState::Critical => StateThresholds {
            exit_up: Some((494, BatteryState::Low)), // = 17.8V
            exit_down: None,
        },
    }
}

fn get_battery_state(current_state: BatteryState, raw_adc: u16) -> BatteryState {
    if let Some((threshold, next)) = threshold(current_state).exit_down
        && raw_adc < threshold
    {
        return next;
    }

    if let Some((threshold, next)) = threshold(current_state).exit_up
        && raw_adc > threshold
    {
        return next;
    }

    current_state
}

#[embassy_executor::task]
pub async fn battery_monitoring_task(
    adc: &'static Mutex<ThreadModeRawMutex, Adc<'static, ADC1>>,
    mut voltage_sense_pin: AnyAdcChannel<'static, ADC1>,
) {
    let mut battery_state = get_battery_state(BatteryState::High, 1000);

    channels::update_battery_state(battery_state);

    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(1000));
    loop {
        const SAMPLES: u32 = 32;
        let mut sum = 0u32;
        {
            let mut adc_locked = adc.lock().await;
            for _ in 0..SAMPLES {
                sum += u32::from(
                    adc_locked.blocking_read(&mut voltage_sense_pin, SampleTime::CYCLES144),
                );
            }
        }
        let battery_raw_voltage = u16::try_from(sum / SAMPLES).unwrap_or(0);
        let battery_voltage = f32::from(battery_raw_voltage) * BATTERY_VOLTAGE_RATIO;

        battery_state = get_battery_state(battery_state, battery_raw_voltage);
        channels::update_battery_state(battery_state);

        UART_TX
            .send(messages::McuToRpi::BatteryVoltage(battery_voltage))
            .await;
        UART_TX
            .send(messages::McuToRpi::BatteryState(battery_state))
            .await;

        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn battery_led_task(
    mut led_red_pin: Output<'static>,
    mut led_green_pin: Output<'static>,
) {
    let mut receiver = channels::subscribe_battery_state();
    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(500));
    loop {
        ticker.next().await;
        let battery_state = receiver.get().await;

        match battery_state {
            BatteryState::High => {
                led_green_pin.set_low();
                led_red_pin.set_high();
            }
            BatteryState::Medium => {
                led_green_pin.set_low();
                led_red_pin.set_low();
            }
            BatteryState::Low => {
                led_green_pin.set_high();
                led_red_pin.set_low();
            }
            BatteryState::Critical => {
                led_green_pin.set_high();
                led_red_pin.toggle();
            }
        }
    }
}
