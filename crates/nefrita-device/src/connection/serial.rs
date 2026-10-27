use std::io::{self, Read, Write};
use std::time::Duration;

use serialport::{ClearBuffer, SerialPort};

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;

const DEVICE_BAUD_RATE: u32 = 115_200;
const PORT_TIMEOUT_WITHOUT_REPLY_TIMEOUT: Duration = Duration::from_hours(24);

pub struct SerialConnection {
    serial_port: Box<dyn SerialPort>,
    reply_timeout: Option<Duration>,
}

impl SerialConnection {
    fn prepare_open_port(
        mut serial_port: Box<dyn SerialPort>,
        reply_timeout: Duration,
    ) -> serialport::Result<Self> {
        lower_auto_reset_lines(serial_port.as_mut())?;
        serial_port.clear(ClearBuffer::Input)?;
        serial_port.set_timeout(reply_timeout)?;
        Ok(Self {
            serial_port,
            reply_timeout: Some(reply_timeout),
        })
    }
}

impl Read for SerialConnection {
    fn read(&mut self, read_buffer: &mut [u8]) -> io::Result<usize> {
        self.serial_port.read(read_buffer)
    }
}

impl Write for SerialConnection {
    fn write(&mut self, outgoing_bytes: &[u8]) -> io::Result<usize> {
        self.serial_port.write(outgoing_bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl DeviceConnection for SerialConnection {
    fn reply_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(self.reply_timeout)
    }

    fn set_reply_timeout(&mut self, reply_timeout: Option<Duration>) -> io::Result<()> {
        self.serial_port
            .set_timeout(reply_timeout.unwrap_or(PORT_TIMEOUT_WITHOUT_REPLY_TIMEOUT))?;
        self.reply_timeout = reply_timeout;
        Ok(())
    }
}

impl Device<SerialConnection> {
    pub fn connect_serial(port_name: &str, reply_timeout: Duration) -> Result<Self, DeviceError> {
        let connection = serialport::new(port_name, DEVICE_BAUD_RATE)
            .open()
            .and_then(|serial_port| SerialConnection::prepare_open_port(serial_port, reply_timeout))
            .map_err(|serial_port_error| DeviceError::ConnectionFailed(serial_port_error.into()))?;
        Ok(Self::new(connection))
    }
}

fn lower_auto_reset_lines(serial_port: &mut dyn SerialPort) -> serialport::Result<()> {
    if serial_port.write_request_to_send(false).is_ok() {
        serial_port.write_data_terminal_ready(false)?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
