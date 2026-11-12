# System Architecture & Protocol

## 1. System Overview

Kiwi uses a 3-tier distributed architecture connected by end-to-end strongly typed binary protocols:

```
+-----------------------+          UART 115200 baud          +-----------------------+         WebSocket (:8080/ws)        +-----------------------+
|       kiwi-mcu        | <================================> |       kiwi-rpi        | <=================================> |          gcs          |
|  STM32F767ZI (no_std) |   Postcard + CRC16-MODBUS + COBS   |  Raspberry Pi 4 (std) |             MessagePack             |   Vue 3 + TypeScript  |
+-----------------------+          ( 0x00 delimiter )        +-----------------------+           ( named binary )          +-----------------------+
  - 3x BLM3650 Motors                                          - LiDAR (/dev/ttyUSB0)                                        - Pinia Store (robot)
  - 3x TIM2 Encoders                                           - Keyframe LiDAR Odometry (2D ICP)                            - Typed RPC Client
  - MPU-6050 IMU (I2C2)                                        - MCU Bridge (/dev/serial0)                                   - Canvas LiDAR Map
  - Wheel & Heading Odometry                                   - Axum HTTP/WS Server                                         - Virtual Joystick & Slider
  - ADC1 Battery & Power Safety                                - Embedded Web Assets (rust-embed)                            - Pose & Health Display
```

---

## 2. Key Architectural Decisions

### End-to-End Type Continuity (Rust $\rightarrow$ TypeScript)
All data structures and message enums are defined **once** in [`crates/shared/src/messages.rs`](../../crates/shared/src/messages.rs), [`crates/shared/src/geometry/`](../../crates/shared/src/geometry/mod.rs), and [`crates/shared/src/rpc.rs`](../../crates/shared/src/rpc.rs):
1. **`kiwi-mcu`** depends on `shared` with default features disabled (`#![no_std]`).
2. **`kiwi-rpi`** depends on `shared` with the `std` feature enabled (unlocking RPi $\leftrightarrow$ GCS messages that use heap allocations or `std` math like `Arc<Vec<LidarPoint>>` and `Twist::to_wheel_speeds()`, as well as random ID generation for RPC requests).
3. **`gcs`** consumes TypeScript definitions automatically generated from `shared` via the `ts` feature (`ts-rs`) in [`gcs/src/bindings/`](../../gcs/src/bindings/).

This guarantees at compile time that schema drift (missing fields, renamed enum variants) cannot happen between the microcontroller, the Raspberry Pi, and the Web UI.

### Why There Is No Cargo Workspace
The three Rust crates (`shared`, `kiwi-mcu`, `kiwi-rpi`) are **not** grouped into a root `Cargo.toml` workspace:
- `kiwi-mcu` compiles to `thumbv7em-none-eabihf` (`no_std`, `link.x` and `defmt.x` linker scripts, `remote-run.sh` runner via `probe-rs`).
- `kiwi-rpi` cross-compiles to `aarch64-unknown-linux-gnu` (`std`, `aarch64-unknown-linux-gnu-gcc` linker, `deploy.sh` runner via `rsync`/`systemd`).
- A unified root Cargo workspace enforces shared feature resolution and prevents per-crate `.cargo/config.toml` default `[build] target = ...` settings. Each crate therefore keeps its own `.cargo/config.toml` and `target/` directory.

---

## 3. The `shared` Crate

### Feature Flags (`crates/shared/Cargo.toml`)
- `default = []`: `no_std` mode. Exposes:
  - [`consts`](../../crates/shared/src/consts.rs): Physical constants (`WHEEL_RADIUS = 0.03` m, `WHEEL_BASE = 0.184` m, `MAX_WHEEL_SPEED = 25.0` rad/s).
  - [`geometry`](../../crates/shared/src/geometry/mod.rs): [`Pose2D`](../../crates/shared/src/geometry/pose.rs) (with `+`, `-`, `+=`, `-=`, unary `-` over values and references, and `wrap_angle` normalizing `theta` to $[-\pi, \pi)$), [`Point2D`](../../crates/shared/src/geometry/point.rs) (Cartesian operators and `[f32; 2]` conversions), [`PointPolar`](../../crates/shared/src/geometry/point.rs) (CCW polar coordinates), and [`Twist`](../../crates/shared/src/geometry/twist.rs).
  - [`messages`](../../crates/shared/src/messages.rs): `McuToRpi` (including `Odom(Pose2D)` and `ResetAck`), `RpiToMcu` (`WheelSpeedSetpoints`, `Arm`, `Sync`, `Reset`), `McuState`, `BatteryLevel`, `ArmState`, and `LidarPoint` (with `From<LidarPoint> for PointPolar` converting clockwise sensor angles to CCW).
  - [`codec`](../../crates/shared/src/codec.rs): Static-buffer `&mut [u8]` `encode` / `decode` functions (`COBS` + `CRC-16/MODBUS` + `postcard`).
  - [`pid`](../../crates/shared/src/pid.rs): `PIDController` (with `.update(...)` and `.reset()`).
