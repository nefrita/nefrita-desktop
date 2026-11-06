use nefrita_device::{
    Device, DeviceError, ErrorReplyKind, HttpPinServer, Network, PinServer, PinServerError,
    PinServerPayload, PinServerRequest, SerialConnection, UnlockOutcome, VersionInformation,
};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::device_connection::{ConnectedDeviceSlot, ConnectedDeviceState};

const UNLOCK_NETWORK: Network = Network::XbtTestnet4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum UnlockStep {
    PinServerExchange,
    PinServerReplyCheck,
    RefusedPin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct UnlockAttempt {
    outcome: UnlockAttemptOutcome,
    device_state: ConnectedDeviceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum UnlockAttemptOutcome {
    Unlocked,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum DeviceUnlockError {
    NoConnectedDevice,
    ConnectionLost,
    DeviceNotAnswering,
    PinEntryCancelled,
    NetworkMismatch,
    TemporaryWalletInUse,
    PinServerExchangeFailed,
    PinServerNeedsTor,
    PinServerMisconfigured,
    UnsupportedPinServerRequest,
    DeviceRejectedRequest,
    DeviceNotRecognized,
    CommandInterrupted,
}

struct StepReportingPinServer<InnerPinServer, UnlockStepReporter> {
    inner_pin_server: InnerPinServer,
    report_unlock_step: UnlockStepReporter,
}

#[tauri::command]
pub async fn unlock_device(
    app_handle: AppHandle,
    unlock_step_channel: Channel<UnlockStep>,
) -> Result<UnlockAttempt, DeviceUnlockError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle
            .state::<ConnectedDeviceSlot>()
            .with_stored_device(|stored_device| {
                attempt_unlock(stored_device, |unlock_step| {
                    report_unlock_step_to_page(&unlock_step_channel, unlock_step)
                })
            })
            .unwrap_or(Err(DeviceUnlockError::NoConnectedDevice))
    })
    .await
    .map_err(|_| DeviceUnlockError::CommandInterrupted)?
}

#[tauri::command]
pub async fn logout_device(
    app_handle: AppHandle,
) -> Result<ConnectedDeviceState, DeviceUnlockError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle
            .state::<ConnectedDeviceSlot>()
            .with_stored_device(device_state_after_logout)
            .unwrap_or(Err(DeviceUnlockError::NoConnectedDevice))
    })
    .await
    .map_err(|_| DeviceUnlockError::CommandInterrupted)?
}

impl<InnerPinServer: PinServer, UnlockStepReporter: FnMut(UnlockStep)> PinServer
    for StepReportingPinServer<InnerPinServer, UnlockStepReporter>
{
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        (self.report_unlock_step)(UnlockStep::PinServerExchange);
        let exchange_attempt = self.inner_pin_server.exchange(pin_server_request);
        (self.report_unlock_step)(UnlockStep::PinServerReplyCheck);
        exchange_attempt
    }
}

impl From<UnlockOutcome> for UnlockAttemptOutcome {
    fn from(unlock_outcome: UnlockOutcome) -> Self {
        match unlock_outcome {
            UnlockOutcome::Unlocked => Self::Unlocked,
            UnlockOutcome::Refused => Self::Refused,
        }
    }
}

fn attempt_unlock(
    stored_device: &mut Device<SerialConnection>,
    mut report_unlock_step: impl FnMut(UnlockStep),
) -> Result<UnlockAttempt, DeviceUnlockError> {
    let attempt_outcome = unlock_reporting_steps(stored_device, &mut report_unlock_step)?;
    let device_state = match attempt_outcome {
        UnlockAttemptOutcome::Unlocked => read_device_state(stored_device)?,
        UnlockAttemptOutcome::Refused => {
            report_unlock_step(UnlockStep::RefusedPin);
            wait_for_device_state(stored_device)?
        }
    };
    Ok(UnlockAttempt {
        outcome: attempt_outcome,
        device_state,
    })
}

fn unlock_reporting_steps(
    stored_device: &mut Device<SerialConnection>,
    report_unlock_step: impl FnMut(UnlockStep),
) -> Result<UnlockAttemptOutcome, DeviceUnlockError> {
    let mut step_reporting_pin_server = StepReportingPinServer {
        inner_pin_server: HttpPinServer::direct(),
        report_unlock_step,
    };
    stored_device
        .unlock(UNLOCK_NETWORK, &mut step_reporting_pin_server)
        .map(UnlockAttemptOutcome::from)
        .map_err(|unlock_failure| unlock_error_from_device_failure(&unlock_failure))
}

