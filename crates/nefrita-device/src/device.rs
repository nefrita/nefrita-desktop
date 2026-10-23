use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::DeviceError;
use crate::message::{self, MessageReader, WITHOUT_PARAMETERS};

const XBT_CHAIN: &str = "XBT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DeviceState {
    #[serde(rename = "UNINIT")]
    Uninitialized,
    #[serde(rename = "UNSAVED")]
    Unsaved,
    #[serde(rename = "LOCKED")]
    Locked,
    #[serde(rename = "READY")]
    Ready,
    #[serde(rename = "TEMP")]
    Temporary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionInformation {
    #[serde(rename = "JADE_VERSION")]
    pub firmware_version: String,
    #[serde(rename = "BOARD_TYPE")]
    pub board_type: String,
    #[serde(rename = "JADE_STATE")]
    pub device_state: DeviceState,
    #[serde(rename = "JADE_CHAIN")]
    pub chain: Option<String>,
}

pub struct Device<Connection> {
    connection: Connection,
    message_reader: MessageReader,
    last_request_number: u32,
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

impl<Connection: Read + Write> Device<Connection> {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            message_reader: MessageReader::default(),
            last_request_number: 0,
        }
    }

    pub fn request_version_information(&mut self) -> Result<VersionInformation, DeviceError> {
        self.send_request("get_version_info", WITHOUT_PARAMETERS)
    }

    pub fn require_xbt_firmware(&mut self) -> Result<VersionInformation, DeviceError> {
        let version_information = self.request_version_information()?;
        if version_information.chain.as_deref() == Some(XBT_CHAIN) {
            Ok(version_information)
        } else {
            Err(DeviceError::NotXbtFirmware {
                chain: version_information.chain,
            })
        }
    }

    fn send_request<Parameters: Serialize, MethodResult: DeserializeOwned>(
        &mut self,
        method_name: &str,
        parameters: Option<&Parameters>,
    ) -> Result<MethodResult, DeviceError> {
        let request_identifier = self.next_request_identifier();
        let encoded_request =
            message::encode_request(&request_identifier, method_name, parameters)?;
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
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::scripted_connection::{ScriptedConnection, ScriptedRead, encode_cbor_message};

    fn version_information_reply(request_identifier: &str, chain: Option<&str>) -> Vec<u8> {
        let mut method_result = json!({
            "JADE_VERSION": "1.0.41",
            "BOARD_TYPE": "M5_STICKC_PLUS_2",
            "JADE_STATE": "LOCKED",
            "JADE_NETWORKS": "ALL",
            "JADE_HAS_PIN": true,
        });
        if let Some(chain) = chain {
            method_result["JADE_CHAIN"] = json!(chain);
        }
        encode_cbor_message(&json!({"id": request_identifier, "result": method_result}))
    }

    fn device_replying_with(encoded_replies: Vec<Vec<u8>>) -> Device<ScriptedConnection> {
        let scripted_reads = encoded_replies
            .into_iter()
            .map(ScriptedRead::Bytes)
            .collect();
        Device::new(ScriptedConnection::new(scripted_reads))
    }

    #[test]
    fn version_information_is_requested_and_decoded() {
        let mut device = device_replying_with(vec![version_information_reply("1", Some("XBT"))]);
        let version_information = device
            .request_version_information()
            .expect("the version information should be decoded");
        assert_eq!(
            version_information,
            VersionInformation {
                firmware_version: "1.0.41".to_owned(),
                board_type: "M5_STICKC_PLUS_2".to_owned(),
                device_state: DeviceState::Locked,
                chain: Some("XBT".to_owned()),
            }
        );
        assert_eq!(
            device.connection().written_messages(),
            vec![json!({"id": "1", "method": "get_version_info"})]
        );
    }

    #[test]
    fn each_request_uses_the_next_identifier() {
        let mut device = device_replying_with(vec![
            version_information_reply("1", Some("XBT")),
            version_information_reply("2", Some("XBT")),
        ]);
        device
            .request_version_information()
            .expect("the first request should be answered");
        device
            .request_version_information()
            .expect("the second request should be answered");
        let request_identifiers: Vec<Value> = device
            .connection()
            .written_messages()
            .into_iter()
            .map(|written_message| written_message["id"].clone())
            .collect();
        assert_eq!(request_identifiers, vec![json!("1"), json!("2")]);
    }

    #[test]
    fn device_states_decode_from_their_protocol_names() {
        let protocol_names_and_states = [
            ("UNINIT", DeviceState::Uninitialized),
            ("UNSAVED", DeviceState::Unsaved),
            ("LOCKED", DeviceState::Locked),
            ("READY", DeviceState::Ready),
            ("TEMP", DeviceState::Temporary),
        ];
        for (protocol_name, device_state) in protocol_names_and_states {
            let encoded_state = encode_cbor_message(&json!(protocol_name));
            let decoded_state: DeviceState =
                minicbor_serde::from_slice(&encoded_state).expect("the state should decode");
            assert_eq!(decoded_state, device_state, "state {protocol_name}");
        }
    }

    #[test]
    fn xbt_firmware_is_accepted() {
        let mut device = device_replying_with(vec![version_information_reply("1", Some("XBT"))]);
        let version_information = device
            .require_xbt_firmware()
            .expect("an XBT firmware should be accepted");
        assert_eq!(version_information.chain.as_deref(), Some("XBT"));
    }

    #[test]
    fn firmware_without_a_chain_is_rejected() {
        let mut device = device_replying_with(vec![version_information_reply("1", None)]);
        let firmware_check = device.require_xbt_firmware();
        assert!(matches!(
            firmware_check,
            Err(DeviceError::NotXbtFirmware { chain: None })
        ));
    }

    #[test]
    fn firmware_of_another_chain_is_rejected() {
        let mut device = device_replying_with(vec![version_information_reply("1", Some("BTC"))]);
        let firmware_check = device.require_xbt_firmware();
        let Err(DeviceError::NotXbtFirmware { chain }) = firmware_check else {
            panic!("expected a firmware that is not XBT, got {firmware_check:?}");
        };
        assert_eq!(chain.as_deref(), Some("BTC"));
    }
}
