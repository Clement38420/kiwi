# Raspberry Pi Compute (`kiwi-rpi`) & Web UI (`gcs`)

## 1. Raspberry Pi Daemon (`crates/kiwi-rpi`)

`kiwi-rpi` is an asynchronous **Tokio** binary cross-compiled for `aarch64-unknown-linux-gnu` and running as a `systemd`
service (`kiwi.service`) on the Raspberry Pi 4B (`kiwi.local`).

### Integrated Build Pipeline (`build.rs`)

During `cargo build`, [`crates/kiwi-rpi/build.rs`](../../crates/kiwi-rpi/build.rs) automatically runs
`pnpm --dir ../../gcs build-only` whenever files in `gcs/` change. The resulting `gcs/dist/` folder is embedded directly
into the Rust binary via `#[derive(RustEmbed)]` in [`tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs#L21-L23). A
single standalone binary therefore serves both the robotics backend and the web frontend.

### Shared State & Channel Architecture in `main.rs`

Unlike the MCU which uses `static` variables (`no_std`), [
`crates/kiwi-rpi/src/main.rs`](../../crates/kiwi-rpi/src/main.rs) instantiates shared state and Tokio channels in
`main()` and passes cloned handles to tasks:

- **`robot_state_tx` / `robot_state_rx` (`watch::channel(RobotState::default())`)**:
  In-memory digital twin of the robot's persistent state (`RobotState`, containing `mcu: McuState`). Updated by
  `mcu_task` via `send_modify` whenever the STM32 reports a state change, and read directly by `gcs_task` to immediately
  answer `GetStateReq` RPC requests without round-tripping over UART. A dedicated forwarding task in `main.rs` listens to
  `robot_state_rx.changed()` and broadcasts `RpiToGcs::RobotState` to connected WebSocket clients.
- **`gcs_tx` (`broadcast::Sender<RpiToGcs>`, capacity 64)**:
  Outbound stream to connected WebSocket clients. Using a `broadcast` channel allows multiple open browser tabs/clients
  to simultaneously receive `RobotState`, `RobotPose`, and `LidarScan` updates.
- **`mcu_tx` (`broadcast::Sender<RpiToMcu>`, capacity 64)**:
  Outbound command stream to the STM32 (fed by the GCS or RPi autonomy tasks).
- **`lidar_tx` / `lidar_rx` (`watch::channel(Arc::new(Vec::<LidarPoint>::new()))`)**:
  Holds the latest full 360° LiDAR scan in shared memory (`Arc`) for local RPi tasks (`odometry_task`) without cloning
  the point vector. A dedicated forwarding task in `main.rs` listens to `lidar_rx.changed()` and broadcasts
  `RpiToGcs::LidarScan` to `gcs_tx`.
- **`odom_tx` / `odom_rx` (`watch::channel(RobotPose::default())`)**:
  Holds the latest 50 Hz wheel and IMU odometry pose (`McuToRpi::Odom`) received from the STM32. Consumed by
  `odometry_task` both for real-time 50 Hz heading (`theta`) updates and as the 2D motion prior for LiDAR scan matching.
- **`reset_tx` (`broadcast::Sender<()>`, capacity 16)**:
  Internal synchronization channel triggered when the STM32 confirms a pose reset via `McuToRpi::ResetAck`. Ensures the
  RPi resets its LiDAR keyframe and `robot_pose_tx` only after the MCU has zeroed its own odometry.
- **`robot_pose_tx` / `robot_pose_rx` (`watch::channel(RobotPose::default())`)**:
  Holds the latest estimated 2D world pose (`RobotPose { x, y, theta }`). A dedicated forwarding task in `main.rs`
  listens to `robot_pose_rx.changed()` and automatically broadcasts `RpiToGcs::RobotPose` to `gcs_tx`.

### Tasks and Drivers (`tasks/` & `drivers/`)

1. **MCU Task ([`tasks/mcu.rs`](../../crates/kiwi-rpi/src/tasks/mcu.rs)) & Driver ([
   `drivers/mcu.rs`](../../crates/kiwi-rpi/src/drivers/mcu.rs))**:
    - Opens `/dev/serial0` at `115200` baud (automatically retrying every 5 seconds on open failure or serial
      disconnect).
    - Immediately sends `RpiToMcu::Sync` upon opening the serial port so the STM32 replies with its full `McuState` and
      current `Odom`.
    - Runs a bidirectional `tokio::select!` loop:
        - **RX (`mcu.receive_message()`)**: Reads the serial stream up to `0x00` (`read_until(0x00)`), strips the
          delimiter, and decodes via `shared::messages::decode`:
            - `BatteryVoltage`, `BatteryLevel`, `ArmState`, `McuState` mutate `robot_state_tx` via `send_modify`.
            - `Odom(v)` updates `odom_tx` via `send_replace`.
            - `ResetAck` zeroes `odom_tx` (`RobotPose::default()`) and notifies `odometry_task` via `reset_tx.send(())`.
            - Any message convertible via `RpiToGcs::try_from(message)` is forwarded to `gcs_tx`.
        - **TX (`sending_queue.recv()`)**: Encodes incoming `RpiToMcu` messages with `shared::messages::encode` and
          writes them to the serial port.

2. **LiDAR Task ([`tasks/lidar.rs`](../../crates/kiwi-rpi/src/tasks/lidar.rs)) & Driver ([
   `drivers/lidar.rs`](../../crates/kiwi-rpi/src/drivers/lidar.rs))**:
    - Drives the **YDLidar G6** (or YDLidar-compatible scanner) on `/dev/ttyUSB0` at `512000` baud.
    - Normalizes physical units at the driver level (`compute_data_point`): distances are converted from millimeters to
      meters (`f32` in meters), and angles from degrees to radians (`angle.to_radians()`).
    - Sends `START_SCAN_CMD` (`[0xA5, 0x60]`), waits 1 s for motor stabilization, flushes the input buffer, and
      synchronizes on a start packet (`ScanPacketType::Start`).
    - On every full revolution (`read_scan_full_revolution`), verifies the 16-bit XOR checksum of each packet, applies
      geometric angle correction, wraps the point cloud in an `Arc<Vec<LidarPoint>>`, and publishes it to `lidar_tx`
      (`send_replace`).
    - Implements `Drop` on `Lidar` to send `STOP_SCAN_CMD` (`[0xA5, 0x65]`) on shutdown.

3. **LiDAR Odometry Task ([`tasks/odometry.rs`](../../crates/kiwi-rpi/src/tasks/odometry.rs))**:
    - Fuses 360° LiDAR scans (`lidar_rx`, ~10 Hz) with MCU wheel + IMU odometry (`odom_rx`, 50 Hz) and reset events
      (`reset_rx`):
    - **Preprocessing (`preprocess_scan`)**: Filters valid range measurements ($0.05\,\text{m} < d < 16.0\,\text{m}$)
      and projects polar coordinates into 2D Cartesian points in the robot's direct right-hand frame ($+X$ forward, $+Y$
      left, $+Z$ up): $x = r \cos\theta, y = -r \sin\theta$ (accounting for the clockwise rotation of the LiDAR).
    - **50 Hz Heading Propagation**: Whenever `odom_rx.changed()` fires between LiDAR scans, `robot_pose_tx` is updated
      immediately with `current_odom.theta` so the GCS receives low-latency 50 Hz orientation updates.
    - **Keyframe-Based 2D ICP (`Keyframe::align`)**:
        - Rather than matching consecutive scans (which accumulates drift on every frame), each incoming scan is aligned
          against a reference **`Keyframe`** holding a static 2D $k$-d tree (`KdTree2::build_by_ordered_float`), its
          world `pose`, and the MCU `odom` snapshot at keyframe creation.
        - **MCU Odometry Prior**: Computes the relative displacement $(\Delta x_{\text{prior}}, \Delta y_{\text{prior}}, \Delta\theta_{\text{prior}})$
          measured by the MCU since `keyframe.odom` (rotated into the keyframe's local frame) and pre-transforms the
          incoming scan before starting ICP.
        - **Adaptive Distance Threshold**: Iterates up to `ICP_MAX_ITERATIONS = 10` with a coarse-to-fine association
          radius $d_{\text{max}}(k) = \max(0.30 \times 0.75^k,\, 0.04)\,\text{m}$ and a 1-to-1 closest-point filter per
          keyframe tree node.
        - **SVD / Cross-Correlation Alignment**: Computes incremental $(\Delta x, \Delta y, \Delta\theta)$ via centroids
          and 2D cross-covariance ($b.\text{atan2}(a)$) until convergence ($\Delta t^2 < 0.0005^2\,\text{m}^2$ and
          $|\Delta\theta| < 0.0002\,\text{rad}$).
    - **World Pose Update & Keyframe Spawning**:
        - Projects the converged local displacement `(icp.dx, icp.dy)` from `keyframe.pose` into world coordinates
          `(world_x, world_y)` while taking `theta = current_odom.theta` directly from the IMU.
        - Spawns a new `Keyframe` (`should_replace`) when translation from the keyframe exceeds `15 cm`
          (`KEYFRAME_T_THRESHOLD`), rotation exceeds `~20°` (`KEYFRAME_R_THRESHOLD = 0.35 rad`), or the inlier match
          ratio drops below `60%` (`KEYFRAME_MATCHES_THRESHOLD`).
        - If ICP fails (`< 10` matched points) on a valid scan (`>= 10` points), re-anchors a new keyframe at the
          current pose to recover tracking automatically.
    - **Synchronized Reset**: On `reset_rx.recv()`, re-initializes the `Keyframe` with the current LiDAR scan at
      `RobotPose::default()` and zeroes `robot_pose_tx`.

4. **GCS Task ([`tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs))**:
    - **Axum** HTTP/WebSocket server listening on `0.0.0.0:8080` with shared `WsState` (`ws_sending_queue`,
      `mcu_sending_queue`, `robot_state_tx`).
    - `/ws` route: WebSocket upgrade. Each connected client subscribes to `gcs_tx` (`sending_queue_tx.subscribe()`) and
      runs a `tokio::select!` loop:
        - Serializes outgoing `RpiToGcs` messages with `rmp_serde::to_vec_named` and sends them as binary WebSocket
          frames via `ws_send`.
        - Deserializes incoming binary frames into `GcsToRpi` (`rmp_serde::from_slice`):
            - `GetStateReq(req)`: Answered immediately on the client's socket (`ws_send`) by reading `robot_state_tx`.
            - `Twist(t)`: Converts the body velocity command `(vx, vy, w)` into individual wheel speed setpoints via
              `t.to_wheel_speeds()` and publishes `RpiToMcu::WheelSpeedSetpoints` to `mcu_sending_queue`.
            - `Arm(v)` and `Reset`: Converted via `RpiToMcu::try_from(message)` and published to `mcu_sending_queue`.
    - HTTP fallback: Serves embedded static files from `gcs/dist/` (falling back to `index.html` for SPA routing).

