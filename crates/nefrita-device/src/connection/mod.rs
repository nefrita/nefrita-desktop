#[cfg(test)]
pub(crate) mod scripted;
mod serial;
mod tcp;
mod usb_discovery;

use std::io::{self, Read, Write};
use std::time::Duration;

pub use serial::SerialConnection;
pub use usb_discovery::{DiyBoardPort, UsbSerialChip, find_diy_board_ports};

pub trait DeviceConnection: Read + Write {
    fn reply_timeout(&self) -> io::Result<Option<Duration>>;

    fn set_reply_timeout(&mut self, reply_timeout: Option<Duration>) -> io::Result<()>;
}
