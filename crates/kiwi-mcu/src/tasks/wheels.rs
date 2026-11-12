use crate::board::{WheelEncoders, WheelMotors};
use crate::channels::WHEEL_SPEED_SETPOINTS;
use crate::drivers::blm3650;
use crate::{board, channels};
use core::array;
use embassy_time::Duration;
use shared::pid::PIDController;

const KP: f32 = 0.04;
const KI: f32 = 0.05;
const KD: f32 = 0.0;
const FF_INTERCEPT: f32 = blm3650::PWM_DEAD_ZONE;
const FF_SLOPE: f32 = 0.0277;
const FF_THRESHOLD: f32 = 2.0; // rad/s
const SPEED_FILTER_WINDOW: usize = 6;
const SPEED_ZERO_CROSSING_THRESHOLD: f32 = 3.0;
const WHEELS_CONTROL_TASK_DT: Duration = Duration::from_millis(1);
pub const SETPOINTS_TIMEOUT: Duration = Duration::from_millis(1000);

fn get_wheel_speed_setpoints() -> Option<[f32; 3]> {
    WHEEL_SPEED_SETPOINTS.try_take()
}

/// Compute the sign of the speed by detecting zero crossing. If the absolute value of the
/// speed is less than or equal to the threshold, the speed probably crossed zero so it is now the same sign as the command.
/// Otherwise, the speed is still far from zero so it is probably still the same sign as before.
fn speed_sign(speed: f32, cmd: f32, threshold: f32) -> f32 {
    if speed.abs() <= threshold {
        cmd.signum()
    } else {
        speed.signum()
    }
}

#[embassy_executor::task]
pub async fn wheels_control_task(mut wheel_motors: WheelMotors, mut wheel_encoders: WheelEncoders) {
    let mut pids: [PIDController; 3] = array::from_fn(|_| {
        PIDController::new(
            KP,
            KI,
            KD,
            (FF_SLOPE, FF_INTERCEPT),
            FF_THRESHOLD,
            (blm3650::PWM_MIN, blm3650::PWM_MAX),
        )
    });
    let mut cmds = [0.0f32; 3];
    let mut speeds = [[0.0f32; SPEED_FILTER_WINDOW]; 3];
    let mut setpoints = [0.0f32; 3];

    let mut last_setpoints_timestamp = embassy_time::Instant::now();

    let mut ticker = embassy_time::Ticker::every(WHEELS_CONTROL_TASK_DT);
    loop {
        ticker.next().await;

        setpoints = match get_wheel_speed_setpoints() {
            Some(sp) => {
                last_setpoints_timestamp = embassy_time::Instant::now();
                sp
            }
            None => {
                // This prevents the wheels from continuing to spin if the setpoints are not updated for a long time. (no signal from the controller ahead)
                if last_setpoints_timestamp.elapsed() > SETPOINTS_TIMEOUT {
                    [0.0f32; 3]
                } else {
                    setpoints
                }
            }
        };

        let mut current_speeds = [0.0f32; 3];

        for i in 0..3 {
            let encoder = &mut wheel_encoders[i];
            let pid = &mut pids[i];
            let motor = &mut wheel_motors[i];
            let cmd = &mut cmds[i];
            let speeds = &mut speeds[i];
            let setpoint = setpoints[i];

            speeds.rotate_right(1); // Makes room for the new speed measurement at index 0
            speeds[0] = encoder.get_frequency()
                * board::WHEEL_ENCODER_RADIANS_PER_TICK
                * speed_sign(speeds[1], *cmd, SPEED_ZERO_CROSSING_THRESHOLD);
            let speed = speeds.iter().sum::<f32>() / SPEED_FILTER_WINDOW as f32;
            current_speeds[i] = speed;

            *cmd = pid.update(
                setpoint,
                speed,
                WHEELS_CONTROL_TASK_DT.as_millis() as f32 / 1000.0,
            );
            motor.set_pwm(*cmd);
        }

        channels::WHEEL_SPEEDS.sender().send(current_speeds);
    }
}
