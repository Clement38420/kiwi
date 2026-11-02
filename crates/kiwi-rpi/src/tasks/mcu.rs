use crate::drivers;
use messages::RpiToMcu;
use shared::messages;
use shared::messages::{McuToRpi, RobotState, RpiToGcs};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

const MCU_PORT: &str = "/dev/serial0";

pub async fn mcu_task(
    sending_queue: broadcast::Sender<RpiToMcu>,
    gcs_tx: broadcast::Sender<RpiToGcs>,
    robot_state: Arc<RwLock<RobotState>>,
) {
    let mut sending_queue = sending_queue.subscribe();

    loop {
        let mut mcu = loop {
            let mcu_port_builder = tokio_serial::new(MCU_PORT, 115_200)
                .parity(tokio_serial::Parity::None)
                .stop_bits(tokio_serial::StopBits::One)
                .data_bits(tokio_serial::DataBits::Eight)
                .flow_control(tokio_serial::FlowControl::None);

            match drivers::mcu::Mcu::new(mcu_port_builder) {
                Ok(mut mcu) => {
                    let _ = mcu.send_message(&RpiToMcu::SyncState).await;
                    break mcu;
                }
                Err(e) => {
                    eprintln!(
                        "MCU: Failed to open serial port on {MCU_PORT}: {e}, retrying in 5 seconds..."
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        };

        loop {
            tokio::select! {
                message = mcu.receive_message() => match message {
                    Ok(message) => {
                        match &message {
                            McuToRpi::BatteryVoltage(v) => update_state(&robot_state, &gcs_tx, |s| s.mcu.battery_voltage = *v),
                            McuToRpi::BatteryLevel(v) => update_state(&robot_state, &gcs_tx, |s| s.mcu.battery_level = *v),
                            McuToRpi::ArmState(v) => update_state(&robot_state, &gcs_tx, |s| s.mcu.arm = *v),
                            McuToRpi::McuState(v) => update_state(&robot_state, &gcs_tx, |s| s.mcu = *v),
                        }

                        // If the message can be converted to RpiToGcs, forward it
                        if let Ok(message) = RpiToGcs::try_from(message) {
                            let _ = gcs_tx.send(message);
                        }
                    },
                    Err(e @ drivers::mcu::McuError::InvalidDataReceived) => {
                        eprintln!("MCU RX: {e}");
                    }
                    Err(e) => {
                        eprintln!("MCU RX: Serial connection lost ({e}), reconnecting in 5 seconds...");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        break;
                    }
                },
                message = sending_queue.recv() => match message {
                    Ok(message) => {
                        if let Err(e) = mcu.send_message(&message).await {
                            eprintln!("MCU TX: Failed to send message to MCU: {e:?}");
                        }
                    },
                    Err(RecvError::Lagged(skipped)) => {
                        eprintln!("MCU TX: sending lagged, skipped {skipped} messages");
                    }
                    Err(RecvError::Closed) => {
                        return;
                    }
                }
            }
        }
    }
}

fn update_state(
    robot_state: &RwLock<RobotState>,
    gcs_tx: &broadcast::Sender<RpiToGcs>,
    mutate: impl FnOnce(&mut RobotState),
) {
    let mut state = robot_state.write().unwrap();
    mutate(&mut state);
    let _ = gcs_tx.send(RpiToGcs::RobotState(*state));
}
