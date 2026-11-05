use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use nefrita_device::{
    Device, DeviceError, DeviceState, DiyBoardPort, SerialConnection, VersionInformation,
    find_diy_board_ports,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct ConnectedDeviceSlot {
    stored_device: Mutex<Option<Device<SerialConnection>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ConnectedDevice {
    port_name: String,
    firmware_version: String,
    board_type: String,
    device_state: ConnectedDeviceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum ConnectedDeviceState {
    Uninitialized,
    Unsaved,
    Locked,
    Ready,
    Temporary,
    TemporaryUnlocked,
    UnlockedPossiblyTemporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum DeviceConnectionError {
    PortEnumerationFailed,
    PortNotListed,
    PortUnavailable,
    PortAccessDenied,
    PortOpenFailed,
    DeviceNotAnswering,
    DeviceNotRecognized,
    NotXbtFirmware,
    ConnectionLost,
    CommandInterrupted,
}

#[tauri::command]
pub async fn connect_device(
    app_handle: AppHandle,
    port_name: String,
) -> Result<ConnectedDevice, DeviceConnectionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle.state::<ConnectedDeviceSlot>().connect(port_name)
    })
    .await
    .map_err(|_| DeviceConnectionError::CommandInterrupted)?
}

#[tauri::command]
pub async fn disconnect_device(app_handle: AppHandle) -> Result<(), DeviceConnectionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle.state::<ConnectedDeviceSlot>().disconnect();
    })
    .await
    .map_err(|_| DeviceConnectionError::CommandInterrupted)
}

impl ConnectedDeviceSlot {
    fn connect(&self, port_name: String) -> Result<ConnectedDevice, DeviceConnectionError> {
        let mut stored_device = self.lock_stored_device();
        *stored_device = None;
        require_listed_port(&port_name, find_diy_board_ports())?;
        let mut opened_device = Device::connect_serial(&port_name, REPLY_TIMEOUT)
            .map_err(|opening_failure| connection_error_from_port_opening(&opening_failure))?;
        let version_information = opened_device
            .require_xbt_firmware()
            .map_err(|request_failure| connection_error_from_version_request(&request_failure))?;
        *stored_device = Some(opened_device);
        Ok(ConnectedDevice::from_version_information(
            port_name,
            version_information,
        ))
    }

    fn disconnect(&self) {
        *self.lock_stored_device() = None;
    }

    pub fn with_stored_device<DeviceUseOutcome>(
        &self,
        use_stored_device: impl FnOnce(&mut Device<SerialConnection>) -> DeviceUseOutcome,
    ) -> Option<DeviceUseOutcome> {
        self.lock_stored_device().as_mut().map(use_stored_device)
    }

    fn lock_stored_device(&self) -> MutexGuard<'_, Option<Device<SerialConnection>>> {
        self.stored_device
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl ConnectedDevice {
    fn from_version_information(
        port_name: String,
        version_information: VersionInformation,
    ) -> Self {
        let device_state = ConnectedDeviceState::from(&version_information);
        Self {
            port_name,
            firmware_version: version_information.firmware_version,
            board_type: version_information.board_type,
            device_state,
        }
    }
}

impl From<&VersionInformation> for ConnectedDeviceState {
    fn from(version_information: &VersionInformation) -> Self {
        match (
            version_information.device_state,
            version_information.has_temporary_wallet,
        ) {
            (DeviceState::Uninitialized, _) => Self::Uninitialized,
            (DeviceState::Unsaved, _) => Self::Unsaved,
            (DeviceState::Locked, _) => Self::Locked,
            (DeviceState::Ready, Some(false)) => Self::Ready,
            (DeviceState::Ready, Some(true)) => Self::TemporaryUnlocked,
            (DeviceState::Ready, None) => Self::UnlockedPossiblyTemporary,
            (DeviceState::Temporary, _) => Self::Temporary,
        }
    }
}

fn require_listed_port(
    port_name: &str,
    search_attempt: io::Result<Vec<DiyBoardPort>>,
) -> Result<(), DeviceConnectionError> {
    let diy_board_ports =
        search_attempt.map_err(|_| DeviceConnectionError::PortEnumerationFailed)?;
    if diy_board_ports
        .iter()
        .any(|diy_board_port| diy_board_port.port_name == port_name)
    {
        Ok(())
    } else {
        Err(DeviceConnectionError::PortNotListed)
    }
}

fn connection_error_from_port_opening(opening_failure: &DeviceError) -> DeviceConnectionError {
    match opening_failure {
        DeviceError::ConnectionFailed(io_failure) => match io_failure.kind() {
            io::ErrorKind::NotFound => DeviceConnectionError::PortUnavailable,
            io::ErrorKind::PermissionDenied => DeviceConnectionError::PortAccessDenied,
            _ => DeviceConnectionError::PortOpenFailed,
        },
        _ => DeviceConnectionError::PortOpenFailed,
    }
}

fn connection_error_from_version_request(request_failure: &DeviceError) -> DeviceConnectionError {
    match request_failure {
        DeviceError::ConnectionFailed(_) | DeviceError::ConnectionClosed => {
            DeviceConnectionError::ConnectionLost
        }
        DeviceError::ReplyTimedOut => DeviceConnectionError::DeviceNotAnswering,
        DeviceError::NotXbtFirmware { .. } => DeviceConnectionError::NotXbtFirmware,
        DeviceError::RequestEncodingFailed(_)
        | DeviceError::MalformedMessage(_)
        | DeviceError::UnexpectedReplyIdentifier { .. }
        | DeviceError::ReplyWithoutResult
        | DeviceError::Rejected(_)
        | DeviceError::RequestNotConfirmed { .. }
        | DeviceError::PinServerFailed(_)
        | DeviceError::UnsupportedPinServerRequest(_) => DeviceConnectionError::DeviceNotRecognized,
    }
}

#[cfg(test)]
mod tests;
