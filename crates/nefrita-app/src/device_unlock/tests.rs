use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use nefrita_device::ErrorReply;
use serde_json::json;

use super::*;

const PIN_SERVER_URL: &str = "http://localhost:8096";
const PIN_SERVER_ONION_URL: &str = "http://examplepinserver.onion";
const DEVICE_ENCRYPTED_MESSAGE: &str = "the encrypted message of the device";
const PIN_SERVER_ENCRYPTED_MESSAGE: &str = "the encrypted message of the server";

type RecordedEventLog = Rc<RefCell<Vec<RecordedEvent>>>;

#[derive(Debug, PartialEq, Eq)]
enum RecordedEvent {
    ReportedStep(UnlockStep),
    InnerExchange(PinServerRequest),
}

struct ScriptedPinServer {
    exchange_attempt: Result<PinServerPayload, PinServerError>,
    recorded_events: RecordedEventLog,
}

impl PinServer for ScriptedPinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        self.recorded_events
            .borrow_mut()
            .push(RecordedEvent::InnerExchange(pin_server_request.clone()));
        self.exchange_attempt.clone()
    }
}

fn device_request() -> PinServerRequest {
    PinServerRequest {
        urls: vec![PIN_SERVER_URL.to_owned()],
        root_certificates: Vec::new(),
        device_payload: PinServerPayload {
            encrypted_message: DEVICE_ENCRYPTED_MESSAGE.to_owned(),
        },
    }
}

fn pin_server_reply() -> PinServerPayload {
    PinServerPayload {
        encrypted_message: PIN_SERVER_ENCRYPTED_MESSAGE.to_owned(),
    }
}

fn record_reported_steps_in(recorded_events: RecordedEventLog) -> impl FnMut(UnlockStep) {
    move |unlock_step| {
        recorded_events
            .borrow_mut()
            .push(RecordedEvent::ReportedStep(unlock_step))
    }
}

fn step_reporting_pin_server_around(
    exchange_attempt: Result<PinServerPayload, PinServerError>,
    recorded_events: &RecordedEventLog,
) -> StepReportingPinServer<ScriptedPinServer, impl FnMut(UnlockStep)> {
    StepReportingPinServer {
        inner_pin_server: ScriptedPinServer {
            exchange_attempt,
            recorded_events: Rc::clone(recorded_events),
        },
        report_unlock_step: record_reported_steps_in(Rc::clone(recorded_events)),
    }
}

fn events_around_one_exchange() -> [RecordedEvent; 3] {
    [
        RecordedEvent::ReportedStep(UnlockStep::PinServerExchange),
        RecordedEvent::InnerExchange(device_request()),
        RecordedEvent::ReportedStep(UnlockStep::PinServerReplyCheck),
    ]
}

fn rejection_with_code(rejection_code: i64) -> DeviceError {
    DeviceError::Rejected(ErrorReply {
        code: rejection_code,
        description: "the device rejected the request".to_owned(),
    })
}

#[test]
fn each_exchange_reports_its_two_steps_in_order() {
    let recorded_events = RecordedEventLog::default();
    let mut step_reporting_pin_server =
        step_reporting_pin_server_around(Ok(pin_server_reply()), &recorded_events);
    step_reporting_pin_server
        .exchange(&device_request())
        .expect("the scripted exchange should succeed");
    assert_eq!(*recorded_events.borrow(), events_around_one_exchange());
}

#[test]
fn the_pin_server_reply_passes_through_unchanged() {
    let recorded_events = RecordedEventLog::default();
    let mut step_reporting_pin_server =
        step_reporting_pin_server_around(Ok(pin_server_reply()), &recorded_events);
    assert_eq!(
        step_reporting_pin_server.exchange(&device_request()),
        Ok(pin_server_reply())
    );
}

#[test]
fn a_pin_server_failure_passes_through_after_both_steps_are_reported() {
    let recorded_events = RecordedEventLog::default();
    let pin_server_failure = PinServerError::UnexpectedHttpStatus(500);
    let mut step_reporting_pin_server =
        step_reporting_pin_server_around(Err(pin_server_failure.clone()), &recorded_events);
    assert_eq!(
        step_reporting_pin_server.exchange(&device_request()),
        Err(pin_server_failure)
    );
    assert_eq!(*recorded_events.borrow(), events_around_one_exchange());
}

