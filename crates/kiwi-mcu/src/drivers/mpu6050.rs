use embassy_stm32::i2c;
use embassy_stm32::i2c::Master;
use embassy_stm32::mode::Async;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

const SENSITIVITY: f32 = (500.0 / 32768.0f32).to_radians();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I2C error: {0}")]
    I2c(#[from] i2c::Error),
}

/// Driver for the MPU-6050 6-axis IMU over an asynchronous I2C bus.
pub struct Mpu6050<'a> {
    i2c: &'a Mutex<ThreadModeRawMutex, i2c::I2c<'static, Async, Master>>,
    address: u8,
    z_gyro_bias: f32,
}

impl<'a> Mpu6050<'a> {
    /// Creates a new [`Mpu6050`] driver instance on the given shared `i2c` bus.
    ///
    /// `address` is the 7-bit I2C slave address (`0x68` when `AD0` is pulled low,
    /// or `0x69` when `AD0` is pulled high).
    pub fn new(
        i2c: &'a Mutex<ThreadModeRawMutex, i2c::I2c<'static, Async, Master>>,
        address: u8,
    ) -> Self {
        Self {
            i2c,
            address,
            z_gyro_bias: 0.0,
        }
    }

    /// Wakes up the MPU-6050 and configures its internal registers:
    /// - Clock source set to the X-axis gyroscope PLL (`PWR_MGMT_1 = 0x01`)
    /// - Digital Low-Pass Filter (DLPF) set to ~42 Hz for the gyroscope (`CONFIG = 0x03`)
    /// - Gyroscope full-scale range set to ±500 °/s (`GYRO_CONFIG = 0x08`)
    ///
    /// # Errors
    /// Can return an error if the I2C write operation fails.
    pub async fn init(&self) -> Result<(), Error> {
        const REG_PWR_MGMT_1: u8 = 0x6B;
        const REG_CONFIG: u8 = 0x1A;
        const REG_GYRO_CONFIG: u8 = 0x1B;

        let mut i2c = self.i2c.lock().await;

        i2c.write(self.address, &[REG_PWR_MGMT_1, 0x01]).await?; // SLEEP = 0, CLKSEL = 1 (PLL Gyro X)
        i2c.write(self.address, &[REG_CONFIG, 0x03]).await?; // Low-pass filter 44Hz
        i2c.write(self.address, &[REG_GYRO_CONFIG, 0x08]).await?; // FS_SEL = 1 (500 deg/s)

        Ok(())
    }

    /// Computes and stores the Z-axis gyroscope zero-rate bias (in rad/s) by averaging
    /// `samples` measurements (defaults to `200` if `None`, sampled every 2 ms).
    ///
    /// # Errors
    /// Can return an error if the I2C write operation fails.
    ///
    /// # Note
    /// The sensor must remain completely stationary while this method runs.
    pub async fn calibrate(&mut self, samples: Option<usize>) -> Result<(), Error> {
        let samples = samples.unwrap_or(500);

        self.z_gyro_bias = 0.0;

        let mut sum = 0.0;
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(2));
        for _ in 0..samples {
            ticker.next().await;
            sum += self.read_z_gyro().await?;
        }
        self.z_gyro_bias = sum / samples as f32;

        Ok(())
    }

    /// Reads the Z-axis angular velocity from the gyroscope in radians per second (rad/s),
    /// compensated by the calibrated zero-rate bias.
    ///
    /// # Errors
    /// Can return an error if the I2C read operation fails.
    pub async fn read_z_gyro(&self) -> Result<f32, Error> {
        const Z_GYRO_ADDR: u8 = 0x47;
        let mut buffer = [0u8; 2];

        let mut i2c = self.i2c.lock().await;

        i2c.write_read(self.address, &[Z_GYRO_ADDR], &mut buffer)
            .await?;

        Ok((f32::from(i16::from_be_bytes(buffer)) * SENSITIVITY) - self.z_gyro_bias)
    }
}
