use serde::{Serialize, Serializer};

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;

const PIN_SERVER_PUBLIC_KEY_LENGTH: usize = 33;

const RESET_DETAILS_AND_CERTIFICATE: PinServerReset = PinServerReset {
    reset_details: true,
    reset_certificate: true,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomPinServer {
    #[serde(rename = "urlA")]
    pub url: String,
    #[serde(rename = "urlB", skip_serializing_if = "Option::is_none")]
    pub onion_url: Option<String>,
    #[serde(rename = "pubkey", serialize_with = "serialize_public_key_as_bytes")]
    pub public_key: [u8; PIN_SERVER_PUBLIC_KEY_LENGTH],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
}

#[derive(Serialize)]
struct PinServerReset {
    reset_details: bool,
    reset_certificate: bool,
}

impl<Connection: DeviceConnection> Device<Connection> {
    pub fn set_custom_pin_server(
        &mut self,
        custom_pin_server: &CustomPinServer,
    ) -> Result<(), DeviceError> {
        self.send_request_requiring_confirmation("update_pinserver", Some(custom_pin_server))
    }

    pub fn reset_pin_server(&mut self) -> Result<(), DeviceError> {
        self.send_request_requiring_confirmation(
            "update_pinserver",
            Some(&RESET_DETAILS_AND_CERTIFICATE),
        )
    }
}

fn serialize_public_key_as_bytes<FieldSerializer: Serializer>(
    public_key: &[u8; PIN_SERVER_PUBLIC_KEY_LENGTH],
    serializer: FieldSerializer,
) -> Result<FieldSerializer::Ok, FieldSerializer::Error> {
    serializer.serialize_bytes(public_key)
}

#[cfg(test)]
mod tests;
