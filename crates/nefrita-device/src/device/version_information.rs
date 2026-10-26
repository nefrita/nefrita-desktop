use serde::Deserialize;

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;
use crate::protocol::WITHOUT_PARAMETERS;

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

impl<Connection: DeviceConnection> Device<Connection> {
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
}

#[cfg(test)]
mod tests;
