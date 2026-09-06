// ── Pin mapping ──
// PA6 ─ TIM3_CH1 ─ PWM moteur roue 1
// PA7 ─ TIM3_CH2 ─ PWM moteur roue 2
// PB0 ─ TIM3_CH3 ─ PWM moteur roue 3
// Fréquence PWM : 20 kHz, edge-aligned up

use embassy_stm32::Peri;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA6, PA7, PB0, TIM3};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannel};

pub type WheelMotor = SimplePwmChannel<'static, TIM3>;

pub struct WheelsMotors {
    pub m1: WheelMotor,
    pub m2: WheelMotor,
    pub m3: WheelMotor,
}

pub fn init_wheels_motors(
    tim3: Peri<'static, TIM3>,
    pa6: Peri<'static, PA6>,
    pa7: Peri<'static, PA7>,
    pb0: Peri<'static, PB0>,
) -> WheelsMotors {
    let pwm = SimplePwm::new(
        tim3,
        Some(PwmPin::new(pa6, OutputType::PushPull)),
        Some(PwmPin::new(pa7, OutputType::PushPull)),
        Some(PwmPin::new(pb0, OutputType::PushPull)),
        None,
        Hertz(20_000),
        CountingMode::EdgeAlignedUp,
    );
    let ch = pwm.split();
    WheelsMotors {
        m1: ch.ch1,
        m2: ch.ch2,
        m3: ch.ch3,
    }
}
