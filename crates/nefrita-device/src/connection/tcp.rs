use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;

impl DeviceConnection for TcpStream {
    fn reply_timeout(&self) -> io::Result<Option<Duration>> {
        self.read_timeout()
    }

    fn set_reply_timeout(&mut self, reply_timeout: Option<Duration>) -> io::Result<()> {
        self.set_read_timeout(reply_timeout)
    }
}

impl Device<TcpStream> {
    pub fn connect_tcp(
        address: impl ToSocketAddrs,
        reply_timeout: Duration,
    ) -> Result<Self, DeviceError> {
        let connection = TcpStream::connect(address)
            .and_then(|tcp_stream| {
                tcp_stream.set_read_timeout(Some(reply_timeout))?;
                tcp_stream.set_nodelay(true)?;
                Ok(tcp_stream)
            })
            .map_err(DeviceError::ConnectionFailed)?;
        Ok(Self::new(connection))
    }
}
