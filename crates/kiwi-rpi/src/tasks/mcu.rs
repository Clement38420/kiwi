use crate::drivers;
use shared::messages::{McuToRpi, RpiToGcs};
use tokio::sync::broadcast::error::RecvError;

const MCU_PORT: &str = "/dev/serial0";

pub(crate) async fn mcu_task(
    sending_queue: tokio::sync::broadcast::Sender<shared::messages::RpiToMcu>,
    gcs_tx: tokio::sync::broadcast::Sender<shared::messages::RpiToGcs>,
) {
    let mut mcu = loop {
        let mcu_port_builder = tokio_serial::new(MCU_PORT, 115_200)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .data_bits(tokio_serial::DataBits::Eight)
            .flow_control(tokio_serial::FlowControl::None);

        match drivers::mcu::Mcu::new(mcu_port_builder) {
            Ok(mcu) => break mcu,
            Err(e) => {
                eprintln!(
                    "MCU: Failed to open serial port on {MCU_PORT}: {e}, retrying in 5 second..."
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    };

    let mut sending_queue = sending_queue.subscribe();

    loop {
        tokio::select! {
            message = mcu.receive_message() => match message {
                Ok(message) => {
                    match &message {
                        McuToRpi::BatteryVoltage(_)
                        | McuToRpi::BatteryLevel(_)
                        | McuToRpi::ArmState(_)
                        | McuToRpi::GetStateRes(_) => {}
                    }

                    // If the message can be converted to RpiToGcs, forward it
                    if let Ok(message) = RpiToGcs::try_from(message) {
                        let _ = gcs_tx.send(message);
                    }
                },
                Err(e) => {
                    eprintln!("MCU RX: Failed to receive message from MCU: {e:?}");
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
                    break;
                }
            }
        }
    }
}
