# Raspberry Pi Compute (`kiwi-rpi`) & Web UI (`gcs`)

## 1. Raspberry Pi Daemon (`crates/kiwi-rpi`)

`kiwi-rpi` is an asynchronous **Tokio** binary cross-compiled for `aarch64-unknown-linux-gnu` and running as a `systemd` service (`kiwi.service`) on the Raspberry Pi 4B (`kiwi.local`).

### Integrated Build Pipeline (`build.rs`)
During `cargo build`, [`crates/kiwi-rpi/build.rs`](../../crates/kiwi-rpi/build.rs) automatically runs `pnpm --dir ../../gcs build-only` whenever files in `gcs/` change. The resulting `gcs/dist/` folder is embedded directly into the Rust binary via `#[derive(RustEmbed)]` in [`tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs#L21-L23). A single standalone binary therefore serves both the robotics backend and the web frontend.

### Shared State & Channel Architecture in `main.rs`
Unlike the MCU which uses `static` variables (`no_std`), [`crates/kiwi-rpi/src/main.rs`](../../crates/kiwi-rpi/src/main.rs) instantiates shared state and Tokio channels in `main()` and passes cloned handles to tasks:

- **`robot_state` (`Arc<RwLock<RobotState>>`)**:
  In-memory digital twin of the robot's persistent state (`RobotState`, containing `mcu: McuState`). Updated by `mcu_task` whenever the STM32 reports a state change, and read directly by `gcs_task` to immediately answer `GetStateReq` RPC requests without round-tripping over UART.
- **`gcs_tx` (`broadcast::Sender<RpiToGcs>`, capacity 64)**:
  Outbound stream to connected WebSocket clients. Using a `broadcast` channel allows multiple open browser tabs/clients to simultaneously receive `RobotState` updates and LiDAR scans.
- **`mcu_tx` (`broadcast::Sender<RpiToMcu>`, capacity 64)**:
  Outbound command stream to the STM32 (fed by the GCS or future RPi autonomy tasks).
- **`lidar_tx` (`watch::Sender<Arc<Vec<LidarPoint>>>`)**:
  Holds the latest full 360° LiDAR scan in shared memory (`Arc`) for local RPi tasks (SLAM, obstacle avoidance) without cloning the point vector.

### Tasks and Drivers (`tasks/` & `drivers/`)

1. **MCU Task ([`tasks/mcu.rs`](../../crates/kiwi-rpi/src/tasks/mcu.rs)) & Driver ([`drivers/mcu.rs`](../../crates/kiwi-rpi/src/drivers/mcu.rs))**:
   - Opens `/dev/serial0` at `115200` baud (automatically retrying every 5 seconds on open failure or serial disconnect).
   - Immediately sends `RpiToMcu::SyncState` upon opening the serial port so the STM32 replies with its full `McuState`.
   - Runs a bidirectional `tokio::select!` loop:
     - **RX (`mcu.receive_message()`)**: Reads the serial stream up to `0x00` (`read_until(0x00)`), strips the delimiter, and decodes via `shared::messages::decode`. State messages (`BatteryVoltage`, `BatteryLevel`, `ArmState`, `McuState`) mutate `robot_state` via `update_state` and broadcast the updated `RpiToGcs::RobotState` on `gcs_tx`. Other messages are forwarded to `gcs_tx` if `RpiToGcs::try_from(message)` succeeds.
     - **TX (`sending_queue.recv()`)**: Encodes incoming `RpiToMcu` messages with `shared::messages::encode` and writes them to the serial port.

2. **LiDAR Task ([`tasks/lidar.rs`](../../crates/kiwi-rpi/src/tasks/lidar.rs)) & Driver ([`drivers/lidar.rs`](../../crates/kiwi-rpi/src/drivers/lidar.rs))**:
   - Drives the **YDLidar G6** (or YDLidar-compatible scanner) on `/dev/ttyUSB0` at `512000` baud.
   - Sends `START_SCAN_CMD` (`[0xA5, 0x60]`), waits 1 s for motor stabilization, flushes the input buffer, and synchronizes on a start packet (`ScanPacketType::Start`).
   - On every full revolution (`read_scan_full_revolution`), verifies the 16-bit XOR checksum of each packet, applies geometric angle correction, wraps the point cloud in an `Arc<Vec<LidarPoint>>`, and publishes it to both `lidar_tx` (internal RPi) and `gcs_tx` (`RpiToGcs::LidarScan`).
   - Implements `Drop` on `Lidar` to send `STOP_SCAN_CMD` (`[0xA5, 0x65]`) on shutdown.

3. **GCS Task ([`tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs))**:
   - **Axum** HTTP/WebSocket server listening on `0.0.0.0:8080` with shared `WsState` (`ws_sending_queue`, `mcu_sending_queue`, `robot_state`).
   - `/ws` route: WebSocket upgrade. Each connected client subscribes to `gcs_tx` (`sending_queue_tx.subscribe()`) and runs a `tokio::select!` loop:
     - Serializes outgoing `RpiToGcs` messages with `rmp_serde::to_vec_named` and sends them as binary WebSocket frames via `ws_send`.
     - Deserializes incoming binary frames into `GcsToRpi` (`rmp_serde::from_slice`). `GetStateReq` RPC requests are answered immediately on the client's socket (`ws_send`) by reading `robot_state`. Messages convertible via `RpiToMcu::try_from(message)` are published to `mcu_sending_queue`.
   - HTTP fallback: Serves embedded static files from `gcs/dist/` (falling back to `index.html` for SPA routing).

