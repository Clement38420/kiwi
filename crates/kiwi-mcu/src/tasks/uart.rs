use crate::board::{UART1_RX_BUFFER_SIZE, UART1_TX_BUFFER_SIZE};
use crate::channels::{UART_RX, UART_TX};
use defmt::error;
use embassy_futures::join::join;
use embassy_stm32::usart;
use embedded_io_async::{Read, Write};
use shared::codec;

#[embassy_executor::task]
pub async fn uart_task(uart: usart::BufferedUart<'static>) {
    let (tx, rx) = uart.split();
    join(uart_tx_task(tx), uart_rx_task(rx)).await;
}

async fn uart_rx_task(mut uart: usart::BufferedUartRx<'static>) {
    let mut rx_buffer = [0u8; UART1_RX_BUFFER_SIZE];

    let mut i = 0;
    let mut overflow = false;
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
            if overflow {
                error!("UART RX: Frame exceeded buffer size, discarding");
                overflow = false;
            } else {
                match codec::decode(&mut rx_buffer[..i]) {
                    Ok(message) => {
                        if let Err(e) = UART_RX.try_send(message) {
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
                        );
                    }
                }
            }

            i = 0;
        } else if i < UART1_RX_BUFFER_SIZE {
            rx_buffer[i] = b;
            i += 1;
        } else {
            overflow = true;
        }
    }
}

async fn uart_tx_task(mut uart: usart::BufferedUartTx<'static>) {
    loop {
        let message = UART_TX.receive().await;
        let mut computation_buffer = [0u8; UART1_TX_BUFFER_SIZE];
        let mut tx_buffer = [0u8; UART1_TX_BUFFER_SIZE];

        match codec::encode(&message, &mut tx_buffer, &mut computation_buffer) {
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
