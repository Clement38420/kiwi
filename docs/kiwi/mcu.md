# STM32 Firmware (`kiwi-mcu`)

The embedded firmware runs on the **NUCLEO-F767ZI** board (`STM32F767ZI`, ARM Cortex-M7 with hardware FPU `thumbv7em-none-eabihf`). It is written in `#![no_std]` Rust on top of the **Embassy** async runtime (`embassy-executor`, `embassy-stm32`, `embassy-sync`, `embassy-time`).

---

## 1. Code Organization (`crates/kiwi-mcu/src/`)

The codebase follows a strict 4-layer separation:

1. **[`main.rs`](../../crates/kiwi-mcu/src/main.rs)**:
   - Calls `Board::init().await`, waits 500 ms for peripherals to stabilize, and spawns (`spawner.spawn`) all asynchronous tasks.
2. **[`board.rs`](../../crates/kiwi-mcu/src/board.rs)**:
   - **The only file allowed to directly reference hardware pins (`p.PA6`, `p.PB14`, `p.PF0`, etc.) and low-level peripheral configuration** (216 MHz HSE+PLL clock tree, timer frequencies, ADC resolution, I2C2 with DMA, `bind_interrupts!` mappings, static DMA/UART buffers).
   - Instantiates drivers and returns a [`Board`](../../crates/kiwi-mcu/src/board.rs#L117-L129) struct whose fields are moved into their respective tasks.
3. **[`drivers/`](../../crates/kiwi-mcu/src/drivers/)**:
   - Reusable hardware abstractions decoupled from application logic:
     - [`blm3650.rs`](../../crates/kiwi-mcu/src/drivers/blm3650.rs): BLM3650 brushless motor driver (20 kHz PWM + Open-Drain `direction` and `brake` pins with `PWM_DEAD_ZONE = 0.08`). Direction pin polarity: `set_low()` for positive speeds (> dead zone), `set_high()` for negative speeds (< -dead zone); brake pin is active-low (`set_low()` on stop).
     - [`one_channel_encoder.rs`](../../crates/kiwi-mcu/src/drivers/one_channel_encoder.rs): 32-bit Input Capture frequency measurement (`TIM2`) on falling edges with hardware input filtering and stall timeout detection (`max_period`).
     - [`mpu6050.rs`](../../crates/kiwi-mcu/src/drivers/mpu6050.rs): Asynchronous I2C driver for the MPU-6050 IMU (`0x68`). Configures the PLL clock (`CLKSEL = 1`), a 44 Hz low-pass filter (`CONFIG = 0x03`), and a $\pm 500\,^\circ/\text{s}$ gyroscope range (`FS_SEL = 1`), provides stationary zero-rate bias calibration (`calibrate`, averaging 500 samples every 2 ms), and reads bias-compensated $Z$-axis angular velocity in $\text{rad/s}$ (`read_z_gyro`).
4. **[`tasks/`](../../crates/kiwi-mcu/src/tasks/) & [`channels.rs`](../../crates/kiwi-mcu/src/channels.rs)**:
   - Domain tasks (`battery_management`, `safety`, `wheels`, `imu`, `odometry`, `uart`, `communication`) run cooperatively on the single-threaded Embassy executor (`executor-thread`).
   - Tasks **never communicate directly with each other**; all inter-task state and events go through static synchronization primitives declared in [`channels.rs`](../../crates/kiwi-mcu/src/channels.rs).

---

## 2. Inter-Task Synchronization (`channels.rs`)

Because all tasks run on the same thread-mode executor (no interrupt handlers access the channels), every synchronization primitive uses `ThreadModeRawMutex`.

Choosing between `Watch`, `Signal`, and `Channel` follows strict rules:

| `embassy_sync` Primitive | Use Case | Behavior | Current Examples |
| :--- | :--- | :--- | :--- |
| **`Watch<M, T, N>`** | **Persistent robot state or high-rate shared state** (shared across tasks or mirrored to RPi/GCS). | Always holds the latest value. Allows $N$ tasks to subscribe to changes (`.changed().await` / `.try_changed()`) and any code to read or mutate the current value synchronously (`.try_get()`, `.send_modify()`). | `BATTERY_VOLTAGE`, `BATTERY_LEVEL`, `ARM_STATE`, `WHEEL_SPEEDS`, `POSE` |
| **`Signal<M, T>`** | **Real-time setpoint / command** (1 producer $\rightarrow$ 1 consumer). | Only the latest value matters (overwrites unread values). Consumer awaits `.wait().await` or polls `.try_take()`. | `ARM_CMD`, `WHEEL_SPEED_SETPOINTS` |
| **`Channel<M, T, N>`** | **Event or packet queue** (FIFO of capacity $N$). | Every message must be delivered in order without overwriting; queues up to $N$ items. | `UART_RX`, `UART_TX` |

---

## 3. The `Watch` Pattern in `channels.rs`

### Persistent State Telemetry (`BATTERY_VOLTAGE`, `BATTERY_LEVEL`, `ARM_STATE`)
Global states mirrored inside `McuState` follow a standardized pattern that automates telemetry forwarding to the Raspberry Pi:

1. **Private `static` visibility**:
   The `Watch` static itself is **never** `pub`. This prevents tasks from mutating state without going through the update function.
2. **Exact `*_RECEIVERS` constant**:
   Embassy statically allocates receiver slots for a `Watch`. The constant (e.g., `const BATTERY_LEVEL_RECEIVERS: usize = 2;`) must match the exact number of `subscribe_*()` calls across tasks. If another task subscribes without incrementing this constant, `defmt::expect!` will panic at startup with `"CHANNELS: No more ... receiver available."`.
3. **Default initialization (`Watch::new_with(...)`)**:
   Every `Watch` is initialized with `Watch::new_with(default_val)` so that `.try_get().unwrap()` is guaranteed never to panic.
4. **The 3 public accessor functions** (plus `get_full_state`):
   - `pub async fn update_<name>(new_state: T)`: Calls the internal [`update_watch`](../../crates/kiwi-mcu/src/channels.rs#L10-L25) helper (`send_if_modified`). **If and only if the value changed**, it notifies local subscribers **and** automatically enqueues the update onto `UART_TX.send(McuToRpi::...).await`.
   - `pub fn subscribe_<name>() -> watch::Receiver<...>`: Returns a receiver for a task that reacts to state changes (`.changed().await` or `.get().await`).
   - `pub fn get_<name>() -> T`: Immediate non-blocking read (`.try_get().unwrap()`), aggregated by [`get_full_state()`](../../crates/kiwi-mcu/src/channels.rs#L86-L92) to build a complete `McuState` snapshot when the RPi sends `RpiToMcu::Sync`.

### High-Rate Kinematic State (`WHEEL_SPEEDS`, `POSE`)
High-frequency kinematic variables (`1 kHz`) do not use `update_watch` (which would saturate `UART_TX` at 1 kHz):
- **`WHEEL_SPEEDS`**: Updated at `1 kHz` by `wheels_control_task` via `WHEEL_SPEEDS.sender().send(...)` and read synchronously by `odometry_task` via `get_wheel_speeds()`.
- **`POSE`**: Mutated in place at `1 kHz` by both `imu_task` (updating `pose.theta`) and `odometry_task` (integrating `pose.x` and `pose.y`) via `POSE.sender().send_modify(...)`. A dedicated task (`send_pose_task` in `tasks/odometry.rs`) polls `rx.try_changed()` at `50 Hz` and streams `McuToRpi::Odom(pose)` over `UART_TX` without blocking (`try_send`).

> [!IMPORTANT]
> For noisy analog values like `BATTERY_VOLTAGE` (`f32`), mind the call frequency of `update_battery_voltage`: because `f32` changes on the slightest ADC fluctuation, calling it at high frequency would flood `UART_TX`. Currently, [`battery_monitoring_task`](../../crates/kiwi-mcu/src/tasks/battery_management.rs#L53-L82) averages 32 samples and updates once per second (`1000 ms`).

---

## 4. Task Breakdown (`crates/kiwi-mcu/src/tasks/`)

### `battery_management.rs`
- **`battery_monitoring_task`** (period: `1000 ms`):
  - Locks the `ADC1` mutex, averages 32 blocking reads on `PC0` (`SampleTime::CYCLES144`), and converts to volts using `BATTERY_VOLTAGE_RATIO = 0.0365`.
  - Computes `BatteryLevel` (`High`, `Medium`, `Low`, `Critical`) using a **hysteresis state machine** (`threshold()`) to prevent rapid toggling near voltage boundaries.
  - Calls `update_battery_level` and `update_battery_voltage`.
- **`battery_led_task`** (period: `500 ms`):
  - Subscribes to `BATTERY_LEVEL` (receiver #1). Drives the active-low dual-color LED (`PG6` red, `PG7` green): Green = `High`, Amber (Green+Red) = `Medium`, Solid Red = `Low`, Blinking Red = `Critical`.

### `safety.rs`
- **`safety_task`**:
  - Subscribes to `BATTERY_LEVEL` (receiver #2) and awaits `ARM_CMD` via `select(battery_rx.changed(), ARM_CMD.wait())`.
  - Controls the `pwr_cmd` pin (`PG8`) driving the motor power relay:
    - On `BatteryLevel::Critical`: immediately cuts power (`pwr_cmd.set_low()`) and transitions to `ArmState::CannotArm`.
    - When battery recovers above `Critical`, transitions from `CannotArm` to `Disarmed` (never re-arms automatically).
    - On `ARM_CMD` (`true`/`false`), enables or disables `pwr_cmd` (unless `CannotArm`) and updates `ARM_STATE`.

### `wheels.rs`
- **`wheels_control_task`** (period: `1 ms` / `1 kHz`):
  - **Disarmed guard**: when `get_arm_state() != ArmState::Armed`, drains any pending `WHEEL_SPEED_SETPOINTS`, zeroes setpoints, commands, and speed filter buffers, calls `pid.reset()` and `motor.set_pwm(0.0)` on all 3 wheels, publishes `[0.0; 3]` to `WHEEL_SPEEDS`, and skips closed-loop computation. This prevents PID windup and ghost odometry integration from residual encoder noise while disarmed.
  - Polls wheel speed setpoints `[f32; 3]` (in rad/s) via `WHEEL_SPEED_SETPOINTS.try_take()`.
  - **Setpoint watchdog**: if no setpoint update is received within `SETPOINTS_TIMEOUT` (`1000 ms`), setpoints are reset to `[0.0, 0.0, 0.0]`.
  - **Speed sign reconstruction**: because each motor uses a single-channel pulse encoder (no A/B quadrature), `encoder.get_frequency()` returns an unsigned speed magnitude. [`speed_sign`](../../crates/kiwi-mcu/src/tasks/wheels.rs#L28-L34) infers the sign: when the previous filtered speed is near zero ($\le 3.0\text{ rad/s}$), the sign of the current PWM command is used; otherwise, the previous speed sign is kept (mechanical inertia).
  - Applies a 6-sample moving average filter (`SPEED_FILTER_WINDOW = 6`), computes the PWM command via [`PIDController`](../../crates/shared/src/pid.rs) (PID + thresholded affine feedforward + clamping anti-windup), and publishes the filtered speeds `[f32; 3]` to `WHEEL_SPEEDS`.

### `imu.rs`
- **`imu_task`** (period: `1 ms` / `1 kHz`):
  - On startup, calibrates the MPU-6050 gyroscope zero-rate bias (`imu.calibrate(None)`, retrying every 1 s on I2C error).
  - Every `1 ms`, reads the bias-compensated $Z$-axis angular velocity $\omega$ (`imu.read_z_gyro()`) and integrates the robot heading $\theta$ in `POSE` using **trapezoidal integration** (`f32::midpoint(w, prev_w) * dt`), wrapping $\theta \in [-\pi, \pi]$.

### `odometry.rs`
- **`odometry_task`** (period: `1 ms` / `1 kHz`):
  - Reads the latest filtered wheel speeds $[\omega_0, \omega_1, \omega_2]$ via `channels::get_wheel_speeds()`.
  - Computes body-frame linear velocities using forward 3-wheel Kiwi kinematics:
    $$v_x = \frac{R}{\sqrt{3}}(\omega_1 - \omega_2), \qquad v_y = \frac{R}{3}(-2\omega_0 + \omega_1 + \omega_2)$$
  - Rotates $(v_x \Delta t, v_y \Delta t)$ by the current IMU heading $\theta$ and accumulates world-frame coordinates $(x, y)$ in `POSE` via `send_modify`.
- **`send_pose_task`** (period: `20 ms` / `50 Hz`):
  - Subscribes to `POSE` and streams `McuToRpi::Odom(pose)` onto `UART_TX` (`try_send`) whenever `POSE` has changed since the last tick (`rx.try_changed()`).

### `uart.rs` & `communication.rs`
- **`uart_task`**: Splits `USART1` into `tx` and `rx` and runs concurrently (`join`):
  - `uart_rx_task`: Reads byte-by-byte until `0x00`, handles buffer overflows (`256` bytes), decodes the frame via `shared::messages::decode`, and pushes it to `UART_RX`.
  - `uart_tx_task`: Waits for messages on `UART_TX`, encodes them via `shared::messages::encode`, and writes them to UART.
- **`message_dispatcher_task`**: Sends an initial synchronization snapshot (`send_sync()`) at startup, then consumes `UART_RX`:
  - `RpiToMcu::WheelSpeedSetpoints(setpoints)` $\rightarrow$ `WHEEL_SPEED_SETPOINTS.signal(setpoints)`.
  - `RpiToMcu::Arm(arm)` $\rightarrow$ `ARM_CMD.signal(arm)`.
  - `RpiToMcu::Sync` $\rightarrow$ sends `McuToRpi::McuState(get_full_state())` and `McuToRpi::Odom(POSE)` on `UART_TX`.
  - `RpiToMcu::Reset` $\rightarrow$ resets `POSE` to `RobotPose::default()` and sends both `McuToRpi::Odom(RobotPose::default())` and `McuToRpi::ResetAck` on `UART_TX` to synchronize the RPi LiDAR odometry reset.
