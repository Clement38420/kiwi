# Guides & Workflows (Step-by-Step Recipes)

This document details the exact procedures for extending robot features across all 4 packages (`shared` $\leftrightarrow$ `kiwi-mcu` $\leftrightarrow$ `kiwi-rpi` $\leftrightarrow$ `gcs`).

---

## 1. Add a Monitored Telemetry State (`Watch`: MCU $\rightarrow$ RPi $\rightarrow$ GCS)

Use this workflow when a persistent state or measurement on the MCU (e.g., temperature, actuator status, operating mode) must be:
1. Automatically pushed to the GCS whenever it changes.
2. Retrieved immediately when the GCS connects via the initial `GetState` RPC request.

### Step 1: Declare the type and messages in `crates/shared/src/messages.rs`
1. If introducing a new `struct` or `enum`, derive:
   `#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]` and `#[cfg_attr(feature = "ts", derive(TS), ts(export))]`.
   *(Note: `PartialEq` and `Copy` are required by the MCU's `update_watch` helper).*
2. Add the field to [`RobotState`](../../crates/shared/src/messages.rs#L33-L37) (for `GetState` initialization).
3. Add the variant to [`McuToRpi`](../../crates/shared/src/messages.rs#L40-L45) and [`RpiToGcs`](../../crates/shared/src/messages.rs#L50-L56).
4. Map the conversion in `impl TryFrom<McuToRpi> for RpiToGcs` so `kiwi-rpi` automatically forwards the message to the GCS.

### Step 2: Create the `Watch` in `crates/kiwi-mcu/src/channels.rs`
1. Define the constant for the exact number of subscribed MCU tasks:
   `const MY_STATE_RECEIVERS: usize = N;` (use at least `1` even if only `get_my_state()` is called, as Embassy `Watch` capacity must be $\ge 1$).
2. Declare the private `static` `Watch` initialized with a default value:
   `static MY_STATE: Watch<ThreadModeRawMutex, MyType, MY_STATE_RECEIVERS> = Watch::new_with(default_value);`
3. Implement the public helper functions:
   - `pub async fn update_my_state(new_state: MyType)`: calls `if update_watch(&MY_STATE, new_state) { UART_TX.send(McuToRpi::MyState(new_state)).await; }`.
   - `pub fn subscribe_my_state() -> watch::Receiver<...>` (if local MCU tasks need `.changed().await`).
   - `pub fn get_my_state() -> MyType`: returns `MY_STATE.try_get().unwrap()`.

### Step 3: Update `GetState` in `crates/kiwi-mcu/src/tasks/communication.rs`
In [`get_state_handler`](../../crates/kiwi-mcu/src/tasks/communication.rs#L28-L39), populate the new `RobotState` field by calling `get_my_state()`.

### Step 4: Update `crates/kiwi-rpi/src/tasks/mcu.rs`
In the receive loop of [`mcu_task`](../../crates/kiwi-rpi/src/tasks/mcu.rs#L36-L41), add `McuToRpi::MyState(_)` to the `match &message` statement. The `RpiToGcs::try_from(message)` below it will forward the update to `gcs_tx`.

### Step 5: Regenerate TypeScript bindings and update `gcs`
1. From `crates/shared/`, run:
   ```sh
   TS_RS_EXPORT_DIR=../../gcs/src/bindings cargo test --features ts
   ```
2. In [`gcs/src/stores/robot.ts`](../../gcs/src/stores/robot.ts):
   - Add the initial default field to `robotState`.
   - Add the `else if ('MyState' in message)` branch in `dispatchMessage` to update `robotState.value.my_state`.

---

## 2. Add a One-Way Command (GCS or RPi $\rightarrow$ MCU)

Use this workflow to send an order without expecting a response (e.g., velocity setpoint, actuator toggle).

### Step 1: Declare the message in `crates/shared/src/messages.rs`
1. Add the variant to [`RpiToMcu`](../../crates/shared/src/messages.rs#L93-L97).
2. If the command can be triggered from the Web UI:
   - Also add the variant to [`GcsToRpi`](../../crates/shared/src/messages.rs#L75-L78).
   - Map the conversion in `impl TryFrom<GcsToRpi> for RpiToMcu`.
   - If the command is only sent by an algorithm running on the RPi (like `WheelSpeedSetpoints`), only add it to `RpiToMcu`.

### Step 2: Create the synchronization primitive in `crates/kiwi-mcu/src/channels.rs`
- **If only the latest value matters** (e.g., speed setpoint, state toggle): declare `pub static MY_CMD: Signal<ThreadModeRawMutex, MyType> = Signal::new();`.
- **If every command must be processed sequentially without loss**: declare `pub static MY_CMD: Channel<ThreadModeRawMutex, MyType, N> = Channel::new();`.

### Step 3: Route the command in `crates/kiwi-mcu/src/tasks/communication.rs`
In [`message_dispatcher`](../../crates/kiwi-mcu/src/tasks/communication.rs#L9-L18), add `RpiToMcu::MyCmd(val)` to the `match` statement and call `MY_CMD.signal(val)` (or `MY_CMD.send(val).await`).

### Step 4: Handle on RPi and GCS (if exposed to GCS)
1. In [`crates/kiwi-rpi/src/tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs#L78-L86), add `GcsToRpi::MyCmd(_)` to `match &message`. `RpiToMcu::try_from(message)` will automatically forward it to `mcu_sending_queue`.
2. Regenerate TS bindings (`TS_RS_EXPORT_DIR=../../gcs/src/bindings cargo test --features ts`).
3. Call `robotStore.sendMessage({ MyCmd: value })` from a Vue component.

---

## 3. Add an RPC Request (GCS $\rightarrow$ RPi/MCU $\rightarrow$ GCS)

Use this workflow when a call from the GCS requires an explicit response, return value, or timeout handling.

### Step 1: Follow the `Req` / `Res` convention in `crates/shared/src/messages.rs`
For an RPC named `<Action>` taking `ReqPayload` (use `()` if no argument) and returning `ResPayload`:
1. In `GcsToRpi`: add `<Action>Req(RpcRequest<ReqPayload>)`.
2. In `RpiToGcs`: add `<Action>Res(RpcResponse<ResPayload>)`.
3. **If the RPC is handled by the MCU** (like `GetState`):
   - Also add `<Action>Req(RpcRequest<ReqPayload>)` to `RpiToMcu` + its conversion in `TryFrom<GcsToRpi> for RpiToMcu`.
   - Also add `<Action>Res(RpcResponse<ResPayload>)` to `McuToRpi` + its conversion in `TryFrom<McuToRpi> for RpiToGcs`.
4. **If the RPC is handled locally by the RPi**:
   - Do not add it to `RpiToMcu` or `McuToRpi`.

### Step 2: Implement the handler
- **On the MCU** (in [`crates/kiwi-mcu/src/tasks/communication.rs`](../../crates/kiwi-mcu/src/tasks/communication.rs)):
  Match `RpiToMcu::<Action>Req(req)` and reply via:
  `UART_TX.send(McuToRpi::<Action>Res(RpcResponse::new(&req, payload))).await;`
- **On the RPi** (in [`crates/kiwi-rpi/src/tasks/gcs.rs`](../../crates/kiwi-rpi/src/tasks/gcs.rs)):
  Match `GcsToRpi::<Action>Req(req)` and reply via:
  `sending_queue_tx.send(RpiToGcs::<Action>Res(RpcResponse::new(&req, payload)));`

### Step 3: Export and call from GCS
1. Export TS bindings (`TS_RS_EXPORT_DIR=../../gcs/src/bindings cargo test --features ts`).
2. Thanks to type introspection in [`gcs/src/utils/rpc.ts`](../../gcs/src/utils/rpc.ts), no changes to the RPC client or `dispatchMessage` are needed: `rpc.handleMessage` automatically intercepts any message ending in `Res`.
3. Call `const res = await robotStore.sendRequest('<Action>', arg)` (or without `arg` if `ReqPayload` is `()`).

---

## 4. Add a Hardware Peripheral on the STM32 (`kiwi-mcu`)

1. **Check the pinout** in [`docs/kiwi/hardware.md`](hardware.md) and in the vendor manuals (`docs/ST/um-stm32-nucleo144.pdf` and `docs/ST/rm-stm32f76xxx.pdf`) to avoid pin, DMA, or timer conflicts (`TIM2`, `TIM3`, `TIM12`, `ADC1`, `USART1` are already in use).
2. **Create a driver** in `crates/kiwi-mcu/src/drivers/` if it is a reusable hardware abstraction. Drivers must not hardcode specific pins; they should accept generic or pre-configured HAL peripheral instances.
3. **Instantiate in [`crates/kiwi-mcu/src/board.rs`](../../crates/kiwi-mcu/src/board.rs)**:
   - Update the module-level pinout table in `board.rs`.
   - If the peripheral requires an interrupt, bind it inside `bind_interrupts!`.
   - Add the field to `Board` and initialize it in `Board::init`.
4. **Create or update the task** in `crates/kiwi-mcu/src/tasks/` and pass the `b.<peripheral>` field from [`main.rs`](../../crates/kiwi-mcu/src/main.rs).
