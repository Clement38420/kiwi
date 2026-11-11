use cobs::max_encoding_length;
use crc::Crc;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct LidarPoint {
    pub angle: f32,
    pub distance: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum BatteryLevel {
    High,
    Medium,
    Low,
    Critical,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum ArmState {
    Armed,
    Disarmed,
    CannotArm,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(TS))]
pub struct McuState {
    pub battery_voltage: f32,
    pub battery_level: BatteryLevel,
    pub arm: ArmState,
}

impl Default for McuState {
    fn default() -> Self {
        McuState {
            battery_voltage: 0.0,
            battery_level: BatteryLevel::Critical,
            arm: ArmState::CannotArm,
        }
    }
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RobotState {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub mcu: McuState,
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RobotPose {
    pub x: f32,
    pub y: f32,
    pub theta: f32,
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Twist {
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
}

#[cfg(feature = "std")]
impl Twist {
    pub fn to_wheel_speeds(&self) -> [f32; 3] {
        let radius_inv = 1.0_f32 / crate::consts::WHEEL_RADIUS;

        let rot = crate::consts::WHEEL_BASE * self.w;
        let vx_comp = 3.0_f32.sqrt() / 2.0 * self.vx;
        let vy_half = 0.5 * self.vy;

        let mut speeds = [
            radius_inv * (-self.vy + rot),
            radius_inv * (vx_comp + vy_half + rot),
            radius_inv * (-vx_comp + vy_half + rot),
        ];

        let max_speed = speeds
            .iter()
            .map(|s| s.abs())
            .reduce(f32::max)
            .unwrap_or(0.0);

        if max_speed > crate::consts::MAX_WHEEL_SPEED {
            let scale = crate::consts::MAX_WHEEL_SPEED / max_speed;
            for s in &mut speeds {
                *s *= scale;
            }
        }

        speeds
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum McuToRpi {
    BatteryVoltage(f32),
    BatteryLevel(BatteryLevel),
    ArmState(ArmState),
    McuState(McuState),
    Theta(f32),
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum RpiToGcs {
    LidarScan(std::sync::Arc<Vec<LidarPoint>>),
    RobotState(RobotState),
    GetStateRes(RpcResponse<RobotState>),
    RobotPose(RobotPose),
}

#[cfg(feature = "std")]
impl TryFrom<McuToRpi> for RpiToGcs {
    type Error = ();

    fn try_from(value: McuToRpi) -> Result<Self, Self::Error> {
        match value {
            McuToRpi::BatteryVoltage(_)
            | McuToRpi::BatteryLevel(_)
            | McuToRpi::ArmState(_)
            | McuToRpi::McuState(_)
            | McuToRpi::Theta(_) => Err(()),
        }
    }
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum GcsToRpi {
    Arm(bool),
    GetStateReq(RpcRequest<()>),
    Reset,
    Twist(Twist),
}

#[cfg(feature = "std")]
impl TryFrom<GcsToRpi> for RpiToMcu {
    type Error = ();

    fn try_from(value: GcsToRpi) -> Result<Self, Self::Error> {
        match value {
            GcsToRpi::Arm(v) => Ok(RpiToMcu::Arm(v)),
            GcsToRpi::GetStateReq(_) => Err(()),
            GcsToRpi::Reset => Ok(RpiToMcu::Reset),
            GcsToRpi::Twist(_) => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RpiToMcu {
    WheelSpeedSetpoints([f32; 3]),
    Arm(bool),
    Sync,
    Reset,
}

const CRC: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_MODBUS);

#[derive(defmt::Format, Debug)]
pub enum DecodeError {
    CobsDecodeError,
    MessageTooShort,
    InvalidCrc,
    DeserializeError,
}

/// Decodes a message from a buffer which contains exactly de packet without the trailing 0x00, returning the decoded message.
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

#[cfg(feature = "std")]
use crate::rpc::{RpcRequest, RpcResponse};
#[cfg(not(feature = "std"))]
use core::cmp::min;

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
/// Encodes a message into a Vec<u8>, returning the encoded message.
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
