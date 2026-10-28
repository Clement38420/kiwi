use crate::drivers;

const MCU_PORT: &str = "/dev/serial0";

pub(crate) async fn mcu_task(gcs_tx: tokio::sync::broadcast::Sender<shared::messages::RpiToGcs>) {
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

    loop {
        match mcu.receive_message().await {
            Ok(message) => match message {
                shared::messages::McuToRpi::BatteryVoltage(voltage) => {
                    let _ = gcs_tx.send(shared::messages::RpiToGcs::BatteryVoltage(voltage));
                }
            },
            Err(e) => {
                eprintln!("MCU RX: Failed to receive message from MCU: {e:?}");
            }
        }
    }
}
