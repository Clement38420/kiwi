use crate::board::{UART1_RX_BUFFER_SIZE, UART1_TX_BUFFER_SIZE};
use crate::tasks::communication::MESSAGE_CHANNEL;
use defmt::error;
use embassy_futures::join::join;
use embassy_stm32::usart;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel;
use embedded_io_async::{Read, Write};
use shared::messages;
use shared::messages::{McuToRpi, RpiToMcu};

#[embassy_executor::task]
pub async fn uart_task(uart: usart::BufferedUart<'static>) {
    let (tx, rx) = uart.split();
    join(uart_tx_task(tx), uart_rx_task(rx)).await;
}

async fn uart_rx_task(mut uart: usart::BufferedUartRx<'static>) {
    let mut rx_buffer = [0u8; UART1_RX_BUFFER_SIZE];

    let mut i = 0;
    loop {
        let mut byte = [0u8; 1];
        if let Err(e) = uart.read_exact(&mut byte).await {
            error!(
                "UART RX: Failed to read from UART: {:?}",
                defmt::Debug2Format(&e)
            );
        }
        let b = byte[0];

        if b == 0x00 {
            match messages::decode(&mut rx_buffer[..i]) {
                Ok(message) => {
                    if let Err(e) = MESSAGE_CHANNEL.try_send(message) {
                        error!(
                            "UART RX: Failed to send message to channel: {:?}",
                            defmt::Debug2Format(&e)
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "UART RX: {:?}: Failed to decode message : {=[u8]:x}",
                        e,
                        &rx_buffer[..i]
                    )
                }
            }

            i = 0;
        } else {
            rx_buffer[i] = b;
            i = (i + 1) % UART1_RX_BUFFER_SIZE;
        }
    }
}

pub static TX_CHANNEL: channel::Channel<ThreadModeRawMutex, McuToRpi, 4> = channel::Channel::new();

async fn uart_tx_task(mut uart: usart::BufferedUartTx<'static>) {
    loop {
        let message = TX_CHANNEL.receive().await;
        let mut computation_buffer = [0u8; UART1_TX_BUFFER_SIZE];
        let mut tx_buffer = [0u8; UART1_TX_BUFFER_SIZE];

        match messages::encode(&message, &mut tx_buffer, &mut computation_buffer) {
            Ok(encoded_len) => {
                if let Err(e) = uart.write_all(&tx_buffer[..encoded_len]).await {
                    error!(
                        "UART TX: Failed to write message to UART: {:?}",
                        defmt::Debug2Format(&e)
                    );
                }
            }
            Err(e) => {
                error!(
                    "UART TX: Failed to encode message: {:?}",
                    defmt::Debug2Format(&e)
                );
            }
        }
    }
}
