//! Hardware abstraction and pin mapping for the board.
//!
//! # Pin Mapping
//!
//! ## Wheel Motors (`TIM3` & `GPIOG`)
//! PWM configured at **20 kHz**, edge-aligned up, push-pull. Control pins in open-drain (external pull-up).
//!
//! | Pin | Peripheral / Mode | Description |
//! | :--- | :--- | :--- |
//! | `PA6` | `TIM3_CH1` (PWM) | Wheel 1 motor PWM |
//! | `PG0`, `PG1` | `GPIO` (Open-Drain) | Wheel 1 motor control (Enable / Direction) |
//! | `PA7` | `TIM3_CH2` (PWM) | Wheel 2 motor PWM |
//! | `PG2`, `PG3` | `GPIO` (Open-Drain) | Wheel 2 motor control (Enable / Direction) |
//! | `PB0` | `TIM3_CH3` (PWM) | Wheel 3 motor PWM |
//! | `PG4`, `PG5` | `GPIO` (Open-Drain) | Wheel 3 motor control (Enable / Direction) |
//!
//! ## Wheel Encoders (`TIM2`)
//! Input capture on falling edge (`Pull::None`, using external pull-ups).
//!
//! | Pin | Peripheral / Mode | Description |
//! | :--- | :--- | :--- |
//! | `PA0` | `TIM2_CH1` | Wheel 1 encoder |
//! | `PB10` | `TIM2_CH3` | Wheel 2 encoder |
//! | `PB11` | `TIM2_CH4` | Wheel 3 encoder |
//!
//! > **Note:** `TIM2_CH2` is not used because it is not routed to the ST Zio connector (required to wire the external pull-up).
//!
//! ## Power & Voltage Monitoring (`ADC1` & `GPIOG`)
//!
//! | Pin | Peripheral / Mode | Description |
//! | :--- | :--- | :--- |
//! | `PG8` | `GPIO` (Push-Pull) | Power command (`pwr_cmd`) |
//! | `PC0` | `ADC1` (10-bit) | Voltage sense (`voltage_sense`) |
//! | `PG6` | `GPIO` (Push-Pull) | Voltage status LED — Red |
//! | `PG7` | `GPIO` (Push-Pull) | Voltage status LED — Green |

use crate::drivers;
use embassy_stm32::adc::{AdcChannel, AdcConfig, AnyAdcChannel, Resolution};
use embassy_stm32::gpio::{Level, Output, OutputType, Pull, Speed};
use embassy_stm32::interrupt::typelevel::Handler;
use embassy_stm32::time::{Hertz, mhz};
use embassy_stm32::timer::input_capture::CapturePin;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannels};
use embassy_stm32::timer::{Ch1, Ch3, Ch4};
use embassy_stm32::{Peripherals, adc, bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::once_lock::OnceLock;

static WHEEL_ENCODERS_TIM: OnceLock<drivers::one_channel_encoder::Timer<peripherals::TIM2>> =
    OnceLock::new();
const WHEEL_ENCODER_TIM_TICK: Hertz = mhz(1);
const WHEEL_ENCODER_FILTER: embassy_stm32::timer::low_level::FilterValue =
    embassy_stm32::timer::low_level::FilterValue::FCK_INT_N8; // Timer clock is 16 MHz, so the filter cut-off is 500ns
const WHEEL_ENCODER_MAX_PERIOD: u32 = 15_000;

static ADC1: OnceLock<Mutex<ThreadModeRawMutex, adc::Adc<'static, peripherals::ADC1>>> =
    OnceLock::new();
const ADC1_RESOLUTION: Resolution = Resolution::BITS10;

struct WheelEncodersIRQ;
impl Handler<embassy_stm32::interrupt::typelevel::TIM2> for WheelEncodersIRQ {
    unsafe fn on_interrupt() {
        if let Some(tim) = WHEEL_ENCODERS_TIM.try_get() {
            tim.on_interrupt();
        }
    }
}

pub type WheelMotors = [drivers::blm3650::Motor<'static, peripherals::TIM3>; 3];

pub type WheelEncoders = [drivers::one_channel_encoder::Encoder<'static, peripherals::TIM2>; 3];

pub struct Board {
    pub wheel_motors: WheelMotors,
    pub wheel_encoders: WheelEncoders,
    pub tim2: &'static drivers::one_channel_encoder::Timer<peripherals::TIM2>,
    pub pwr_cmd: Output<'static>,
    pub adc1: &'static Mutex<ThreadModeRawMutex, adc::Adc<'static, peripherals::ADC1>>,
    pub voltage_led_red: Output<'static>,
    pub voltage_led_green: Output<'static>,
    pub voltage_sense: AnyAdcChannel<'static, peripherals::ADC1>,
}

impl Board {
    pub fn init(p: Peripherals) -> Board {
        bind_interrupts!(
            struct Irqs {
                TIM2 => WheelEncodersIRQ;
            }
        );

        let tim3 = SimplePwm::new(
            p.TIM3,
            Some(PwmPin::new(p.PA6, OutputType::PushPull)),
            Some(PwmPin::new(p.PA7, OutputType::PushPull)),
            Some(PwmPin::new(p.PB0, OutputType::PushPull)),
            None,
            drivers::blm3650::PWM_FREQ,
            CountingMode::EdgeAlignedUp,
        );

        let SimplePwmChannels { ch1, ch2, ch3, .. } = tim3.split();

        let mut wheel_motors = [
            drivers::blm3650::Motor::new(ch1, p.PG0, p.PG1),
            drivers::blm3650::Motor::new(ch2, p.PG2, p.PG3),
            drivers::blm3650::Motor::new(ch3, p.PG4, p.PG5),
        ];

        for motor in wheel_motors.iter_mut() {
            motor.enable();
        }

        let tim2 = drivers::one_channel_encoder::Timer::new(p.TIM2, Irqs, WHEEL_ENCODER_TIM_TICK);
        let tim2 = WHEEL_ENCODERS_TIM.get_or_init(|| tim2);

        let wheel_encoders = [
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, Ch1>::new(p.PA0, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, Ch3>::new(p.PB10, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, Ch4>::new(p.PB11, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
        ];

        tim2.start();

        // Power command is ON by default
        let pwr_cmd = Output::new(p.PG8, Level::High, Speed::Low);

        let adc1 = ADC1.get_or_init(|| {
            Mutex::new(adc::Adc::new_with_config(
                p.ADC1,
                AdcConfig {
                    resolution: Some(ADC1_RESOLUTION),
                },
            ))
        });

        // The LEDs are active low, so we set them to high (off) by default
        let voltage_led_red = Output::new(p.PG6, Level::High, Speed::Low);
        let voltage_led_green = Output::new(p.PG7, Level::High, Speed::Low);

        Board {
            wheel_motors,
            wheel_encoders,
            tim2,
            pwr_cmd,
            adc1,
            voltage_led_red,
            voltage_led_green,
            voltage_sense: p.PC0.degrade_adc(),
        }
    }
}
