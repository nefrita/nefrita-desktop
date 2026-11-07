mod custom_pin_server_details;

use std::iter;

use nefrita_device::{
    CustomPinServer, CustomPinServerCheckError, DeviceError, ErrorReplyKind, HttpPinServer,
    PinServerError,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::device_connection::ConnectedDeviceSlot;

use custom_pin_server_details::{CustomPinServerDetails, checked_custom_pin_server};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum PinServerChoiceError {
    NoConnectedDevice,
    ConnectionLost,
    DeclinedOnBoard,
    InvalidUrl,
    InvalidOnionUrl,
    InvalidPublicKey,
    InvalidCertificate,
    PinServerUnreachable,
    PinServerNeedsTor,
    DeviceRejectedDetails,
    DeviceNotRecognized,
    CommandInterrupted,
}

#[tauri::command]
pub async fn set_custom_pin_server(
    app_handle: AppHandle,
    custom_pin_server_details: CustomPinServerDetails,
) -> Result<(), PinServerChoiceError> {
    tauri::async_runtime::spawn_blocking(move || {
        let custom_pin_server = checked_custom_pin_server(custom_pin_server_details)?;
        check_reachability_of(&custom_pin_server)?;
        app_handle
            .state::<ConnectedDeviceSlot>()
            .with_stored_device(|stored_device| {
                stored_device
                    .set_custom_pin_server(&custom_pin_server)
                    .map_err(|update_failure| choice_error_from_device_failure(&update_failure))
            })
            .unwrap_or(Err(PinServerChoiceError::NoConnectedDevice))
    })
    .await
    .map_err(|_| PinServerChoiceError::CommandInterrupted)?
}

#[tauri::command]
pub async fn reset_pin_server(app_handle: AppHandle) -> Result<(), PinServerChoiceError> {
    tauri::async_runtime::spawn_blocking(move || {
        app_handle
            .state::<ConnectedDeviceSlot>()
            .with_stored_device(|stored_device| {
                stored_device
                    .reset_pin_server()
                    .map_err(|reset_failure| choice_error_from_device_failure(&reset_failure))
            })
            .unwrap_or(Err(PinServerChoiceError::NoConnectedDevice))
    })
    .await
    .map_err(|_| PinServerChoiceError::CommandInterrupted)?
}

fn check_reachability_of(custom_pin_server: &CustomPinServer) -> Result<(), PinServerChoiceError> {
    let pin_server_base_urls: Vec<String> = iter::once(&custom_pin_server.url)
        .chain(&custom_pin_server.onion_url)
        .cloned()
        .collect();
    let root_certificates: Vec<String> = custom_pin_server.certificate.iter().cloned().collect();
    HttpPinServer::direct()
        .check_reachability(&pin_server_base_urls, &root_certificates)
        .map_err(|reachability_failure| {
            choice_error_from_reachability_failure(&reachability_failure)
        })
}

fn choice_error_from_check_error(check_error: &CustomPinServerCheckError) -> PinServerChoiceError {
    match check_error {
        CustomPinServerCheckError::InvalidUrl(_) => PinServerChoiceError::InvalidUrl,
        CustomPinServerCheckError::InvalidOnionUrl(_) => PinServerChoiceError::InvalidOnionUrl,
        CustomPinServerCheckError::InvalidPublicKey => PinServerChoiceError::InvalidPublicKey,
        CustomPinServerCheckError::InvalidCertificate(_) => {
            PinServerChoiceError::InvalidCertificate
        }
    }
}

fn choice_error_from_reachability_failure(
    reachability_failure: &PinServerError,
) -> PinServerChoiceError {
    match reachability_failure {
        PinServerError::NoUsableUrl(_) => PinServerChoiceError::PinServerNeedsTor,
        PinServerError::InvalidUrl(_) => PinServerChoiceError::InvalidUrl,
        PinServerError::InvalidCertificate(_) => PinServerChoiceError::InvalidCertificate,
        PinServerError::RequestFailed(_)
        | PinServerError::UnexpectedHttpStatus(_)
        | PinServerError::MalformedReply(_)
        | PinServerError::NoOnionUrl(_)
        | PinServerError::TorProxyUnreachable(_) => PinServerChoiceError::PinServerUnreachable,
    }
}

fn choice_error_from_device_failure(device_failure: &DeviceError) -> PinServerChoiceError {
    match device_failure {
        DeviceError::ConnectionFailed(_) | DeviceError::ConnectionClosed => {
            PinServerChoiceError::ConnectionLost
        }
        DeviceError::Rejected(rejection_reply) => {
            choice_error_from_rejection_kind(rejection_reply.kind())
        }
        DeviceError::RequestNotConfirmed { .. } => PinServerChoiceError::DeclinedOnBoard,
        DeviceError::InvalidCustomPinServer(check_error) => {
            choice_error_from_check_error(check_error)
        }
        DeviceError::ReplyTimedOut
        | DeviceError::RequestEncodingFailed(_)
        | DeviceError::MalformedMessage(_)
        | DeviceError::ReplyWithoutResult
        | DeviceError::NotXbtFirmware { .. }
        | DeviceError::PinServerFailed(_)
        | DeviceError::UnsupportedPinServerRequest(_) => PinServerChoiceError::DeviceNotRecognized,
    }
}

fn choice_error_from_rejection_kind(rejection_kind: ErrorReplyKind) -> PinServerChoiceError {
    match rejection_kind {
        ErrorReplyKind::UserCancelled => PinServerChoiceError::DeclinedOnBoard,
        ErrorReplyKind::InvalidParameters
        | ErrorReplyKind::NetworkMismatch
        | ErrorReplyKind::DeviceLocked
        | ErrorReplyKind::InvalidRequest
        | ErrorReplyKind::UnknownMethod
        | ErrorReplyKind::InternalError
        | ErrorReplyKind::ProtocolError
        | ErrorReplyKind::UnrecognizedCode => PinServerChoiceError::DeviceRejectedDetails,
    }
}

#[cfg(test)]
mod tests;
