use crate::channels;
use crate::channels::UART_TX;
use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use shared::messages;
use shared::messages::BatteryStatus;

const BATTERY_VOLTAGE_RATIO: f32 = 0.0365; // (3.3V / 1024 * 11.3)

struct StatusThresholds {
    exit_down: Option<(u16, BatteryStatus)>,
    exit_up: Option<(u16, BatteryStatus)>,
}
fn threshold(status: BatteryStatus) -> StatusThresholds {
    match status {
        BatteryStatus::High => StatusThresholds {
            exit_up: None,
            exit_down: Some((533, BatteryStatus::Medium)), // = 19.2V
        },
        BatteryStatus::Medium => StatusThresholds {
            exit_up: Some((541, BatteryStatus::High)),  // = 19.5V
            exit_down: Some((516, BatteryStatus::Low)), // = 18.6V
        },
        BatteryStatus::Low => StatusThresholds {
            exit_up: Some((524, BatteryStatus::Medium)), // = 18.9V
            exit_down: Some((483, BatteryStatus::Critical)), // = 17.4V
        },
        BatteryStatus::Critical => StatusThresholds {
            exit_up: Some((494, BatteryStatus::Low)), // = 17.8V
            exit_down: None,
        },
    }
}

pub fn get_battery_status(current_status: BatteryStatus, raw_adc: u16) -> BatteryStatus {
    if let Some((threshold, next)) = threshold(current_status).exit_down
        && raw_adc < threshold
    {
        return next;
    }

    if let Some((threshold, next)) = threshold(current_status).exit_up
        && raw_adc > threshold
    {
        return next;
    }

    current_status
}

#[embassy_executor::task]
pub async fn battery_monitoring_task(
    adc: &'static Mutex<ThreadModeRawMutex, Adc<'static, ADC1>>,
    mut voltage_sense_pin: AnyAdcChannel<'static, ADC1>,
) {
    let mut battery_status = get_battery_status(BatteryStatus::High, 1000);

    channels::update_battery_status(battery_status);

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

        battery_status = get_battery_status(battery_status, battery_raw_voltage);
        channels::update_battery_status(battery_status);

        UART_TX
            .send(messages::McuToRpi::BatteryVoltage(battery_voltage))
            .await;
        UART_TX
            .send(messages::McuToRpi::BatteryStatus(battery_status))
            .await;

        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn battery_led_task(
    mut led_red_pin: Output<'static>,
    mut led_green_pin: Output<'static>,
) {
    let mut receiver = channels::subscribe_battery_status();
    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(500));
    loop {
        ticker.next().await;
        let battery_status = receiver.get().await;

        match battery_status {
            BatteryStatus::High => {
                led_green_pin.set_low();
                led_red_pin.set_high();
            }
            BatteryStatus::Medium => {
                led_green_pin.set_low();
                led_red_pin.set_low();
            }
            BatteryStatus::Low => {
                led_green_pin.set_high();
                led_red_pin.set_low();
            }
            BatteryStatus::Critical => {
                led_green_pin.set_high();
                led_red_pin.toggle();
            }
        }
    }
}