---

## 2. Ground Control Station (`gcs/`)

**Vue 3** (`<script setup lang="ts">`), **Pinia**, **Tailwind CSS v4**, and **Vite** single-page application.

### Central Store: [`gcs/src/stores/robot.ts`](../../gcs/src/stores/robot.ts)

`useRobotStore` manages the WebSocket lifecycle, RPC client, and reactive robot state:

- **Automatic WebSocket Connection**:
    - In development (`import.meta.env.DEV`), connects to `ws://kiwi.local:8080/ws` (or through the Vite dev proxy). In
      production (served by `kiwi-rpi`), connects to `/ws`.
    - On close (`onclose`), rejects all pending RPC requests (`rpc.rejectAll('ConnectionClosed')`) and schedules an
      automatic reconnect after `2000 ms`.
- **Initial State Synchronization (`GetState`)**:
    - Immediately upon WebSocket connection (`onopen`), the store executes
      `robotState.value = await rpc.request('GetState')`.
    - The RPi replies immediately from its in-memory `robot_state` (`battery_voltage`, `battery_level`, `arm`).
- **Exhaustive Message Dispatch (`dispatchMessage`)**:
    - First passes incoming messages to `rpc.handleMessage(message)`, which intercepts any variant ending in `Res`
      (e.g., `GetStateRes`).
    - Then branches on each stream variant (`LidarScan`, `RobotState`, `RobotPose`).
    - Enforces compile-time exhaustiveness (`const _exhaustiveCheck: never = message`): **if a new variant is added to
      `RpiToGcs` in Rust and exported via `ts-rs`, TypeScript compilation will fail until `dispatchMessage` handles
      it.**
