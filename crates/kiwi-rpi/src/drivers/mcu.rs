use shared::messages::{McuToRpi, RpiToMcu};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_serial::SerialPortBuilderExt;

#[derive(Error, Debug)]
pub enum McuError {
    #[error("IO error: {0}")]
    IO(std::io::Error),
    #[error("Serial port error: {0}")]
    Serial(tokio_serial::Error),
    #[error("Invalid data received")]
    InvalidDataReceived,
    #[error("Failed to encode message")]
    EncodingError,
}

impl From<std::io::Error> for McuError {
    fn from(err: std::io::Error) -> Self {
        McuError::IO(err)
    }
}

impl From<tokio_serial::Error> for McuError {
    fn from(err: tokio_serial::Error) -> Self {
        McuError::Serial(err)
    }
}

type McuResult<T> = Result<T, McuError>;

pub(crate) struct Mcu {
    port: BufReader<tokio_serial::SerialStream>,
}

impl Mcu {
    /// Creates a new instance of the Mcu driver.
    ///
    /// # Errors
    /// Returns `McuError::Serial` if the serial port could not be opened
    pub fn new(port_builder: tokio_serial::SerialPortBuilder) -> McuResult<Self> {
        let port = BufReader::new(port_builder.open_native_async()?);
        Ok(Self { port })
    }

    /// Receives a message from the MCU over the serial port.
    ///
    /// # Errors
    /// Returns `McuError::IO` if there is an I/O error while reading from the serial port.
    /// Returns `McuError::InvalidDataReceived` if the received data cannot be decoded into a valid `McuToRpi` message.
    pub async fn receive_message(&mut self) -> McuResult<McuToRpi> {
        let mut buffer: Vec<u8> = Vec::new();

        let bytes_read = self.port.read_until(0x00, &mut buffer).await?;

        if bytes_read == 0 {
            return Err(McuError::IO(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Serial port closed",
            )));
        }

        // Remove the trailing null byte (0x00) from the buffer
        buffer.pop();

        shared::messages::decode(&mut buffer).map_err(|_| McuError::InvalidDataReceived)
    }

    /// Sends a message to the MCU over the serial port.
    ///
    /// # Errors
    /// Returns `McuError::IO` if there is an I/O error while writing to the serial port.
    /// Returns `McuError::EncodingError` if the message cannot be encoded into bytes.
    /// Returns `McuError::Serial` if there is an error with the serial port.
    pub async fn send_message(&mut self, message: &RpiToMcu) -> McuResult<()> {
        match shared::messages::encode(message) {
            Ok(bytes) => {
                self.port.write_all(&bytes).await?;
                Ok(())
            }
            Err(_) => Err(McuError::EncodingError),
        }
    }
}
