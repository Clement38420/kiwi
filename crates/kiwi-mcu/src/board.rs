use crate::drivers;
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
use core::marker::PhantomData;
use defmt::error;
use embassy_stm32::adc::{AdcChannel, AdcConfig, AnyAdcChannel, Resolution};
use embassy_stm32::gpio::{Level, Output, OutputOpenDrain, OutputType, Pull, Speed};
use embassy_stm32::interrupt::typelevel::Handler;
use embassy_stm32::time::{Hertz, khz};
use embassy_stm32::timer::input_capture::CapturePin;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannels};
use embassy_stm32::timer::{Ch1, Ch3, Ch4, GeneralInstance4Channel};
use embassy_stm32::{Peripherals, adc, bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::once_lock::OnceLock;

const BLM3650_PWM_FREQ: Hertz = khz(20);
const ADC1_RESOLUTION: Resolution = Resolution::BITS10;

pub static WHEEL_ENCODERS_TIM: OnceLock<
    drivers::encoders::OneChannelEncodersTimer<peripherals::TIM2>,
> = OnceLock::new();

pub static ADC1: OnceLock<Mutex<ThreadModeRawMutex, adc::Adc<'static, peripherals::ADC1>>> =
    OnceLock::new();

pub struct WheelEncodersIRQ<T: GeneralInstance4Channel> {
    _phantom: PhantomData<T>,
}

impl<T: GeneralInstance4Channel> Handler<T::CaptureCompareInterrupt> for WheelEncodersIRQ<T> {
    unsafe fn on_interrupt() {
        if let Some(tim) = WHEEL_ENCODERS_TIM.try_get() {
            let sr = embassy_stm32::pac::TIM2.sr().read();

            if sr.ccif(0) {
                tim.update_channel_period::<Ch1>(embassy_stm32::pac::TIM2.ccr(0).read());
            }
            if sr.ccif(2) {
                tim.update_channel_period::<Ch3>(embassy_stm32::pac::TIM2.ccr(2).read());
            }
            if sr.ccif(3) {
                tim.update_channel_period::<Ch4>(embassy_stm32::pac::TIM2.ccr(3).read());
            }
        } else {
            error!("WheelEncodersIRQ: TIM2 not initialized");
        }
    }
}

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
        bind_interrupts!(
            struct Irqs {
                TIM2 => WheelEncodersIRQ<peripherals::TIM2>;
            }
        );

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

        //Wheel encoders
        let tim2 = embassy_stm32::timer::low_level::Timer::new(p.TIM2);
        let mut tim2 = drivers::encoders::OneChannelEncodersTimer::new(tim2);

        tim2.add_channel(
            CapturePin::<peripherals::TIM2, Ch1>::new(p.PA0, Pull::Up),
            3,
        );
        tim2.add_channel(
            CapturePin::<peripherals::TIM2, Ch3>::new(p.PB10, Pull::Up),
            3,
        );
        tim2.add_channel(
            CapturePin::<peripherals::TIM2, Ch4>::new(p.PB11, Pull::Up),
            3,
        );

        WHEEL_ENCODERS_TIM.get_or_init(|| tim2);

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
