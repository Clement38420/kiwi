pub struct PIDController {
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    prev_error: f32,
    cmd_bounds: (f32, f32),
    ff: (f32, f32),
    ff_threshold: f32,
}

impl PIDController {
    /// Create a new PID controller.
    ///
    /// # Arguments
    /// * `kp` - Proportional gain
    /// * `ki` - Integral gain
    /// * `kd` - Derivative gain
    /// * `ff` - Feedforward affine coefficients (slope, intercept)
    /// * `ff_threshold` - Threshold for the feedforward to be applied
    /// * `cmd_bounds` - Bounds for the command (min, max)
    pub fn new(
        kp: f32,
        ki: f32,
        kd: f32,
        ff: (f32, f32),
        ff_threshold: f32,
        cmd_bounds: (f32, f32),
    ) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            cmd_bounds,
            ff,
            ff_threshold,
        }
    }

    pub fn update(&mut self, setpoint: f32, measured_value: f32, dt: f32) -> f32 {
        let error = setpoint - measured_value;
        let ff = (self.ff.1 * setpoint.signum() + self.ff.0 * setpoint)
            * (setpoint.abs() > self.ff_threshold) as u8 as f32; // Feedforward is only applied if the setpoint is greater than ff_threshold in absolute value

        let p = self.kp * error;

        self.integral += self.ki * error * dt;

        let i_min = (self.cmd_bounds.0 - p - ff).min(0.0);
        let i_max = (self.cmd_bounds.1 - p - ff).max(0.0);
        self.integral = self.integral.clamp(i_min, i_max); // Anti-windup

        let d = self.kd * (error - self.prev_error) / dt;
        self.prev_error = error;

        (p + self.integral + d + ff).clamp(self.cmd_bounds.0, self.cmd_bounds.1)
    }
}