#[test]
fn unlock_outcomes_become_their_attempt_outcomes() {
    let attempt_outcomes_by_unlock_outcome = [
        (UnlockOutcome::Unlocked, UnlockAttemptOutcome::Unlocked),
        (UnlockOutcome::Refused, UnlockAttemptOutcome::Refused),
    ];
    for (unlock_outcome, expected_attempt_outcome) in attempt_outcomes_by_unlock_outcome {
        assert_eq!(
            UnlockAttemptOutcome::from(unlock_outcome),
            expected_attempt_outcome
        );
    }
}

#[test]
fn device_failures_become_their_unlock_errors() {
    let unlock_errors_by_device_failure = [
        (
            DeviceError::ConnectionFailed(io::Error::from(io::ErrorKind::BrokenPipe)),
            DeviceUnlockError::ConnectionLost,
        ),
        (
            DeviceError::ConnectionClosed,
            DeviceUnlockError::ConnectionLost,
        ),
        (
            DeviceError::ReplyTimedOut,
            DeviceUnlockError::DeviceNotAnswering,
        ),
        (
            DeviceError::UnsupportedPinServerRequest("it uses the HTTP method GET".to_owned()),
            DeviceUnlockError::UnsupportedPinServerRequest,
        ),
        (
            DeviceError::RequestNotConfirmed {
                method_name: "logout".to_owned(),
            },
            DeviceUnlockError::DeviceRejectedRequest,
        ),
        (
            DeviceError::RequestEncodingFailed("the request is too large".to_owned()),
            DeviceUnlockError::DeviceNotRecognized,
        ),
        (
            DeviceError::MalformedMessage("the reply is not CBOR".to_owned()),
            DeviceUnlockError::DeviceNotRecognized,
        ),
        (
            DeviceError::UnexpectedReplyIdentifier {
                expected: "1".to_owned(),
                received: "7".to_owned(),
            },
            DeviceUnlockError::DeviceNotRecognized,
        ),
        (
            DeviceError::ReplyWithoutResult,
            DeviceUnlockError::DeviceNotRecognized,
        ),
        (
            DeviceError::NotXbtFirmware {
                chain: Some("BTC".to_owned()),
            },
            DeviceUnlockError::DeviceNotRecognized,
        ),
    ];
    for (device_failure, expected_unlock_error) in unlock_errors_by_device_failure {
        assert_eq!(
            unlock_error_from_device_failure(&device_failure),
            expected_unlock_error,
            "{device_failure}"
        );
    }
}

#[test]
fn device_rejections_become_their_unlock_errors() {
    let unlock_errors_by_rejection_code = [
        (-32000, DeviceUnlockError::PinEntryCancelled),
        (-32003, DeviceUnlockError::NetworkMismatch),
        (-32002, DeviceUnlockError::TemporaryWalletInUse),
        (-32600, DeviceUnlockError::DeviceRejectedRequest),
        (-32601, DeviceUnlockError::DeviceRejectedRequest),
        (-32602, DeviceUnlockError::DeviceRejectedRequest),
        (-32603, DeviceUnlockError::DeviceRejectedRequest),
        (-32001, DeviceUnlockError::DeviceRejectedRequest),
        (-32099, DeviceUnlockError::DeviceRejectedRequest),
    ];
    for (rejection_code, expected_unlock_error) in unlock_errors_by_rejection_code {
        assert_eq!(
            unlock_error_from_device_failure(&rejection_with_code(rejection_code)),
            expected_unlock_error,
            "rejection code {rejection_code}"
        );
    }
}

