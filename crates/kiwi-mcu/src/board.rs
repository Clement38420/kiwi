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
//!
//! ## Communication (`USART1`)
//! Buffered UART with interrupt-driven RX/TX, 8N1, 115200 baud.
//!
//! | Pin | Peripheral / Mode | Description |
//! | :--- | :--- | :--- |
//! | `PB14` | `USART1_RX` | UART RX |
//! | `PB15` | `USART1_TX` | UART TX |
//!
//! ## I2C (`I2C2`)
//! 400 kHz I2C used for the IMU
//!
//! | Pin | Peripheral / Mode | Description |
//! | :--- | :--- | :--- |
//! | `PF0` | `I2C2_SDA` | I2C2 data |
//! | `PF1` | `I2C2_SCL` | I2C2 clock |

use crate::drivers;
use embassy_stm32::i2c::Master;
use embassy_stm32::mode::Async;
use embassy_stm32::time::khz;
use embassy_stm32::{
    Config,
    adc::{AdcChannel, AdcConfig, AnyAdcChannel, Resolution},
    dma,
    gpio::{Level, Output, OutputType, Pull, Speed},
    i2c,
    interrupt::typelevel::Handler,
    rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllSource,
        Sysclk,
    },
    time::{Hertz, mhz},
    timer::{
        input_capture::CapturePin,
        low_level::{CountingMode, FilterValue},
        simple_pwm::{PwmPin, SimplePwm, SimplePwmChannels},
    },
    {adc, bind_interrupts, interrupt, peripherals, timer, usart},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex, once_lock::OnceLock};
use static_cell::ConstStaticCell;

static WHEEL_ENCODERS_TIM: OnceLock<drivers::one_channel_encoder::Timer<peripherals::TIM2>> =
    OnceLock::new();
const WHEEL_ENCODER_TIM_TICK: Hertz = mhz(1);
const WHEEL_ENCODER_FILTER: FilterValue = FilterValue::FDTS_DIV8_N8; // Timer clock is 108 MHz (fDTS = 108 MHz), so 8 samples at fDTS/8 gives a ~593ns filter cut-off
const WHEEL_ENCODER_MAX_PERIOD: u32 = 100_000;
pub const WHEEL_ENCODER_RADIANS_PER_TICK: f32 = 1.0 / 6.0 / 27.0 * 2.0 * core::f32::consts::PI; // 6 * 27 ticks per revolution after the gearbox

static ADC1: OnceLock<Mutex<ThreadModeRawMutex, adc::Adc<'static, peripherals::ADC1>>> =
    OnceLock::new();
const ADC1_RESOLUTION: Resolution = Resolution::BITS10;

const UART1_BAUDRATE: u32 = 115_200;
pub const UART1_RX_BUFFER_SIZE: usize = 256;
pub const UART1_TX_BUFFER_SIZE: usize = 256;
static UART1_RX_BUFFER: ConstStaticCell<[u8; UART1_RX_BUFFER_SIZE]> =
    ConstStaticCell::new([0; UART1_RX_BUFFER_SIZE]);
static UART1_TX_BUFFER: ConstStaticCell<[u8; UART1_TX_BUFFER_SIZE]> =
    ConstStaticCell::new([0; UART1_TX_BUFFER_SIZE]);

static I2C2: OnceLock<Mutex<ThreadModeRawMutex, i2c::I2c<'static, Async, Master>>> =
    OnceLock::new();
const I2C2_FREQUENCY: Hertz = khz(400);

const MPU6050_ADDRESS: u8 = 0x68;

