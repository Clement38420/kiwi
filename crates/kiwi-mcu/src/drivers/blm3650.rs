use embassy_stm32::Peri;
use embassy_stm32::gpio::{Level, OutputOpenDrain, Pull, Speed};
use embassy_stm32::time::{Hertz, khz};
use embassy_stm32::timer::GeneralInstance4Channel;
use embassy_stm32::timer::simple_pwm::SimplePwmChannel;

pub const PWM_FREQ: Hertz = khz(20);
pub const PWM_MAX: f32 = 1.0;
pub const PWM_MIN: f32 = -1.0;
pub const PWM_DEAD_ZONE: f32 = 0.08;

pub struct Motor<'d, T: GeneralInstance4Channel> {
    pwm_channel: SimplePwmChannel<'static, T>,
    direction_pin: OutputOpenDrain<'d>,
    brake_pin: OutputOpenDrain<'d>,
}

impl<'d, T: GeneralInstance4Channel> Motor<'d, T> {
    pub fn new(
        pwm_channel: SimplePwmChannel<'static, T>,
        direction_pin: Peri<'d, impl embassy_stm32::gpio::Pin>,
        brake_pin: Peri<'d, impl embassy_stm32::gpio::Pin>,
    ) -> Self {
        // The BLM3650 ESC is controlled with open-drain pins with external pull-ups. The pins are set to high (open-drain) by default, which is the default state for the ESC.
        let direction_pin =
            OutputOpenDrain::new_pull(direction_pin, Level::High, Speed::Low, Pull::None);
        let brake_pin = OutputOpenDrain::new_pull(brake_pin, Level::High, Speed::Low, Pull::None);

        Self {
            pwm_channel,
            direction_pin,
            brake_pin,
        }
    }

    pub fn enable(&mut self) {
        self.set_pwm(0.0);
        self.pwm_channel.enable();
    }

    /// Set the PWM value of the timer between PWM_MIN and PWM_MAX, where negative values indicate reverse direction. Brakes if |pwm| is less than PWM_DEAD_ZONE.
    pub fn set_pwm(&mut self, pwm: f32) {
        let speed = pwm.clamp(PWM_MIN, PWM_MAX);

        if speed > PWM_DEAD_ZONE {
            self.direction_pin.set_high();
            // Open-drain command, high (open-drain, default state) means the brake is not active
            self.brake_pin.set_high();
        } else if speed < -PWM_DEAD_ZONE {
            self.direction_pin.set_low();
            self.brake_pin.set_high();
        } else {
            self.brake_pin.set_low();
        }

        let duty_cycle = self.pwm_channel.max_duty_cycle() as f32 * speed.abs();

        self.pwm_channel.set_duty_cycle(duty_cycle as u32);
    }
}
