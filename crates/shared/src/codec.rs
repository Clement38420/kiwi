use cobs::max_encoding_length;
#[cfg(not(feature = "std"))]
use core::cmp::min;
use crc::Crc;
use serde::{Deserialize, Serialize};

const CRC: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_MODBUS);

#[derive(defmt::Format, Debug)]
pub enum DecodeError {
    CobsDecodeError,
    MessageTooShort,
    InvalidCrc,
    DeserializeError,
}

/// Decodes a message from a buffer which contains exactly the packet without the trailing 0x00, returning the decoded message.
///
/// # Errors
/// Returns a `DecodeError` if the message is invalid or cannot be decoded.
pub fn decode<'a, T: Deserialize<'a>>(buffer: &'a mut [u8]) -> Result<T, DecodeError> {
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
pub enum EncodeError<T> {
    SerializeError(T),
    MessageTooLarge,
}

#[cfg(not(feature = "std"))]
/// Encodes a message into a buffer, returning the number of bytes written to the buffer.
///
/// # Errors
/// Returns an `EncodeError` if the message is too large to fit in the buffer or if serialization fails.
pub fn encode<T: Serialize>(
    message: &T,
    buffer: &mut [u8],
    computation_buffer: &mut [u8],
) -> Result<usize, EncodeError<postcard::Error>> {
    let message_len = postcard::to_slice(message, computation_buffer)
        .map_err(EncodeError::SerializeError)?
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

#[cfg(feature = "std")]
/// Encodes a message into a `Vec<u8>`, returning the encoded message.
///
/// # Errors
/// Returns an `EncodeError` if serialization fails.
pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, EncodeError<postcard::Error>> {
    let mut payload = postcard::to_stdvec(message).map_err(EncodeError::SerializeError)?;
    let crc = CRC.checksum(&payload);
    payload.extend_from_slice(&crc.to_le_bytes());

    let mut buffer = vec![0u8; max_encoding_length(payload.len()) + 1];
    let encoded_len = cobs::encode(&payload, &mut buffer);
    buffer[encoded_len] = 0x00;
    buffer.truncate(encoded_len + 1);

    Ok(buffer)
}