struct WheelEncodersIRQ;
impl Handler<interrupt::typelevel::TIM2> for WheelEncodersIRQ {
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
    pub _tim2: &'static drivers::one_channel_encoder::Timer<peripherals::TIM2>,
    pub pwr_cmd: Output<'static>,
    pub adc1: &'static Mutex<ThreadModeRawMutex, adc::Adc<'static, peripherals::ADC1>>,
    pub voltage_led_red: Output<'static>,
    pub voltage_led_green: Output<'static>,
    pub voltage_sense: AnyAdcChannel<'static, peripherals::ADC1>,
    pub uart1: usart::BufferedUart<'static>,
    pub _i2c2: &'static Mutex<ThreadModeRawMutex, i2c::I2c<'static, Async, Master>>,
    pub imu: drivers::mpu6050::Mpu6050<'static>,
}

#[derive(Debug, thiserror::Error)]
pub enum BoardInitError {
    #[error("UART initialization error: {0:?}")]
    Uart(usart::ConfigError),
    #[error("IMU initialization error: {0}")]
    Imu(#[from] drivers::mpu6050::Error),
}

impl Board {
    pub async fn init() -> Result<Board, BoardInitError> {
        let mut config = Config::default();
        config.rcc.hsi = false;
        config.rcc.hse = Some(Hse {
            freq: mhz(8), // 8 MHz MCO clock from ST-LINK
            mode: HseMode::Bypass,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,   // 8 MHz / 4 = 2 MHz PLL input
            mul: PllMul::MUL216,       // 2 MHz * 216 = 432 MHz VCO
            divp: Some(PllPDiv::DIV2), // 432 MHz / 2 = 216 MHz SYSCLK
            divq: None,
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1; // HCLK = 216 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV4; // PCLK1 = 54 MHz (APB1 timers = 108 MHz)
        config.rcc.apb2_pre = APBPrescaler::DIV2; // PCLK2 = 108 MHz (APB2 timers = 216 MHz)

        let p = embassy_stm32::init(config);

        bind_interrupts!(
            struct Irqs {
                TIM2 => WheelEncodersIRQ;
                USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
                I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
                I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
                DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
                DMA1_STREAM2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
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

        for motor in &mut wheel_motors {
            motor.enable();
        }

        let tim2 = drivers::one_channel_encoder::Timer::new(p.TIM2, Irqs, WHEEL_ENCODER_TIM_TICK);
        let tim2 = WHEEL_ENCODERS_TIM.get_or_init(|| tim2);

        let wheel_encoders = [
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, timer::Ch1>::new(p.PA0, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, timer::Ch3>::new(p.PB10, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
            tim2.add_encoder(
                CapturePin::<peripherals::TIM2, timer::Ch4>::new(p.PB11, Pull::None),
                WHEEL_ENCODER_FILTER,
                WHEEL_ENCODER_MAX_PERIOD,
            ),
        ];

        tim2.start();

        // Power command is OFF by default
        let pwr_cmd = Output::new(p.PG8, Level::Low, Speed::Low);

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

        let mut uart1_config = usart::Config::default(); // Default config is 8N1
        uart1_config.baudrate = UART1_BAUDRATE;

        let uart1 = usart::BufferedUart::new(
            p.USART1,
            p.PB15,
            p.PB14,
            UART1_RX_BUFFER.take(),
            UART1_TX_BUFFER.take(),
            Irqs,
            uart1_config,
        )
        .map_err(BoardInitError::Uart)?;

        let mut i2c2_cfg = i2c::Config::default();
        i2c2_cfg.frequency = I2C2_FREQUENCY;

        let i2c2 = I2C2.get_or_init(|| {
            Mutex::new(i2c::I2c::new(
                p.I2C2, p.PF1, p.PF0, p.DMA1_CH7, p.DMA1_CH2, Irqs, i2c2_cfg,
            ))
        });

        let imu = drivers::mpu6050::Mpu6050::new(i2c2, MPU6050_ADDRESS);
        imu.init().await?;

        Ok(Board {
            wheel_motors,
            wheel_encoders,
            _tim2: tim2,
            pwr_cmd,
            adc1,
            voltage_led_red,
            voltage_led_green,
            voltage_sense: p.PC0.degrade_adc(),
            uart1,
            _i2c2: i2c2,
            imu,
        })
    }
}
