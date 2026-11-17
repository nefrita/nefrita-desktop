use std::io;
use std::net::SocketAddr;

use serde_json::json;

use super::*;
use crate::device_connection::scripted::rejection_with_code;

const PIN_SERVER_URL: &str = "https://pin.example.test";
const PIN_SERVER_ONION_URL: &str = "http://pinserverexample.onion";

#[test]
fn reachability_failures_become_their_choice_errors() {
    let choice_errors_by_reachability_failure = [
        (
            PinServerError::NoUsableUrl(vec![PIN_SERVER_ONION_URL.to_owned()]),
            PinServerChoiceError::PinServerNeedsTor,
        ),
        (
            PinServerError::InvalidUrl(PIN_SERVER_URL.to_owned()),
            PinServerChoiceError::InvalidUrl,
        ),
        (
            PinServerError::InvalidCertificate("the PEM text holds no certificate".to_owned()),
            PinServerChoiceError::InvalidCertificate,
        ),
        (
            PinServerError::RequestFailed("the connection was refused".to_owned()),
            PinServerChoiceError::PinServerUnreachable,
        ),
        (
            PinServerError::UnexpectedHttpStatus(404),
            PinServerChoiceError::PinServerUnreachable,
        ),
        (
            PinServerError::MalformedReply("the reply is not JSON".to_owned()),
            PinServerChoiceError::PinServerUnreachable,
        ),
        (
            PinServerError::NoOnionUrl(vec![PIN_SERVER_URL.to_owned()]),
            PinServerChoiceError::PinServerUnreachable,
        ),
        (
            PinServerError::TorProxyUnreachable(SocketAddr::from(([127, 0, 0, 1], 9050))),
            PinServerChoiceError::PinServerUnreachable,
        ),
    ];
    for (reachability_failure, expected_choice_error) in choice_errors_by_reachability_failure {
        assert_eq!(
            choice_error_from_reachability_failure(&reachability_failure),
            expected_choice_error,
            "{reachability_failure}"
        );
    }
}

#[test]
fn device_failures_become_their_choice_errors() {
    let choice_errors_by_device_failure = [
        (
            DeviceError::ConnectionFailed(io::Error::from(io::ErrorKind::BrokenPipe)),
            PinServerChoiceError::ConnectionLost,
        ),
        (
            DeviceError::ConnectionClosed,
            PinServerChoiceError::ConnectionLost,
        ),
        (
            DeviceError::RequestNotConfirmed {
                method_name: "update_pinserver".to_owned(),
            },
            PinServerChoiceError::DeclinedOnBoard,
        ),
        (
            DeviceError::InvalidCustomPinServer(CustomPinServerCheckError::InvalidPublicKey),
            PinServerChoiceError::InvalidPublicKey,
        ),
        (
            DeviceError::ReplyTimedOut,
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::RequestEncodingFailed("the request is too large".to_owned()),
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::MalformedMessage("the reply is not CBOR".to_owned()),
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::ReplyWithoutResult,
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::NotXbtFirmware { chain: None },
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::PinServerFailed(PinServerError::UnexpectedHttpStatus(500)),
            PinServerChoiceError::DeviceNotRecognized,
        ),
        (
            DeviceError::UnsupportedPinServerRequest("an unknown request".to_owned()),
            PinServerChoiceError::DeviceNotRecognized,
        ),
    ];
    for (device_failure, expected_choice_error) in choice_errors_by_device_failure {
        assert_eq!(
            choice_error_from_device_failure(&device_failure),
            expected_choice_error,
            "{device_failure}"
        );
    }
}

#[test]
fn device_rejections_become_their_choice_errors() {
    let choice_errors_by_rejection_code = [
        (-32000, PinServerChoiceError::DeclinedOnBoard),
        (-32602, PinServerChoiceError::DeviceRejectedDetails),
        (-32003, PinServerChoiceError::DeviceRejectedDetails),
        (-32002, PinServerChoiceError::DeviceRejectedDetails),
        (-32600, PinServerChoiceError::DeviceRejectedDetails),
        (-32601, PinServerChoiceError::DeviceRejectedDetails),
        (-32603, PinServerChoiceError::DeviceRejectedDetails),
        (-32001, PinServerChoiceError::DeviceRejectedDetails),
        (-32099, PinServerChoiceError::DeviceRejectedDetails),
    ];
    for (rejection_code, expected_choice_error) in choice_errors_by_rejection_code {
        assert_eq!(
            choice_error_from_device_failure(&rejection_with_code(rejection_code)),
            expected_choice_error,
            "rejection code {rejection_code}"
        );
    }
}

#[test]
fn pin_server_choice_errors_serialize_as_their_codes() {
    let codes_by_choice_error = [
        (
            PinServerChoiceError::NoConnectedDevice,
            "no-connected-device",
        ),
        (PinServerChoiceError::ConnectionLost, "connection-lost"),
        (PinServerChoiceError::DeclinedOnBoard, "declined-on-board"),
        (PinServerChoiceError::InvalidUrl, "invalid-url"),
        (PinServerChoiceError::InvalidOnionUrl, "invalid-onion-url"),
        (PinServerChoiceError::InvalidPublicKey, "invalid-public-key"),
        (
            PinServerChoiceError::InvalidCertificate,
            "invalid-certificate",
        ),
        (
            PinServerChoiceError::PinServerUnreachable,
            "pin-server-unreachable",
        ),
        (
            PinServerChoiceError::PinServerNeedsTor,
            "pin-server-needs-tor",
        ),
        (
            PinServerChoiceError::DeviceRejectedDetails,
            "device-rejected-details",
        ),
        (
            PinServerChoiceError::DeviceNotRecognized,
            "device-not-recognized",
        ),
        (
            PinServerChoiceError::CommandInterrupted,
            "command-interrupted",
        ),
    ];
    for (choice_error, expected_code) in codes_by_choice_error {
        assert_eq!(
            serde_json::to_value(choice_error).expect("a choice error should serialize"),
            json!(expected_code)
        );
    }
}
