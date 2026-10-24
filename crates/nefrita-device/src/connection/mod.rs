#[cfg(test)]
pub(crate) mod scripted;
mod tcp;

use std::io::{self, Read, Write};
use std::time::Duration;

pub trait DeviceConnection: Read + Write {
    fn reply_timeout(&self) -> io::Result<Option<Duration>>;

    fn set_reply_timeout(&mut self, reply_timeout: Option<Duration>) -> io::Result<()>;
}