- **Reactivity Optimization**:
    - `lidarScan` uses `shallowRef<LidarPoint[]>([])` instead of `ref()` to avoid deep reactive proxy overhead across
      hundreds of points on every LiDAR revolution.
    - `robotPose` uses `ref<RobotPose>({ x: 0, y: 0, theta: 0 })`.

### Key Components & Utilities

- **[`gcs/src/utils/rpc.ts`](../../gcs/src/utils/rpc.ts)**: Strongly typed generic RPC client. Attaches a random `u32`
  ID (`crypto.getRandomValues`) to each `${name}Req` call and resolves the promise when the matching `${name}Res`
  arrives.
- **[`gcs/src/composables/useViewport.ts`](../../gcs/src/composables/useViewport.ts)**: Composable managing an
  interactive 2D Canvas with click-and-drag panning, cursor-centered wheel zoom, automatic resizing (`ResizeObserver`),
  adaptive logarithmic grid (`m/div`), and light/dark theme reactivity (`MutationObserver`).
- **[`gcs/src/composables/useJoystick.ts`](../../gcs/src/composables/useJoystick.ts)**: Pointer-capture composable
  (`pointerdown` / `pointermove` / `pointerup`) tracking 2D drag displacement clamped to a configurable `maxRadius`
  (`60 px`), exposing normalized coordinates $\in [-1, 1]$, drag state, and smooth spring-back CSS transitions.
