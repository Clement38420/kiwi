use crate::safety::{SAFETY_CHANNEL, SafetyReason};
use defmt::Format;
use embassy_stm32::Peri;
use embassy_stm32::adc::{Adc, AdcConfig, AnyAdcChannel, Resolution, SampleTime};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::{ADC1, PG6, PG7};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;

pub static BATTERY_STATUS: Watch<ThreadModeRawMutex, BatteryStatus, 1> = Watch::new();

fn get_battery_raw_voltage(adc: &mut Adc<'static, ADC1>, pin: &mut AnyAdcChannel<ADC1>) -> u16 {
    adc.blocking_read(pin, SampleTime::CYCLES144)
}

#[derive(PartialEq, Copy, Clone, Format)]
pub enum BatteryStatus {
    High,
    Medium,
    Low,
    Critical,
}

struct StateThresholds {
    state: BatteryStatus,
    exit_down: Option<(u16, BatteryStatus)>,
    exit_up: Option<(u16, BatteryStatus)>,
}

const THRESHOLDS: [StateThresholds; 4] = [
    StateThresholds {
        state: BatteryStatus::High,
        exit_down: Some((533, BatteryStatus::Medium)),
        exit_up: None,
    },
    StateThresholds {
        state: BatteryStatus::Medium,
        exit_down: Some((516, BatteryStatus::Low)),
        exit_up: Some((541, BatteryStatus::High)),
    },
    StateThresholds {
        state: BatteryStatus::Low,
        exit_down: Some((483, BatteryStatus::Critical)),
        exit_up: Some((524, BatteryStatus::Medium)),
    },
    StateThresholds {
        state: BatteryStatus::Critical,
        exit_down: None,
        exit_up: Some((494, BatteryStatus::Low)),
    },
];

pub fn get_battery_status(current_status: &BatteryStatus, raw_adc: u16) -> BatteryStatus {
    let rule = THRESHOLDS
        .iter()
        .find(|t| t.state == *current_status)
        .unwrap();

    if let Some((threshold, next)) = rule.exit_down {
        if raw_adc < threshold {
            return next;
        }
    }

    if let Some((threshold, next)) = rule.exit_up {
        if raw_adc > threshold {
            return next;
        }
    }

    *current_status
}

#[embassy_executor::task]
pub async fn battery_monitoring_task(
    adc_peri: Peri<'static, ADC1>,
    mut voltage_pin: AnyAdcChannel<'static, ADC1>,
) {
    let mut adc = Adc::new_with_config(
        adc_peri,
        AdcConfig {
            resolution: Some(Resolution::BITS10),
        },
    );

    let mut battery_status = get_battery_status(&BatteryStatus::High, 1000);
    let battery_sender = BATTERY_STATUS.sender();

    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(1000));
    loop {
        battery_status = get_battery_status(
            &battery_status,
            get_battery_raw_voltage(&mut adc, &mut voltage_pin),
        );

        battery_sender.send(battery_status);

        if battery_status == BatteryStatus::Critical {
            SAFETY_CHANNEL.send(SafetyReason::BatteryCritical).await;
        }

        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn battery_led_task(led_red_pin: Peri<'static, PG6>, led_green_pin: Peri<'static, PG7>) {
    let mut led_red_pin = Output::new(led_red_pin, Level::High, Speed::Low);
    let mut led_green_pin = Output::new(led_green_pin, Level::High, Speed::Low);

    let mut receiver = BATTERY_STATUS.receiver().unwrap();

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
