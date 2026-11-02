use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_serial::{ClearBuffer, SerialPort, SerialPortBuilderExt};

const STOP_SCAN_CMD: [u8; 2] = [0xA5, 0x65];
const START_SCAN_CMD: [u8; 2] = [0xA5, 0x60];
const SCAN_PACKET_FLAG: [u8; 2] = [0xAA, 0x55];
const SCAN_START_PACKET_TYPE: u8 = 0x01;

#[derive(PartialEq, Eq)]
enum ScanPacketType {
    Start,
    Data,
}

impl From<u8> for ScanPacketType {
    fn from(value: u8) -> Self {
        if value & 0x01 == SCAN_START_PACKET_TYPE {
            ScanPacketType::Start
        } else {
            ScanPacketType::Data
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    pub angle: f32,
    pub distance: f32,
}

struct ScanPacket {
    package_type: ScanPacketType,
    sample_qty: u8,
    start_angle: f32,
    end_angle: f32,
    check_code: u16,
    sample_data: Vec<DataPoint>,
}

#[derive(Error, Debug)]
pub enum LidarError {
    #[error("Corrupt packet received")]
    CorruptPacket,
    #[error("IO error: {0}")]
    IO(std::io::Error),
    #[error("Serial port error: {0}")]
    Serial(tokio_serial::Error),
}

impl From<std::io::Error> for LidarError {
    fn from(err: std::io::Error) -> Self {
        LidarError::IO(err)
    }
}

impl From<tokio_serial::Error> for LidarError {
    fn from(err: tokio_serial::Error) -> Self {
        LidarError::Serial(err)
    }
}

pub type LidarResult<T> = Result<T, LidarError>;

pub struct Lidar {
    port: BufReader<tokio_serial::SerialStream>,
}

impl Lidar {
    /// Creates a new instance of the Lidar driver.
    ///
    /// # Errors
    /// Returns `LidarError::Serial` if the serial port could not be opened
    pub fn new(port_builder: tokio_serial::SerialPortBuilder) -> LidarResult<Self> {
        let port = BufReader::new(port_builder.open_native_async()?);
        Ok(Self { port })
    }

    /// Starts the LIDAR scan.
    ///
    /// # Errors
    /// * Returns `LidarError::IO` if there was an error writing to the serial port
    /// * Returns `LidarError::Serial` if the start scan command could not be sent
    pub async fn start_scan(&mut self) -> LidarResult<()> {
        self.port.write_all(&START_SCAN_CMD).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await; // Wait for the LIDAR motor to stabilize
        self.port.get_mut().clear(ClearBuffer::Input)?;

        loop {
            match self.read_scan_packet().await {
                Ok(packet) if packet.package_type == ScanPacketType::Start => break,
                Ok(_) => {}
                Err(e @ LidarError::CorruptPacket) => {
                    eprintln!("{e}");
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Stops the LIDAR scan.
    ///
    /// # Errors
    /// Returns `LidarError::IO` if there was an error writing to the serial port
    pub async fn stop_scan(&mut self) -> LidarResult<()> {
        self.port.write_all(&STOP_SCAN_CMD).await?;
        Ok(())
    }

    fn compute_data_point(
        start_angle: f32,
        end_angle: f32,
        n_samples: u8,
        index: u8,
        sample: u16,
    ) -> DataPoint {
        let distance = f32::from(sample) / 2.0;
        let mut angle = if n_samples > 1 {
            (end_angle - start_angle).rem_euclid(360.0) / f32::from(n_samples - 1)
                * f32::from(index)
                + start_angle
        } else {
            start_angle
        };

        let angle_correct = if distance == 0.0 {
            0.0
        } else {
            f32::atan(21.8 * (155.3 - distance) / (155.3 * distance)).to_degrees()
        };

        angle += angle_correct;

        angle = angle.rem_euclid(360.0);

        DataPoint { angle, distance }
    }

    async fn read_scan_packet(&mut self) -> LidarResult<ScanPacket> {
        fn bytes_to_angle(bytes: [u8; 2]) -> f32 {
            let raw_angle = u16::from_le_bytes(bytes) >> 1;
            f32::from(raw_angle) / 64.0
        }

        // Detect the start of a scan packet by looking for the SCAN_PACKET_FLAG with a sliding window of 2 bytes to avoid missing the flag if it spans across two reads.
        let mut window = [0u8; 2];
        loop {
            let mut b = [0u8; 1];
            self.port.read_exact(&mut b).await?;
            window = [window[1], b[0]];

            if window == SCAN_PACKET_FLAG {
                break;
            }
        }

        let mut header = [0u8; 8];
        self.port.read_exact(&mut header).await?;

        let mut packet = ScanPacket {
            package_type: header[0].into(),
            sample_qty: header[1],
            start_angle: bytes_to_angle([header[2], header[3]]),
            end_angle: bytes_to_angle([header[4], header[5]]),
            check_code: u16::from_le_bytes([header[6], header[7]]),
            sample_data: Vec::new(),
        };

        let n_samples_bytes = packet.sample_qty as usize * 2;
        let mut sample_data_raw_chunks = [0u8; 510]; // 255 samples max * 2 bytes each
        self.port
            .read_exact(&mut sample_data_raw_chunks[..n_samples_bytes])
            .await?;
        let sample_data_raw_chunks = sample_data_raw_chunks[..n_samples_bytes].as_chunks::<2>().0;

        let mut check_code = header
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 3) // Skip the check code bytes
            .fold(u16::from_le_bytes(SCAN_PACKET_FLAG), |acc, (_, &chunk)| {
                acc ^ u16::from_le_bytes(chunk)
            });

        check_code = sample_data_raw_chunks
            .iter()
            .fold(check_code, |acc, &c| acc ^ u16::from_le_bytes(c));

        if check_code != packet.check_code {
            return Err(LidarError::CorruptPacket);
        }

        packet.sample_data = sample_data_raw_chunks
            .iter()
            .enumerate()
            .map(|(index, &sample)| {
                Self::compute_data_point(
                    packet.start_angle,
                    packet.end_angle,
                    packet.sample_qty,
                    u8::try_from(index).unwrap_or(0),
                    u16::from_le_bytes(sample),
                )
            })
            .collect();

        Ok(packet)
    }

    /// Reads a full revolution of scan data from the LIDAR.
    ///
    /// # Errors
    /// * Returns `LidarError::IO` if there was an error reading from the serial
    /// * Returns `LidarError::Serial` if there was an error with the serial port
    /// * Returns `LidarError::CorruptPacket` if a corrupt packet is received
    pub async fn read_scan_full_revolution(&mut self) -> LidarResult<Vec<DataPoint>> {
        let mut points: Vec<DataPoint> = Vec::new();
        loop {
            let packet = self.read_scan_packet().await?;
            points.extend(packet.sample_data);
            if packet.package_type == ScanPacketType::Start {
                break;
            }
        }

        Ok(points)
    }
}

impl Drop for Lidar {
    fn drop(&mut self) {
        let _ = std::io::Write::write_all(&mut self.port.get_mut(), &STOP_SCAN_CMD);
        let _ = std::io::Write::flush(&mut self.port.get_mut());
    }
}
