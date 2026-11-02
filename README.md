# Kiwi — Autonomous Mobile Robot

DIY 3-wheel holonomic autonomous mobile robot, built from scratch in **Rust** and **TypeScript** without external robotics frameworks (no ROS, no third-party middleware).

## Project Structure

The repository is split into 4 independent packages (intentionally without a root Cargo Workspace, as each target uses its own compilation target, linker, and deployment runner):

| Package | Path | Target | Role |
| :--- | :--- | :--- | :--- |
| **Shared** | [`crates/shared/`](crates/shared/) | `no_std` / `std` / `ts` | Communication protocol, shared types, RPC primitives, COBS+CRC16 framing, PID controller. |
| **MCU** | [`crates/kiwi-mcu/`](crates/kiwi-mcu/) | `thumbv7em-none-eabihf` (STM32F767ZI) | Real-time firmware (`embassy` in `no_std`): 3-wheel closed-loop control, battery management, power safety, UART link. |
| **RPi** | [`crates/kiwi-rpi/`](crates/kiwi-rpi/) | `aarch64-unknown-linux-gnu` (Raspberry Pi 4B) | Compute unit (`tokio`): LiDAR acquisition, MCU serial bridge, HTTP/WebSocket server (`axum` + `rust-embed`). |
| **GCS** | [`gcs/`](gcs/) | Web Browser (Vue 3 / TypeScript) | Ground Control Station: real-time telemetry, LiDAR map, commands, and RPC requests over WebSocket (MessagePack). |

## Documentation

All technical documentation for the robot is located in [`docs/kiwi/`](docs/kiwi/):

* [**Architecture & Protocol**](docs/kiwi/architecture.md) — Hardware/software topology, architectural decisions (end-to-end typing, no Cargo workspace), serial framing (Postcard + CRC16 + COBS), WebSocket (MessagePack), and RPC system.
* [**STM32 Firmware (`kiwi-mcu`)**](docs/kiwi/mcu.md) — `embassy` architecture, separation of `board` / `drivers` / `tasks` / `channels`, and strict usage rules for synchronization primitives (`Watch`, `Signal`, `Channel`).
* [**Compute & Web UI (`kiwi-rpi` & `gcs`)**](docs/kiwi/rpi-and-gcs.md) — Tokio tasks and `broadcast` channels on Raspberry Pi, embedded frontend build, Pinia store, and TypeScript RPC client.
* [**Guides & Workflows**](docs/kiwi/workflows.md) — Step-by-step recipes for adding telemetry (`Watch`), commands (`Signal`/`Channel`), RPC calls, or STM32 peripherals end-to-end.
* [**Hardware & Pinout**](docs/kiwi/hardware.md) — Complete NUCLEO-F767ZI wiring, timers, BLM3650 motors, encoders, LiDAR, and 6S LiFePO4 battery thresholds.
* [**Workshop & Inventory**](docs/kiwi/atelier.md) — Available tools, consumables, and spare parts inventory.

Vendor documentation (STM32 and LiDAR datasheets/reference manuals) is stored locally in `docs/ST/` and `docs/Lidar/` (gitignored).