---

## 2. Ground Control Station (`gcs/`)

**Vue 3** (`<script setup lang="ts">`), **Pinia**, **Tailwind CSS v4**, and **Vite** single-page application.

### Central Store: [`gcs/src/stores/robot.ts`](../../gcs/src/stores/robot.ts)
`useRobotStore` manages the WebSocket lifecycle, RPC client, and reactive robot state:
- **Automatic WebSocket Connection**:
  - In development (`import.meta.env.DEV`), connects to `ws://kiwi.local:8080/ws` (or through the Vite dev proxy). In production (served by `kiwi-rpi`), connects to `/ws`.
  - On close (`onclose`), rejects all pending RPC requests (`rpc.rejectAll('ConnectionClosed')`) and schedules an automatic reconnect after `2000 ms`.
- **Initial State Synchronization (`GetState`)**:
  - Immediately upon WebSocket connection (`onopen`), the store executes `robotState.value = await rpc.request('GetState')`.
  - The RPi replies immediately from its in-memory `robot_state` (`battery_voltage`, `battery_level`, `arm`).
- **Exhaustive Message Dispatch (`dispatchMessage`)**:
  - First passes incoming messages to `rpc.handleMessage(message)`, which intercepts any variant ending in `Res` (e.g., `GetStateRes`).
  - Then branches on each stream variant (`LidarScan`, `RobotState`).
  - Enforces compile-time exhaustiveness (`const _exhaustiveCheck: never = message`): **if a new variant is added to `RpiToGcs` in Rust and exported via `ts-rs`, TypeScript compilation will fail until `dispatchMessage` handles it.**
- **Reactivity Optimization**:
  - `lidarScan` uses `shallowRef<LidarPoint[]>([])` instead of `ref()` to avoid deep reactive proxy overhead across hundreds of points on every LiDAR revolution.

### Key Components & Utilities
- **[`gcs/src/utils/rpc.ts`](../../gcs/src/utils/rpc.ts)**: Strongly typed generic RPC client. Attaches a random `u32` ID (`crypto.getRandomValues`) to each `${name}Req` call and resolves the promise when the matching `${name}Res` arrives.
- **[`gcs/src/composables/useViewport.ts`](../../gcs/src/composables/useViewport.ts)**: Composable managing an interactive 2D Canvas with click-and-drag panning, cursor-centered wheel zoom, automatic resizing (`ResizeObserver`), adaptive logarithmic grid (`m/div`), and light/dark theme reactivity (`MutationObserver`).
- **[`gcs/src/components/modules/ModuleHealth.vue`](../../gcs/src/components/modules/ModuleHealth.vue)**: Displays WebSocket connection status, battery voltage/gauge, and the `Arm` / `Disarm` control button.
- **[`gcs/src/components/modules/ModuleLidarMap.vue`](../../gcs/src/components/modules/ModuleLidarMap.vue)**: Renders the real-time LiDAR point cloud on the `useViewport` canvas.
