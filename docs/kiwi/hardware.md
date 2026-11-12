# Robot Hardware & Pinout

This document gathers all hardware specifications directly tied to the embedded software (`kiwi-mcu` and `kiwi-rpi`).

---

## 1. Real-Time Unit: NUCLEO-F767ZI (`STM32F767ZI`)

- **Microcontroller**: ARM Cortex-M7 @ 216 MHz (`thumbv7em-none-eabihf`, clocked from the 8 MHz ST-LINK MCO in `HSE Bypass` mode via PLL: `SYSCLK = 216 MHz`, `PCLK1 = 54 MHz`, `PCLK2 = 108 MHz`).
- **Vendor Manuals**:
  - `docs/ST/um-stm32-nucleo144.pdf` (ST Zio / Morpho connector pinouts, board jumpers).
  - `docs/ST/rm-stm32f76xxx.pdf` (Registers, peripherals, timers).

### Reserved Internal Peripherals
| Peripheral | Usage | Configuration Details |
| :--- | :--- | :--- |
| **`TIM12`** | Embassy Time Driver | Configured via the `time-driver-tim12` feature of `embassy-stm32` (32,768 Hz tick). |
| **`TIM3`** | Motor PWM (Wheels 1, 2, 3) | `20 kHz`, Edge-Aligned Up (`SimplePwm` on `CH1`, `CH2`, `CH3`, 5,400 duty steps at 108 MHz timer clock). |
| **`TIM2`** | Encoder Input Capture | 32-bit timer clocked at `1 MHz` (`1 µs` per tick), hardware filter `FDTS_DIV8_N8` (~593 ns cutoff at 108 MHz timer clock), falling-edge interrupt. |
| **`ADC1`** | Battery Voltage Sense | `10-bit` resolution ($0\dots 1023$), `SampleTime::CYCLES144`, 32-sample averaging. |
| **`USART1`** | RPi Serial Link | `115200` baud, 8N1, interrupt-driven (`BufferedUart` with 256-byte RX/TX buffers). |
| **`I2C2`** | MPU-6050 IMU (`0x68`) | `400 kHz` Fast Mode, asynchronous master with DMA (`DMA1_CH7` TX / `DMA1_CH2` RX). |

---

## 2. Complete NUCLEO-F767ZI Pinout

Source of truth in code: [`crates/kiwi-mcu/src/board.rs`](../../crates/kiwi-mcu/src/board.rs).

### Wheel Motors (`TIM3` & `GPIOG`)
The integrated ESCs on the BLM3650 motors have externally pulled-up logic inputs: direction and brake pins are therefore configured as **Open-Drain** (`Pull::None`, default high).

| Pin | Peripheral / Mode | Function |
| :--- | :--- | :--- |
| **`PA6`** | `TIM3_CH1` (Push-Pull) | Wheel 1 Motor PWM |
| **`PG0`** | `GPIO` (Open-Drain) | Wheel 1 Motor Direction |
| **`PG1`** | `GPIO` (Open-Drain) | Wheel 1 Motor Brake (active low) |
| **`PA7`** | `TIM3_CH2` (Push-Pull) | Wheel 2 Motor PWM |
| **`PG2`** | `GPIO` (Open-Drain) | Wheel 2 Motor Direction |
| **`PG3`** | `GPIO` (Open-Drain) | Wheel 2 Motor Brake (active low) |
| **`PB0`** | `TIM3_CH3` (Push-Pull) | Wheel 3 Motor PWM |
| **`PG4`** | `GPIO` (Open-Drain) | Wheel 3 Motor Direction |
| **`PG5`** | `GPIO` (Open-Drain) | Wheel 3 Motor Brake (active low) |

### Wheel Encoders (`TIM2`)
Input capture on falling edge, `Pull::None` (external pull-up resistors).
> [!NOTE]
> `TIM2_CH2` is unused because it is not routed to the ST Zio connector required to wire external pull-ups.

| Pin | Peripheral / Mode | Function |
| :--- | :--- | :--- |
| **`PA0`** | `TIM2_CH1` (Input Capture) | Wheel 1 Encoder signal |
| **`PB10`** | `TIM2_CH3` (Input Capture) | Wheel 2 Encoder signal |
| **`PB11`** | `TIM2_CH4` (Input Capture) | Wheel 3 Encoder signal |

### Power & Battery Monitoring (`ADC1` & `GPIOG`)
| Pin | Peripheral / Mode | Function |
| :--- | :--- | :--- |
| **`PG8`** | `GPIO` (Push-Pull, default `Low`) | Power relay command (`pwr_cmd`) |
| **`PC0`** | `ADC1` (Analog) | Battery voltage sense via resistor divider (`voltage_sense`) |
| **`PG6`** | `GPIO` (Push-Pull, active low) | Battery status LED — Red |
| **`PG7`** | `GPIO` (Push-Pull, active low) | Battery status LED — Green |

