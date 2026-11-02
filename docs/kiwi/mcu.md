# STM32 Firmware (`kiwi-mcu`)

The embedded firmware runs on the **NUCLEO-F767ZI** board (`STM32F767ZI`, ARM Cortex-M7 with hardware FPU `thumbv7em-none-eabihf`). It is written in `#![no_std]` Rust on top of the **Embassy** async runtime (`embassy-executor`, `embassy-stm32`, `embassy-sync`, `embassy-time`).

---

## 1. Code Organization (`crates/kiwi-mcu/src/`)

The codebase follows a strict 4-layer separation:

1. **[`main.rs`](../../crates/kiwi-mcu/src/main.rs)**:
   - Calls `Board::init()`, waits 500 ms for peripherals to stabilize, and spawns (`spawner.spawn`) all asynchronous tasks.
2. **[`board.rs`](../../crates/kiwi-mcu/src/board.rs)**:
   - **The only file allowed to directly reference hardware pins (`p.PA6`, `p.PB14`, etc.) and low-level peripheral configuration** (216 MHz HSE+PLL clock tree, timer frequencies, ADC resolution, `bind_interrupts!` mappings, static DMA/UART buffers).
   - Instantiates drivers and returns a [`Board`](../../crates/kiwi-mcu/src/board.rs#L100-L110) struct whose fields are moved into their respective tasks.
3. **[`drivers/`](../../crates/kiwi-mcu/src/drivers/)**:
   - Reusable hardware abstractions decoupled from application logic:
     - [`blm3650.rs`](../../crates/kiwi-mcu/src/drivers/blm3650.rs): BLM3650 brushless motor driver (20 kHz PWM + Open-Drain `direction` and `brake` pins with `PWM_DEAD_ZONE = 0.08`).
     - [`one_channel_encoder.rs`](../../crates/kiwi-mcu/src/drivers/one_channel_encoder.rs): 32-bit Input Capture frequency measurement (`TIM2`) on falling edges with hardware input filtering and stall timeout detection (`max_period`).
4. **[`tasks/`](../../crates/kiwi-mcu/src/tasks/) & [`channels.rs`](../../crates/kiwi-mcu/src/channels.rs)**:
   - Domain tasks (`battery_management`, `safety`, `wheels`, `uart`, `communication`) run cooperatively on the single-threaded Embassy executor (`executor-thread`).
   - Tasks **never communicate directly with each other**; all inter-task state and events go through static synchronization primitives declared in [`channels.rs`](../../crates/kiwi-mcu/src/channels.rs).

---

## 2. Inter-Task Synchronization (`channels.rs`)

Because all tasks run on the same thread-mode executor (no interrupt handlers access the channels), every synchronization primitive uses `ThreadModeRawMutex`.

Choosing between `Watch`, `Signal`, and `Channel` follows strict rules:

| `embassy_sync` Primitive | Use Case | Behavior | Current Examples |
| :--- | :--- | :--- | :--- |
| **`Watch<M, T, N>`** | **Persistent robot state** (shared across multiple readers or mirrored to RPi/GCS). | Always holds the latest value. Allows $N$ tasks to subscribe to changes (`.changed().await`) and any code to read the current value synchronously (`.try_get()`). | `BATTERY_VOLTAGE`, `BATTERY_LEVEL`, `ARM_STATE` |
| **`Signal<M, T>`** | **Real-time setpoint / command** (1 producer $\rightarrow$ 1 consumer). | Only the latest value matters (overwrites unread values). Consumer awaits `.wait().await` or polls `.try_take()`. | `ARM_CMD`, `WHEEL_SPEED_SETPOINTS` |
| **`Channel<M, T, N>`** | **Event or packet queue** (FIFO of capacity $N$). | Every message must be delivered in order without overwriting; queues up to $N$ items. | `UART_RX`, `UART_TX` |

---

## 3. The `Watch` Pattern in `channels.rs`

Every global state backed by a `Watch` in [`channels.rs`](../../crates/kiwi-mcu/src/channels.rs) follows a standardized pattern that automates telemetry forwarding to the Raspberry Pi:

1. **Private `static` visibility**:
   The `Watch` static itself is **never** `pub`. This prevents tasks from mutating state without going through the update function.
2. **Exact `*_RECEIVERS` constant**:
   Embassy statically allocates receiver slots for a `Watch`. The constant (e.g., `const BATTERY_LEVEL_RECEIVERS: usize = 2;`) must match the exact number of `subscribe_*()` calls across tasks. If another task subscribes without incrementing this constant, `defmt::expect!` will panic at startup with `"CHANNELS: No more ... receiver available."`.
3. **Default initialization (`Watch::new_with(...)`)**:
   Every `Watch` is initialized with `Watch::new_with(default_val)` so that `.try_get().unwrap()` is guaranteed never to panic.
4. **The 3 public accessor functions**:
   - `pub async fn update_<name>(new_state: T)`: Calls the internal [`update_watch`](../../crates/kiwi-mcu/src/channels.rs#L10-L25) helper (`send_if_modified`). **If and only if the value changed**, it notifies local subscribers **and** automatically enqueues the update onto `UART_TX.send(McuToRpi::...).await`.
   - `pub fn subscribe_<name>() -> watch::Receiver<...>`: Returns a receiver for a task that reacts to state changes (`.changed().await` or `.get().await`).
   - `pub fn get_<name>() -> T`: Immediate non-blocking read (`.try_get().unwrap()`), used notably by [`get_state_handler`](../../crates/kiwi-mcu/src/tasks/communication.rs#L28-L39) to answer `GetStateReq` RPC requests.

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
  - Polls wheel speed setpoints `[f32; 3]` (in rad/s) via `WHEEL_SPEED_SETPOINTS.try_take()`.
  - **Setpoint watchdog**: if no setpoint update is received within `SETPOINTS_TIMEOUT` (`1000 ms`), setpoints are reset to `[0.0, 0.0, 0.0]`.
  - **Speed sign reconstruction**: because each motor uses a single-channel pulse encoder (no A/B quadrature), `encoder.get_frequency()` returns an unsigned speed magnitude. [`speed_sign`](../../crates/kiwi-mcu/src/tasks/wheels.rs#L27-L33) infers the sign: when the previous filtered speed is near zero ($\le 3.0\text{ rad/s}$), the sign of the current PWM command is used; otherwise, the previous speed sign is kept (mechanical inertia).
  - Applies a 6-sample moving average filter (`SPEED_FILTER_WINDOW = 6`) and computes the PWM command via [`PIDController`](../../crates/shared/src/pid.rs) (PID + thresholded affine feedforward + clamping anti-windup).

### `uart.rs` & `communication.rs`
- **`uart_task`**: Splits `USART1` into `tx` and `rx` and runs concurrently (`join`):
  - `uart_rx_task`: Reads byte-by-byte until `0x00`, handles buffer overflows (`256` bytes), decodes the frame via `shared::messages::decode`, and pushes it to `UART_RX`.
  - `uart_tx_task`: Waits for messages on `UART_TX`, encodes them via `shared::messages::encode`, and writes them to UART.
- **`message_dispatcher`**: Consumes `UART_RX` and dispatches each `RpiToMcu` variant to the appropriate `Signal` (`WHEEL_SPEED_SETPOINTS`, `ARM_CMD`) or replies on `UART_TX` for RPC requests (`GetStateReq`).
