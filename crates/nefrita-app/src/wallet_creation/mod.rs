use nefrita_device::{
    Device, DeviceError, ErrorReplyKind, HttpPinServer, PinServer, PinServerError,
    PinServerPayload, PinServerRequest, SerialConnection, UnlockOutcome,
};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::device_connection::{ConnectedDeviceSlot, ConnectedDeviceState};
use crate::device_unlock::{
    DeviceUnlockError, StepReportingPinServer, UNLOCK_NETWORK, UnlockStep, read_device_state,
    report_unlock_step_to_page, unlock_error_from_device_failure,
};

const PIN_REGISTRATION_PATH: &str = "/set_pin";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WalletCreation {
    outcome: WalletCreationOutcome,
    device_state: ConnectedDeviceState,
    pin_server_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum WalletCreationOutcome {
    Saved,
    CreationCancelled,
    PinEntryCancelled,
    WalletAlreadyOnBoard,
}

struct UrlRecordingPinServer<InnerPinServer> {
    inner_pin_server: InnerPinServer,
    recorded_pin_server_url: Option<String>,
}

#[tauri::command]
pub async fn create_wallet(
    app_handle: AppHandle,
    creation_step_channel: Channel<UnlockStep>,
) -> Result<WalletCreation, DeviceUnlockError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle
            .state::<ConnectedDeviceSlot>()
            .with_stored_device(|stored_device| {
                attempt_wallet_creation(stored_device, |creation_step| {
                    report_unlock_step_to_page(&creation_step_channel, creation_step)
                })
            })
            .unwrap_or(Err(DeviceUnlockError::NoConnectedDevice))
    })
    .await
    .map_err(|_| DeviceUnlockError::CommandInterrupted)?
}

impl<InnerPinServer: PinServer> PinServer for UrlRecordingPinServer<InnerPinServer> {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        if self.recorded_pin_server_url.is_none() {
            self.recorded_pin_server_url = pin_server_request
                .urls
                .first()
                .map(String::as_str)
                .map(pin_server_url_without_registration_path);
        }
        self.inner_pin_server.exchange(pin_server_request)
    }
}

fn attempt_wallet_creation(
    stored_device: &mut Device<SerialConnection>,
    report_creation_step: impl FnMut(UnlockStep),
) -> Result<WalletCreation, DeviceUnlockError> {
    let device_state_before_creation = read_device_state(stored_device)?;
    if !needs_wallet_creation(device_state_before_creation) {
        return Ok(WalletCreation {
            outcome: WalletCreationOutcome::WalletAlreadyOnBoard,
            device_state: device_state_before_creation,
            pin_server_url: None,
        });
    }
    let mut step_reporting_pin_server = StepReportingPinServer {
        inner_pin_server: UrlRecordingPinServer {
            inner_pin_server: HttpPinServer::direct(),
            recorded_pin_server_url: None,
        },
        report_unlock_step: report_creation_step,
    };
    let creation_attempt = stored_device.unlock(UNLOCK_NETWORK, &mut step_reporting_pin_server);
    let creation_outcome = creation_outcome_from_unlock_attempt(creation_attempt)?;
    Ok(WalletCreation {
        outcome: creation_outcome,
        device_state: read_device_state(stored_device)?,
        pin_server_url: step_reporting_pin_server
            .inner_pin_server
            .recorded_pin_server_url,
    })
}

fn needs_wallet_creation(device_state: ConnectedDeviceState) -> bool {
    matches!(
        device_state,
        ConnectedDeviceState::Uninitialized | ConnectedDeviceState::Unsaved
    )
}

fn creation_outcome_from_unlock_attempt(
    unlock_attempt: Result<UnlockOutcome, DeviceError>,
) -> Result<WalletCreationOutcome, DeviceUnlockError> {
    match unlock_attempt {
        Ok(UnlockOutcome::Unlocked) => Ok(WalletCreationOutcome::Saved),
        Ok(UnlockOutcome::Refused) => Ok(WalletCreationOutcome::CreationCancelled),
        Err(DeviceError::Rejected(rejection_reply))
            if rejection_reply.kind() == ErrorReplyKind::UserCancelled =>
        {
            Ok(WalletCreationOutcome::PinEntryCancelled)
        }
        Err(unlock_failure) => Err(unlock_error_from_device_failure(&unlock_failure)),
    }
}

fn pin_server_url_without_registration_path(requested_url: &str) -> String {
    requested_url
        .strip_suffix(PIN_REGISTRATION_PATH)
        .unwrap_or(requested_url)
        .to_owned()
}

#[cfg(test)]
mod tests;