- **[`gcs/src/components/modules/ModuleControl.vue`](../../gcs/src/components/modules/ModuleControl.vue)**: Manual
  teleoperation module combining a 2D virtual joystick for holonomic linear velocities ($V_x, V_y \in [-1.0, 1.0]\,\text{m/s}$)
  and a horizontal slider for angular velocity ($\omega \in [-\pi, \pi]\,\text{rad/s}$). Sends `GcsToRpi::Twist` on
  change and sustains the command every `250 ms` while dragged so the MCU's `1000 ms` setpoint watchdog stays active,
  automatically sending a zero twist on release.
- **[`gcs/src/components/modules/ModuleHealth.vue`](../../gcs/src/components/modules/ModuleHealth.vue)**: Displays
  WebSocket connection status, battery voltage/gauge, current estimated pose ($X$, $Y$ in meters, $\theta$ in degrees),
  the `Arm` / `Disarm` control button (styled with semantic state colors: `crit` when armed, `ok` when disarmed), and
  the `Reset` pose button.
- **[`gcs/src/components/modules/ModuleLidarMap.vue`](../../gcs/src/components/modules/ModuleLidarMap.vue)**: Renders
  the real-time LiDAR point cloud and robot marker on the `useViewport` canvas.
- **[`gcs/src/App.vue`](../../gcs/src/App.vue)**: Responsive CSS Grid dashboard layout with adaptive areas (`health`,
  `lidar`, `control`) transitioning smoothly between mobile, tablet (768px+), and desktop (1280px+) screens.