- `std`: Enables `std`, `Point2D::norm`/`distance_to` and `PointPolar <-> Point2D` trigonometric conversions, `Twist::to_wheel_speeds()` (inverse holonomic kinematics and saturation scaling to `MAX_WHEEL_SPEED`), `RobotState` (flattening `McuState` via `#[serde(flatten)]`), `RpiToGcs` (`LidarScan`, `RobotState`, `GetStateRes`, `RobotPose(Pose2D)`), `GcsToRpi` (`Arm`, `GetStateReq`, `Reset`, `Twist`), `TryFrom<McuToRpi> for RpiToGcs` and `TryFrom<GcsToRpi> for RpiToMcu` conversions, `RpcRequest::new()` (using `rand`), and a `Vec<u8>`-returning `codec::encode` function.
- `ts` (implies `std`): Enables `#[derive(TS)]` and `#[ts(export)]` from `ts-rs` to generate `.ts` binding files (with `#[ts(flatten)]` on `RobotState::mcu` so the GCS sees a flat `RobotState` structure).

### Serial Framing (`MCU <-> RPi`)
Over the UART link (`115200` baud, 8N1), `McuToRpi` and `RpiToMcu` messages go through the following pipeline in [`shared::codec`](../../crates/shared/src/codec.rs):
1. **Compact binary serialization** using `postcard`.
2. **Integrity check**: a `CRC-16/MODBUS` checksum (2 bytes, little-endian) is appended to the Postcard payload.
3. **Zero-byte stuffing**: **COBS** (*Consistent Overhead Byte Stuffing*) encoding over `[payload + crc]`.
4. **Frame delimiter**: a trailing `0x00` byte marks the end of the packet.

On reception (`decode`), the receiver reads bytes up to `0x00` (excluded), performs in-place COBS decoding (`cobs::decode_in_place`), verifies the CRC-16 against the last 2 bytes, and deserializes the payload with `postcard::from_bytes`.

### WebSocket Protocol (`RPi <-> GCS`)
Over the WebSocket connection (`/ws` on port `8080`), `RpiToGcs` and `GcsToRpi` messages are serialized as **named MessagePack**:
- Rust side (`kiwi-rpi`): `rmp_serde::to_vec_named` and `rmp_serde::from_slice`.
- TypeScript side (`gcs`): `encode` and `decode` from `@msgpack/msgpack`.
- Using named serialization (`to_vec_named`) produces objects matching the JSON/TypeScript representation generated by `serde` and `ts-rs` (e.g., `{ "Arm": true }`, `"Reset"`, `{ "Twist": { "vx": 0.5, "vy": 0.0, "w": 0.0 } }`, `{ "RobotPose": { "x": 0.12, "y": -0.05, "theta": 0.45 } }`, or `{ "RobotState": { "battery_voltage": 19.8, "battery_level": "High", "arm": "Disarmed" } }`).

### RPC System (`shared::rpc` & `gcs/src/utils/rpc.ts`)
The project implements an asynchronous request/response mechanism on top of unidirectional message streams:
1. **Mandatory Naming Convention**:
   - A request variant must end with the **`Req`** suffix and wrap `RpcRequest<ReqPayload>` (e.g., `GetStateReq(RpcRequest<()>)`).
   - The matching response variant must share the same prefix, end with **`Res`**, and wrap `RpcResponse<ResPayload>` (e.g., `GetStateRes(RpcResponse<RobotState>)`).
2. **ID Correlation**:
   - `RpcRequest<P>` carries a randomly generated `id: u32` and a `payload: P`.
   - The responder constructs the reply via `RpcResponse::new(&req, response_payload)`, which copies the request's `id`.
3. **Type Inference on the GCS**:
   - In [`gcs/src/utils/rpc.ts`](../../gcs/src/utils/rpc.ts), the TypeScript utility types `RpcRequestName`, `ReqTypeOf<N>`, and `ResTypeOf<N>` automatically introspect the `GcsToRpi` and `RpiToGcs` union types by matching `Req` and `Res` suffixes.
   - Calling `rpc.request('GetState')` in the frontend automatically infers that no payload argument is required (`()`) and returns a `Promise<RobotState>`, with built-in timeout handling (1000 ms by default) and disconnect rejection (`ConnectionClosed`).
