use embassy_stm32::gpio::OutputOpenDrain;
use embassy_stm32::timer::GeneralInstance4Channel;
use embassy_stm32::timer::simple_pwm::SimplePwmChannel;

pub struct Motor<'d, T: GeneralInstance4Channel> {
    pwm_channel: SimplePwmChannel<'static, T>,
    direction_pin: OutputOpenDrain<'d>,
    brake_pin: OutputOpenDrain<'d>,
}

impl<'d, T: GeneralInstance4Channel> Motor<'d, T> {
    pub fn new(
        pwm_channel: SimplePwmChannel<'static, T>,
        direction_pin: OutputOpenDrain<'d>,
        brake_pin: OutputOpenDrain<'d>,
    ) -> Self {
        Self {
            pwm_channel,
            direction_pin,
            brake_pin,
        }
    }

    pub fn enable(&mut self) {
        self.set_pwm_speed(0.0);
        self.pwm_channel.enable();
    }

    /// Set the speed of the motor using PWM value between -1.0 and 1.0, where negative values indicate reverse direction. Brakes if speed is 0.0.
    pub fn set_pwm_speed(&mut self, speed: f32) {
        let speed = speed.clamp(-1.0, 1.0);

        if speed > 0.0 {
            self.direction_pin.set_high();
            // Open-drain command, high (open-drain, default state) means the brake is not active
            self.brake_pin.set_high();
        } else if speed < 0.0 {
            self.direction_pin.set_low();
            self.brake_pin.set_high();
        } else {
            self.brake_pin.set_low();
        }

        self.pwm_channel
            .set_duty_cycle_percent((speed.abs() * 100.0) as u8);
    }
}