#[test]
fn pin_server_failures_become_their_unlock_errors() {
    let unlock_errors_by_pin_server_failure = [
        (
            PinServerError::NoUsableUrl(vec![PIN_SERVER_ONION_URL.to_owned()]),
            DeviceUnlockError::PinServerNeedsTor,
        ),
        (
            PinServerError::InvalidUrl("localhost:8096".to_owned()),
            DeviceUnlockError::PinServerMisconfigured,
        ),
        (
            PinServerError::InvalidCertificate("the certificate is not PEM".to_owned()),
            DeviceUnlockError::PinServerMisconfigured,
        ),
        (
            PinServerError::RequestFailed("the connection was refused".to_owned()),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
        (
            PinServerError::UnexpectedHttpStatus(500),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
        (
            PinServerError::MalformedReply("the reply is not JSON".to_owned()),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
        (
            PinServerError::NoOnionUrl(vec![PIN_SERVER_URL.to_owned()]),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
        (
            PinServerError::TorProxyUnreachable(SocketAddr::from(([127, 0, 0, 1], 9050))),
            DeviceUnlockError::PinServerExchangeFailed,
        ),
    ];
    for (pin_server_failure, expected_unlock_error) in unlock_errors_by_pin_server_failure {
        let device_failure = DeviceError::PinServerFailed(pin_server_failure);
        assert_eq!(
            unlock_error_from_device_failure(&device_failure),
            expected_unlock_error,
            "{device_failure}"
        );
    }
}

#[test]
fn unlock_attempt_serializes_with_the_field_names_that_typescript_reads() {
    let unlock_attempt = UnlockAttempt {
        outcome: UnlockAttemptOutcome::Unlocked,
        device_state: ConnectedDeviceState::Ready,
    };
    assert_eq!(
        serde_json::to_value(unlock_attempt).expect("an unlock attempt should serialize"),
        json!({
            "outcome": "unlocked",
            "deviceState": "ready",
        })
    );
}

#[test]
fn unlock_attempt_outcomes_serialize_as_their_names() {
    let names_by_attempt_outcome = [
        (UnlockAttemptOutcome::Unlocked, "unlocked"),
        (UnlockAttemptOutcome::Refused, "refused"),
    ];
    for (attempt_outcome, expected_name) in names_by_attempt_outcome {
        assert_eq!(
            serde_json::to_value(attempt_outcome).expect("an attempt outcome should serialize"),
            json!(expected_name)
        );
    }
}

#[test]
fn unlock_steps_serialize_as_their_names() {
    let names_by_unlock_step = [
        (UnlockStep::PinServerExchange, "pin-server-exchange"),
        (UnlockStep::PinServerReplyCheck, "pin-server-reply-check"),
        (UnlockStep::RefusedPin, "refused-pin"),
    ];
    for (unlock_step, expected_name) in names_by_unlock_step {
        assert_eq!(
            serde_json::to_value(unlock_step).expect("an unlock step should serialize"),
            json!(expected_name)
        );
    }
}

#[test]
fn unlock_errors_serialize_as_their_codes() {
    let codes_by_unlock_error = [
        (DeviceUnlockError::NoConnectedDevice, "no-connected-device"),
        (DeviceUnlockError::ConnectionLost, "connection-lost"),
        (
            DeviceUnlockError::DeviceNotAnswering,
            "device-not-answering",
        ),
        (DeviceUnlockError::PinEntryCancelled, "pin-entry-cancelled"),
        (DeviceUnlockError::NetworkMismatch, "network-mismatch"),
        (
            DeviceUnlockError::TemporaryWalletInUse,
            "temporary-wallet-in-use",
        ),
        (
            DeviceUnlockError::PinServerExchangeFailed,
            "pin-server-exchange-failed",
        ),
        (DeviceUnlockError::PinServerNeedsTor, "pin-server-needs-tor"),
        (
            DeviceUnlockError::PinServerMisconfigured,
            "pin-server-misconfigured",
        ),
        (
            DeviceUnlockError::UnsupportedPinServerRequest,
            "unsupported-pin-server-request",
        ),
        (
            DeviceUnlockError::DeviceRejectedRequest,
            "device-rejected-request",
        ),
        (
            DeviceUnlockError::DeviceNotRecognized,
            "device-not-recognized",
        ),
        (DeviceUnlockError::CommandInterrupted, "command-interrupted"),
    ];
    for (unlock_error, expected_code) in codes_by_unlock_error {
        assert_eq!(
            serde_json::to_value(unlock_error).expect("an unlock error should serialize"),
            json!(expected_code)
        );
    }
}
