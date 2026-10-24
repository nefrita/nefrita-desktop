#[cfg(test)]
mod scripted;
mod version_information;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::connection::DeviceConnection;
use crate::error::DeviceError;
use crate::protocol::{self, MessageReader};

pub use version_information::{DeviceState, VersionInformation};

pub struct Device<Connection> {
    connection: Connection,
    message_reader: MessageReader,
    last_request_number: u32,
}

impl<Connection: DeviceConnection> Device<Connection> {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            message_reader: MessageReader::default(),
            last_request_number: 0,
        }
    }

    fn send_request<Parameters: Serialize, MethodResult: DeserializeOwned>(
        &mut self,
        method_name: &str,
        parameters: Option<&Parameters>,
    ) -> Result<MethodResult, DeviceError> {
        let request_identifier = self.next_request_identifier();
        let encoded_request =
            protocol::encode_request(&request_identifier, method_name, parameters)?;
        self.connection
            .write_all(&encoded_request)
            .and_then(|()| self.connection.flush())
            .map_err(DeviceError::ConnectionFailed)?;
        self.message_reader
            .read_reply(&mut self.connection, &request_identifier)
    }

    fn next_request_identifier(&mut self) -> String {
        self.last_request_number = self.last_request_number.wrapping_add(1);
        self.last_request_number.to_string()
    }
}

#[cfg(test)]
impl<Connection> Device<Connection> {
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }
}

#[cfg(test)]
mod tests;
