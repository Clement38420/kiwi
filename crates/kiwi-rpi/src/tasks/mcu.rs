use crate::channels::{Base, Communication};
use crate::drivers;
use messages::RpiToMcu;
use shared::geometry::Pose2D;
use shared::messages::{self, McuToRpi, RpiToGcs};
use tokio::sync::broadcast::error::RecvError;

const MCU_PORT: &str = "/dev/serial0";

pub async fn mcu_task(comm: Communication, base: Base) {
    let mut mcu_rx = comm.mcu.subscribe();

    loop {
        let mut mcu = loop {
            let mcu_port_builder = tokio_serial::new(MCU_PORT, 115_200)
                .parity(tokio_serial::Parity::None)
                .stop_bits(tokio_serial::StopBits::One)
                .data_bits(tokio_serial::DataBits::Eight)
                .flow_control(tokio_serial::FlowControl::None);

            match drivers::mcu::Mcu::new(mcu_port_builder) {
                Ok(mut mcu) => {
                    let _ = mcu.send_message(&RpiToMcu::Sync).await;
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
                            McuToRpi::BatteryVoltage(v) => base.state.send_modify(|s| s.mcu.battery_voltage = *v),
                            McuToRpi::BatteryLevel(v) => base.state.send_modify(|s| s.mcu.battery_level = *v),
                            McuToRpi::ArmState(v) => base.state.send_modify(|s| s.mcu.arm = *v),
                            McuToRpi::McuState(v) => base.state.send_modify(|s| s.mcu = *v),
                            McuToRpi::Odom(v) => { base.odom.send_replace(*v); },
                            McuToRpi::ResetAck => {
                                base.odom.send_replace(Pose2D::default());
                                let _ = base.reset.send(());
                            },
                        }

                        // If the message can be converted to RpiToGcs, forward it
                        if let Ok(message) = RpiToGcs::try_from(message) {
                            let _ = comm.gcs.send(message);
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
                message = mcu_rx.recv() => match message {
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
