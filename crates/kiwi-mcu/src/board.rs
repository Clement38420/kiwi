/// ────── Pin mapping ──────
/// ── Wheels Motors ──
/// PA6 ─ TIM3_CH1 ─ Wheel 1 motor PWM
/// PA7 ─ TIM3_CH2 ─ Wheel 2 motor PWM
/// PB0 ─ TIM3_CH3 ─ Wheel 3 motor PWM
/// PWM frequency : 20 kHz, edge-aligned up
///
/// ── Wheels Encoders ──
/// PA0 ─ TIM2_CH1 ─ Encoder wheel 1
/// PB10 ─ TIM2_CH3 ─ Encoder wheel 2
/// PB11 ─ TIM2_CH4 ─ Encoder wheel 3
/// CH2 not used because it does not have a pin on the ST Zio connector (to connect the external pull up)
///
///
use crate::drivers::motors::BLM3650Motor;
use embassy_stm32::adc::{AdcChannel, AdcConfig, AnyAdcChannel, Resolution};
use embassy_stm32::gpio::{Level, Output, OutputOpenDrain, OutputType, Pull, Speed};
use embassy_stm32::time::{Hertz, khz};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannels};
use embassy_stm32::{Peripherals, adc, peripherals};

const BLM3650_PWM_FREQ: Hertz = khz(20);
const ADC1_RESOLUTION: Resolution = Resolution::BITS10;

pub struct Board {
    pub wheel_motors: [BLM3650Motor<'static, peripherals::TIM3>; 3],
    pub pwr_cmd: Output<'static>,
    pub adc1: adc::Adc<'static, peripherals::ADC1>,
    pub voltage_led_red: Output<'static>,
    pub voltage_led_green: Output<'static>,
    pub voltage_sense: AnyAdcChannel<'static, peripherals::ADC1>,
}

impl Board {
    pub fn init(p: Peripherals) -> Board {
        // Wheel motors
        let tim3 = SimplePwm::new(
            p.TIM3,
            Some(PwmPin::new(p.PA6, OutputType::PushPull)),
            Some(PwmPin::new(p.PA7, OutputType::PushPull)),
            Some(PwmPin::new(p.PB0, OutputType::PushPull)),
            None,
            BLM3650_PWM_FREQ,
            CountingMode::EdgeAlignedUp,
        );

        let pg0 = OutputOpenDrain::new_pull(p.PG0, Level::High, Speed::Low, Pull::None);
        let pg1 = OutputOpenDrain::new_pull(p.PG1, Level::High, Speed::Low, Pull::None);
        let pg2 = OutputOpenDrain::new_pull(p.PG2, Level::High, Speed::Low, Pull::None);
        let pg3 = OutputOpenDrain::new_pull(p.PG3, Level::High, Speed::Low, Pull::None);
        let pg4 = OutputOpenDrain::new_pull(p.PG4, Level::High, Speed::Low, Pull::None);
        let pg5 = OutputOpenDrain::new_pull(p.PG5, Level::High, Speed::Low, Pull::None);

        let SimplePwmChannels { ch1, ch2, ch3, .. } = tim3.split();

        let mut wheel_motors = [
            BLM3650Motor::new(ch1, pg0, pg1),
            BLM3650Motor::new(ch2, pg2, pg3),
            BLM3650Motor::new(ch3, pg4, pg5),
        ];

        for motor in wheel_motors.iter_mut() {
            motor.enable();
        }

        // Power command pin
        let pwr_cmd = Output::new(p.PG8, Level::High, Speed::Low);

        // Voltage sense ADC
        let adc1 = adc::Adc::new_with_config(
            p.ADC1,
            AdcConfig {
                resolution: Some(ADC1_RESOLUTION),
            },
        );

        //Voltage LED pins
        let voltage_led_red = Output::new(p.PG6, Level::High, Speed::Low);
        let voltage_led_green = Output::new(p.PG7, Level::High, Speed::Low);

        Board {
            wheel_motors,
            pwr_cmd,
            adc1,
            voltage_led_red,
            voltage_led_green,
            voltage_sense: p.PC0.degrade_adc(),
        }
    }
}
