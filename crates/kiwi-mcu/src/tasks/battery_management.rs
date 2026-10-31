use crate::channels;
use crate::channels::UART_TX;
use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use shared::messages;
use shared::messages::BatteryLevel;

const BATTERY_VOLTAGE_RATIO: f32 = 0.0365; // (3.3V / 1024 * 11.3)

struct StateThresholds {
    exit_down: Option<(u16, BatteryLevel)>,
    exit_up: Option<(u16, BatteryLevel)>,
}
fn threshold(state: BatteryLevel) -> StateThresholds {
    match state {
        BatteryLevel::High => StateThresholds {
            exit_up: None,
            exit_down: Some((533, BatteryLevel::Medium)), // = 19.2V
        },
        BatteryLevel::Medium => StateThresholds {
            exit_up: Some((541, BatteryLevel::High)),  // = 19.5V
            exit_down: Some((516, BatteryLevel::Low)), // = 18.6V
        },
        BatteryLevel::Low => StateThresholds {
            exit_up: Some((524, BatteryLevel::Medium)),     // = 18.9V
            exit_down: Some((483, BatteryLevel::Critical)), // = 17.4V
        },
        BatteryLevel::Critical => StateThresholds {
            exit_up: Some((494, BatteryLevel::Low)), // = 17.8V
            exit_down: None,
        },
    }
}

fn compute_battery_level(current_state: BatteryLevel, raw_adc: u16) -> BatteryLevel {
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
    let mut battery_level = compute_battery_level(BatteryLevel::High, 1000);

    channels::update_battery_level(battery_level);

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

        battery_level = compute_battery_level(battery_level, battery_raw_voltage);
        channels::update_battery_level(battery_level);
        channels::update_battery_voltage(battery_voltage);

        UART_TX
            .send(messages::McuToRpi::BatteryVoltage(battery_voltage))
            .await;
        UART_TX
            .send(messages::McuToRpi::BatteryLevel(battery_level))
            .await;

        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn battery_led_task(
    mut led_red_pin: Output<'static>,
    mut led_green_pin: Output<'static>,
) {
    let mut receiver = channels::subscribe_battery_level();
    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(500));
    loop {
        ticker.next().await;
        let battery_level = receiver.get().await;

        match battery_level {
            BatteryLevel::High => {
                led_green_pin.set_low();
                led_red_pin.set_high();
            }
            BatteryLevel::Medium => {
                led_green_pin.set_low();
                led_red_pin.set_low();
            }
            BatteryLevel::Low => {
                led_green_pin.set_high();
                led_red_pin.set_low();
            }
            BatteryLevel::Critical => {
                led_green_pin.set_high();
                led_red_pin.toggle();
            }
        }
    }
}
