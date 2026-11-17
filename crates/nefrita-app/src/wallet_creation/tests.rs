use serde_json::json;

use super::*;
use crate::device_connection::scripted::rejection_with_code;

const PIN_SERVER_BASE_URL: &str = "http://localhost:8096";
const OTHER_PIN_SERVER_BASE_URL: &str = "http://localhost:8097";
const PIN_SERVER_ONION_BASE_URL: &str = "http://examplepinserver.onion";
const DEVICE_ENCRYPTED_MESSAGE: &str = "the encrypted message of the device";
const PIN_SERVER_ENCRYPTED_MESSAGE: &str = "the encrypted message of the server";

struct AnsweringPinServer;

impl PinServer for AnsweringPinServer {
    fn exchange(
        &mut self,
        _pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        Ok(pin_server_reply())
    }
}

fn pin_server_reply() -> PinServerPayload {
    PinServerPayload {
        encrypted_message: PIN_SERVER_ENCRYPTED_MESSAGE.to_owned(),
    }
}

fn pin_registration_request_to(pin_server_base_url: &str) -> PinServerRequest {
    PinServerRequest {
        urls: vec![
            format!("{pin_server_base_url}/set_pin"),
            format!("{PIN_SERVER_ONION_BASE_URL}/set_pin"),
        ],
        root_certificates: Vec::new(),
        device_payload: PinServerPayload {
            encrypted_message: DEVICE_ENCRYPTED_MESSAGE.to_owned(),
        },
    }
}

fn url_recording_pin_server_around<InnerPinServer>(
    inner_pin_server: InnerPinServer,
) -> UrlRecordingPinServer<InnerPinServer> {
    UrlRecordingPinServer {
        inner_pin_server,
        recorded_pin_server_url: None,
    }
}

#[test]
fn pin_registration_url_loses_its_path() {
    assert_eq!(
        pin_server_url_without_registration_path(&format!("{PIN_SERVER_BASE_URL}/set_pin")),
        PIN_SERVER_BASE_URL
    );
}

#[test]
fn url_without_the_registration_path_stays_as_it_is() {
    assert_eq!(
        pin_server_url_without_registration_path(PIN_SERVER_BASE_URL),
        PIN_SERVER_BASE_URL
    );
}

#[test]
fn only_the_main_url_of_the_first_pin_server_request_is_recorded() {
    let mut url_recording_pin_server = url_recording_pin_server_around(AnsweringPinServer);
    for pin_server_base_url in [PIN_SERVER_BASE_URL, OTHER_PIN_SERVER_BASE_URL] {
        url_recording_pin_server
            .exchange(&pin_registration_request_to(pin_server_base_url))
            .expect("the scripted exchange should succeed");
    }
    assert_eq!(
        url_recording_pin_server.recorded_pin_server_url.as_deref(),
        Some(PIN_SERVER_BASE_URL)
    );
}

#[test]
fn the_pin_server_reply_passes_through_the_url_recording() {
    assert_eq!(
        url_recording_pin_server_around(AnsweringPinServer)
            .exchange(&pin_registration_request_to(PIN_SERVER_BASE_URL)),
        Ok(pin_server_reply())
    );
}

#[test]
fn boards_without_a_wallet_or_with_an_unsaved_one_need_the_creation() {
    let creation_needs_by_device_state = [
        (ConnectedDeviceState::Uninitialized, true),
        (ConnectedDeviceState::Unsaved, true),
        (ConnectedDeviceState::Locked, false),
        (ConnectedDeviceState::Ready, false),
        (ConnectedDeviceState::Temporary, false),
        (ConnectedDeviceState::TemporaryUnlocked, false),
        (ConnectedDeviceState::UnlockedPossiblyTemporary, false),
    ];
    for (device_state, expected_creation_need) in creation_needs_by_device_state {
        assert_eq!(
            needs_wallet_creation(device_state),
            expected_creation_need,
            "{device_state:?}"
        );
    }
}

#[test]
fn unlock_attempts_become_their_creation_outcomes() {
    let creation_outcomes_by_unlock_attempt = [
        (Ok(UnlockOutcome::Unlocked), WalletCreationOutcome::Saved),
        (
            Ok(UnlockOutcome::Refused),
            WalletCreationOutcome::CreationCancelled,
        ),
        (
            Err(rejection_with_code(-32000)),
            WalletCreationOutcome::PinEntryCancelled,
        ),
    ];
    for (unlock_attempt, expected_creation_outcome) in creation_outcomes_by_unlock_attempt {
        assert_eq!(
            creation_outcome_from_unlock_attempt(unlock_attempt),
            Ok(expected_creation_outcome),
            "{expected_creation_outcome:?}"
        );
    }
}

#[test]
fn other_device_failures_become_their_unlock_errors() {
    let unlock_errors_by_device_failure = [
        (
            DeviceError::ConnectionClosed,
            DeviceUnlockError::ConnectionLost,
        ),
        (
            rejection_with_code(-32602),
            DeviceUnlockError::DeviceRejectedRequest,
        ),
        (
            DeviceError::PinServerFailed(PinServerError::UnexpectedHttpStatus(500)),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
    ];
    for (device_failure, expected_unlock_error) in unlock_errors_by_device_failure {
        let device_failure_description = device_failure.to_string();
        assert_eq!(
            creation_outcome_from_unlock_attempt(Err(device_failure)),
            Err(expected_unlock_error),
            "{device_failure_description}"
        );
    }
}

#[test]
fn wallet_creation_serializes_with_the_field_names_that_typescript_reads() {
    let wallet_creation = WalletCreation {
        outcome: WalletCreationOutcome::Saved,
        device_state: ConnectedDeviceState::Ready,
        pin_server_url: Some(PIN_SERVER_BASE_URL.to_owned()),
    };
    assert_eq!(
        serde_json::to_value(wallet_creation).expect("a wallet creation should serialize"),
        json!({
            "outcome": "saved",
            "deviceState": "ready",
            "pinServerUrl": PIN_SERVER_BASE_URL,
        })
    );
}

#[test]
fn wallet_creation_outcomes_serialize_as_their_names() {
    let names_by_creation_outcome = [
        (WalletCreationOutcome::Saved, "saved"),
        (
            WalletCreationOutcome::CreationCancelled,
            "creation-cancelled",
        ),
        (
            WalletCreationOutcome::PinEntryCancelled,
            "pin-entry-cancelled",
        ),
        (
            WalletCreationOutcome::WalletAlreadyOnBoard,
            "wallet-already-on-board",
        ),
    ];
    for (creation_outcome, expected_name) in names_by_creation_outcome {
        assert_eq!(
            serde_json::to_value(creation_outcome).expect("a creation outcome should serialize"),
            json!(expected_name)
        );
    }
}