fn device_state_after_logout(
    stored_device: &mut Device<SerialConnection>,
) -> Result<ConnectedDeviceState, DeviceUnlockError> {
    stored_device
        .logout()
        .map_err(|logout_failure| unlock_error_from_device_failure(&logout_failure))?;
    read_device_state(stored_device)
}

fn read_device_state(
    stored_device: &mut Device<SerialConnection>,
) -> Result<ConnectedDeviceState, DeviceUnlockError> {
    device_state_from_version_request(stored_device.require_xbt_firmware())
}

fn wait_for_device_state(
    stored_device: &mut Device<SerialConnection>,
) -> Result<ConnectedDeviceState, DeviceUnlockError> {
    device_state_from_version_request(
        stored_device
            .run_without_reply_timeout(|stored_device| stored_device.require_xbt_firmware()),
    )
}

fn device_state_from_version_request(
    version_request_attempt: Result<VersionInformation, DeviceError>,
) -> Result<ConnectedDeviceState, DeviceUnlockError> {
    version_request_attempt
        .map(|version_information| ConnectedDeviceState::from(&version_information))
        .map_err(|version_request_failure| {
            unlock_error_from_device_failure(&version_request_failure)
        })
}

fn report_unlock_step_to_page(unlock_step_channel: &Channel<UnlockStep>, unlock_step: UnlockStep) {
    if let Err(delivery_failure) = unlock_step_channel.send(unlock_step) {
        eprintln!("The unlock step did not reach the page: {delivery_failure}");
    }
}

fn unlock_error_from_device_failure(device_failure: &DeviceError) -> DeviceUnlockError {
    match device_failure {
        DeviceError::ConnectionFailed(_) | DeviceError::ConnectionClosed => {
            DeviceUnlockError::ConnectionLost
        }
        DeviceError::ReplyTimedOut => DeviceUnlockError::DeviceNotAnswering,
        DeviceError::Rejected(rejection_reply) => {
            unlock_error_from_rejection_kind(rejection_reply.kind())
        }
        DeviceError::PinServerFailed(pin_server_failure) => {
            unlock_error_from_pin_server_failure(pin_server_failure)
        }
        DeviceError::UnsupportedPinServerRequest(_) => {
            DeviceUnlockError::UnsupportedPinServerRequest
        }
        DeviceError::RequestNotConfirmed { .. } => DeviceUnlockError::DeviceRejectedRequest,
        DeviceError::RequestEncodingFailed(_)
        | DeviceError::MalformedMessage(_)
        | DeviceError::ReplyWithoutResult
        | DeviceError::NotXbtFirmware { .. } => DeviceUnlockError::DeviceNotRecognized,
    }
}

fn unlock_error_from_rejection_kind(rejection_kind: ErrorReplyKind) -> DeviceUnlockError {
    match rejection_kind {
        ErrorReplyKind::UserCancelled => DeviceUnlockError::PinEntryCancelled,
        ErrorReplyKind::NetworkMismatch => DeviceUnlockError::NetworkMismatch,
        ErrorReplyKind::DeviceLocked => DeviceUnlockError::TemporaryWalletInUse,
        ErrorReplyKind::InvalidRequest
        | ErrorReplyKind::UnknownMethod
        | ErrorReplyKind::InvalidParameters
        | ErrorReplyKind::InternalError
        | ErrorReplyKind::ProtocolError
        | ErrorReplyKind::UnrecognizedCode => DeviceUnlockError::DeviceRejectedRequest,
    }
}

fn unlock_error_from_pin_server_failure(pin_server_failure: &PinServerError) -> DeviceUnlockError {
    match pin_server_failure {
        PinServerError::NoUsableUrl(_) => DeviceUnlockError::PinServerNeedsTor,
        PinServerError::InvalidUrl(_) | PinServerError::InvalidCertificate(_) => {
            DeviceUnlockError::PinServerMisconfigured
        }
        PinServerError::RequestFailed(_)
        | PinServerError::UnexpectedHttpStatus(_)
        | PinServerError::MalformedReply(_)
        | PinServerError::NoOnionUrl(_)
        | PinServerError::TorProxyUnreachable(_) => DeviceUnlockError::PinServerExchangeFailed,
    }
}

#[cfg(test)]
mod tests;
