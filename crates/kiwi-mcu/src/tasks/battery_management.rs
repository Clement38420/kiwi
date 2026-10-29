use crate::tasks::uart::TX_CHANNEL;
use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::watch::Watch;
use shared::messages;

const BATTERY_VOLTAGE_RATIO: f32 = 0.036; // (3.3V / 1024 * 11.2)

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BatteryStatus {
    High,
    Medium,
    Low,
    Critical,
}

impl From<BatteryStatus> for messages::BatteryStatus {
    fn from(value: BatteryStatus) -> Self {
        match value {
            BatteryStatus::High => messages::BatteryStatus::High,
            BatteryStatus::Medium => messages::BatteryStatus::Medium,
            BatteryStatus::Low => messages::BatteryStatus::Low,
            BatteryStatus::Critical => messages::BatteryStatus::Critical,
        }
    }
}

pub static BATTERY_STATUS: Watch<ThreadModeRawMutex, BatteryStatus, 2> = Watch::new();

impl BatteryStatus {
    fn threshold(self) -> StatusThresholds {
        match self {
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
}

struct StatusThresholds {
    exit_down: Option<(u16, BatteryStatus)>,
    exit_up: Option<(u16, BatteryStatus)>,
}

pub fn get_battery_status(current_status: BatteryStatus, raw_adc: u16) -> BatteryStatus {
    if let Some((threshold, next)) = current_status.threshold().exit_down
        && raw_adc < threshold
    {
        return next;
    }

    if let Some((threshold, next)) = current_status.threshold().exit_up
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

    let battery_status_sender = BATTERY_STATUS.sender();
    battery_status_sender.send(battery_status);

    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(1000));
    loop {
        const SAMPLES: u32 = 16;
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

        let new_battery_status = get_battery_status(battery_status, battery_raw_voltage);
        if new_battery_status != battery_status {
            battery_status = new_battery_status;
            battery_status_sender.send(battery_status);
        }

        TX_CHANNEL
            .send(messages::McuToRpi::BatteryVoltage(battery_voltage))
            .await;
        TX_CHANNEL
            .send(messages::McuToRpi::BatteryStatus(battery_status.into()))
            .await;

        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn battery_led_task(
    mut led_red_pin: Output<'static>,
    mut led_green_pin: Output<'static>,
) {
    let mut receiver = defmt::expect!(
        BATTERY_STATUS.receiver(),
        "BATTERY LED TASK: No more battery status receiver available."
    );

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
