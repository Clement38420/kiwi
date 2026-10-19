use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::watch::Watch;

pub static BATTERY_STATUS: Watch<ThreadModeRawMutex, BatteryStatus, 2> = Watch::new();

#[derive(PartialEq, Copy, Clone)]
pub enum BatteryStatus {
    High,
    Medium,
    Low,
    Critical,
}

impl BatteryStatus {
    fn threshold(&self) -> StatusThresholds {
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

pub fn get_battery_status(current_status: &BatteryStatus, raw_adc: u16) -> BatteryStatus {
    if let Some((threshold, next)) = current_status.threshold().exit_down {
        if raw_adc < threshold {
            return next;
        }
    }

    if let Some((threshold, next)) = current_status.threshold().exit_up {
        if raw_adc > threshold {
            return next;
        }
    }

    *current_status
}

#[embassy_executor::task]
pub async fn battery_monitoring_task(
    adc: &'static Mutex<ThreadModeRawMutex, Adc<'static, ADC1>>,
    mut voltage_sense_pin: AnyAdcChannel<'static, ADC1>,
) {
    let mut battery_status = get_battery_status(&BatteryStatus::High, 1000);
    let battery_sender = BATTERY_STATUS.sender();

    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(1000));
    loop {
        battery_status = get_battery_status(
            &battery_status,
            adc.lock()
                .await
                .blocking_read(&mut voltage_sense_pin, SampleTime::CYCLES144),
        );

        battery_sender.send(battery_status);

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