### Raspberry Pi Serial Link (`USART1`)
| Pin | Peripheral / Mode | Function |
| :--- | :--- | :--- |
| **`PB14`** | `USART1_RX` | UART RX from Raspberry Pi |
| **`PB15`** | `USART1_TX` | UART TX to Raspberry Pi |

### Inertial Measurement Unit (`I2C2`)
| Pin | Peripheral / Mode | Function |
| :--- | :--- | :--- |
| **`PF0`** | `I2C2_SDA` | I2C2 data line to MPU-6050 (`0x68`) |
| **`PF1`** | `I2C2_SCL` | I2C2 clock line (`400 kHz`) |

---

## 3. Kinematics, Motor & Closed-Loop Control Specifications

- **Chassis & Wheel Geometry ([`crates/shared/src/consts.rs`](../../crates/shared/src/consts.rs))**:
  - 3 omni-wheels spaced at $120^\circ$ around the chassis center.
  - Wheel radius: `WHEEL_RADIUS = 0.03` m ($30\text{ mm}$).
  - Distance from robot center to each wheel contact point: `WHEEL_BASE = 0.184` m ($184\text{ mm}$).
  - Maximum wheel angular velocity saturation limit: `MAX_WHEEL_SPEED = 25.0` rad/s.
- **Motor Model**: **BLM3650** brushless motor with integrated ESC.
- **PWM Dead Zone**: `PWM_DEAD_ZONE = 0.08` (below 8% duty cycle magnitude, the active brake pin is pulled low).
- **Encoders & Gear Ratio**:
  - `6 * 27 = 162` ticks per output shaft revolution (`WHEEL_ENCODER_RADIANS_PER_TICK = 2 * PI / (6 * 27)` $\approx 0.038785\text{ rad/tick}$).
  - Pulse timeout (`WHEEL_ENCODER_MAX_PERIOD`): `100,000 µs` (`100 ms`). Beyond 100 ms without a falling edge, measured speed returns `0.0 Hz`.
- **PID Parameters (`wheels.rs`)**:
  - `1 kHz` control loop (`dt = 1 ms`), 6-sample moving average speed filter.
  - Gains: $K_p = 0.04$, $K_i = 0.05$, $K_d = 0.0$.
  - Affine feedforward: slope `0.0277`, intercept `0.08` (`PWM_DEAD_ZONE`), applied only when $|\text{setpoint}| > 2.0\text{ rad/s}$.

---

## 4. Battery & Safety Thresholds

- **Main Battery**: **6S LiFePO4** (288 Wh).
- **ADC Conversion**: $\text{Voltage (V)} = \text{ADC}_{10\text{bit}} \times 0.0365$ ($\times 11.3$ voltage divider with $3.3\text{ V} / 1024$ reference).
- **Hysteresis Thresholds ([`battery_management.rs`](../../crates/kiwi-mcu/src/tasks/battery_management.rs#L15-L34))**:

| State (`BatteryLevel`) | Downward Threshold (`exit_down`) | Upward Threshold (`exit_up`) | LED Status (`PG6`/`PG7`) | Safety Action (`safety.rs`) |
| :--- | :--- | :--- | :--- | :--- |
| **`High`** | $< 533$ ($\approx 19.2\text{ V}$) $\rightarrow$ `Medium` | — | Solid Green | Allows `Armed` |
| **`Medium`** | $< 516$ ($\approx 18.6\text{ V}$) $\rightarrow$ `Low` | $> 541$ ($\approx 19.5\text{ V}$) $\rightarrow$ `High` | Solid Amber (Green + Red) | Allows `Armed` |
| **`Low`** | $< 483$ ($\approx 17.4\text{ V}$) $\rightarrow$ `Critical` | $> 524$ ($\approx 18.9\text{ V}$) $\rightarrow$ `Medium` | Solid Red | Allows `Armed` (`crit` alert on GCS) |
| **`Critical`** | — | $> 494$ ($\approx 17.8\text{ V}$) $\rightarrow$ `Low` | Blinking Red ($1\text{ Hz}$) | **Cuts `pwr_cmd` (`PG8 = Low`) and locks `CannotArm`** |

---

## 5. Compute Unit: Raspberry Pi 4B (4 GB)

- **Network Hostname**: `kiwi.local` (user `clement`).
- **Systemd Service**: `kiwi.service` (binary deployed to `~/kiwi-rpi/kiwi-rpi`).
- **Hardware Connections on the Pi**:
  - `/dev/serial0` (`115200` baud, 8N1) $\leftrightarrow$ `USART1` (`PB14`/`PB15`) on NUCLEO-F767ZI.
  - `/dev/ttyUSB0` (`512000` baud, 8N1) $\leftrightarrow$ **YDLidar G6** LiDAR.
  - USB $\leftrightarrow$ NUCLEO-F767ZI onboard ST-Link (used by `probe-rs` for remote flashing, `defmt` RTT logs, and remote GDB server).
