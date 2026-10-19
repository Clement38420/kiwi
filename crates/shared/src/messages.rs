use cobs::max_encoding_length;
use core::cmp::min;
use crc::Crc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Copy, Clone, Debug)]
pub struct WheelSpeedSetpoint(pub f32);

impl WheelSpeedSetpoint {
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl From<f32> for WheelSpeedSetpoint {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, Default, Copy, Clone, Debug)]
pub struct WheelSpeedSetpoints(pub [WheelSpeedSetpoint; 3]);

impl WheelSpeedSetpoints {
    pub fn values(&self) -> [f32; 3] {
        self.0.map(|setpoint| setpoint.value())
    }
}

impl From<[f32; 3]> for WheelSpeedSetpoints {
    fn from(values: [f32; 3]) -> Self {
        Self(values.map(WheelSpeedSetpoint::from))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    WheelSpeedSetpoints(WheelSpeedSetpoints),
}

const CRC: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_MODBUS);

#[derive(defmt::Format, Debug)]
pub enum DecodeError {
    CobsDecodeError,
    MessageTooShort,
    InvalidCrc,
    DeserializeError,
}

pub fn decode(buffer: &mut [u8]) -> Result<Message, DecodeError> {
    let decoded_len =
        cobs::decode_in_place(&mut buffer[..]).map_err(|_| DecodeError::CobsDecodeError)?;
    let message = &buffer[..decoded_len];

    if message.len() < 2 {
        return Err(DecodeError::MessageTooShort);
    }

    let (payload, crc) = message.split_at(message.len() - 2);

    // This unwrap is safe because we already checked that message.len() >= 2
    if CRC.checksum(payload) != u16::from_le_bytes(crc.try_into().unwrap()) {
        return Err(DecodeError::InvalidCrc);
    }

    let message = postcard::from_bytes(&message[..message.len() - 2])
        .map_err(|_| DecodeError::DeserializeError)?;

    Ok(message)
}

#[derive(defmt::Format, Debug)]
pub enum EncodeError {
    SerializeError,
    MessageTooLarge,
}

pub fn encode(
    message: &Message,
    buffer: &mut [u8],
    computation_buffer: &mut [u8],
) -> Result<usize, EncodeError> {
    let message_len = postcard::to_slice(message, computation_buffer)
        .map_err(|_| EncodeError::SerializeError)?
        .len();

    if max_encoding_length(message_len + 2) + 1 > min(computation_buffer.len(), buffer.len()) {
        return Err(EncodeError::MessageTooLarge);
    }

    let crc = CRC.checksum(&computation_buffer[..message_len]);

    computation_buffer[message_len..message_len + 2].copy_from_slice(&crc.to_le_bytes());

    let encoded_len = cobs::encode(&computation_buffer[..message_len + 2], buffer);

    buffer[encoded_len] = 0x00;

    Ok(encoded_len + 1)
}
