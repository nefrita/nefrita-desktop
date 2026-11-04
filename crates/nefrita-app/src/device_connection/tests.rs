use std::io;

use nefrita_device::{ErrorReply, PinServerError, UsbSerialChip};
use serde_json::json;

use super::*;

const PORT_NAME: &str = "/dev/cu.usbserial-0001";
const OTHER_PORT_NAME: &str = "/dev/cu.usbserial-0002";
const FIRMWARE_VERSION: &str = "1.0.36";
const BOARD_TYPE: &str = "M5StickCPlus2";
const CP210X_CHIP: UsbSerialChip = UsbSerialChip {
    name: "CP210x",
    vendor_identifier: 0x10c4,
    product_identifier: 0xea60,
};

fn cp210x_diy_board_port(port_name: &str) -> DiyBoardPort {
    DiyBoardPort {
        port_name: port_name.to_owned(),
        chip: CP210X_CHIP,
        product_name: None,
        serial_number: None,
    }
}

fn xbt_version_information(device_state: DeviceState) -> VersionInformation {
    VersionInformation {
        firmware_version: FIRMWARE_VERSION.to_owned(),
        board_type: BOARD_TYPE.to_owned(),
        device_state,
        chain: Some("XBT".to_owned()),
    }
}

fn connection_failure(failure_kind: io::ErrorKind) -> DeviceError {
    DeviceError::ConnectionFailed(io::Error::from(failure_kind))
}

#[test]
fn version_information_becomes_the_connected_device_with_its_port_name() {
    let connected_device = ConnectedDevice::from_version_information(
        PORT_NAME.to_owned(),
        xbt_version_information(DeviceState::Locked),
    );
    assert_eq!(
        connected_device,
        ConnectedDevice {
            port_name: PORT_NAME.to_owned(),
            firmware_version: FIRMWARE_VERSION.to_owned(),
            board_type: BOARD_TYPE.to_owned(),
            device_state: ConnectedDeviceState::Locked,
        }
    );
}

#[test]
fn each_device_state_becomes_its_connected_device_state() {
    let connected_device_states_by_device_state = [
        (
            DeviceState::Uninitialized,
            ConnectedDeviceState::Uninitialized,
        ),
        (DeviceState::Unsaved, ConnectedDeviceState::Unsaved),
        (DeviceState::Locked, ConnectedDeviceState::Locked),
        (DeviceState::Ready, ConnectedDeviceState::Ready),
        (DeviceState::Temporary, ConnectedDeviceState::Temporary),
    ];
    for (device_state, expected_state) in connected_device_states_by_device_state {
        assert_eq!(ConnectedDeviceState::from(device_state), expected_state);
    }
}

#[test]
fn connected_device_serializes_with_the_field_names_that_typescript_reads() {
    let connected_device = ConnectedDevice::from_version_information(
        PORT_NAME.to_owned(),
        xbt_version_information(DeviceState::Temporary),
    );
    assert_eq!(
        serde_json::to_value(connected_device).expect("a connected device should serialize"),
        json!({
            "portName": PORT_NAME,
            "firmwareVersion": FIRMWARE_VERSION,
            "boardType": BOARD_TYPE,
            "deviceState": "temporary",
        })
    );
}

#[test]
fn connected_device_states_serialize_as_their_names() {
    let names_by_connected_device_state = [
        (ConnectedDeviceState::Uninitialized, "uninitialized"),
        (ConnectedDeviceState::Unsaved, "unsaved"),
        (ConnectedDeviceState::Locked, "locked"),
        (ConnectedDeviceState::Ready, "ready"),
        (ConnectedDeviceState::Temporary, "temporary"),
    ];
    for (connected_device_state, expected_name) in names_by_connected_device_state {
        assert_eq!(
            serde_json::to_value(connected_device_state)
                .expect("a connected device state should serialize"),
            json!(expected_name)
        );
    }
}

#[test]
fn listed_port_is_accepted() {
    let search_attempt = Ok(vec![
        cp210x_diy_board_port(OTHER_PORT_NAME),
        cp210x_diy_board_port(PORT_NAME),
    ]);
    assert_eq!(require_listed_port(PORT_NAME, search_attempt), Ok(()));
}

#[test]
fn unlisted_port_is_rejected_as_not_listed() {
    let search_attempt = Ok(vec![cp210x_diy_board_port(OTHER_PORT_NAME)]);
    assert_eq!(
        require_listed_port(PORT_NAME, search_attempt),
        Err(DeviceConnectionError::PortNotListed)
    );
}

#[test]
fn port_is_rejected_as_enumeration_failed_when_the_search_fails() {
    let search_attempt = Err(io::Error::other(
        "the system could not list its serial ports",
    ));
    assert_eq!(
        require_listed_port(PORT_NAME, search_attempt),
        Err(DeviceConnectionError::PortEnumerationFailed)
    );
}

#[test]
fn port_opening_failures_become_their_error_codes() {
    let codes_by_opening_failure = [
        (
            connection_failure(io::ErrorKind::NotFound),
            DeviceConnectionError::PortUnavailable,
        ),
        (
            connection_failure(io::ErrorKind::PermissionDenied),
            DeviceConnectionError::PortAccessDenied,
        ),
        (
            connection_failure(io::ErrorKind::InvalidInput),
            DeviceConnectionError::PortOpenFailed,
        ),
        (
            DeviceError::ConnectionClosed,
            DeviceConnectionError::PortOpenFailed,
        ),
    ];
    for (opening_failure, expected_code) in codes_by_opening_failure {
        assert_eq!(
            connection_error_from_port_opening(&opening_failure),
            expected_code,
            "{opening_failure}"
        );
    }
}

#[test]
fn version_request_failures_become_their_error_codes() {
    let codes_by_request_failure = [
        (
            connection_failure(io::ErrorKind::BrokenPipe),
            DeviceConnectionError::ConnectionLost,
        ),
        (
            DeviceError::ConnectionClosed,
            DeviceConnectionError::ConnectionLost,
        ),
        (
            DeviceError::ReplyTimedOut,
            DeviceConnectionError::DeviceNotAnswering,
        ),
        (
            DeviceError::NotXbtFirmware {
                chain: Some("BTC".to_owned()),
            },
            DeviceConnectionError::NotXbtFirmware,
        ),
        (
            DeviceError::NotXbtFirmware { chain: None },
            DeviceConnectionError::NotXbtFirmware,
        ),
        (
            DeviceError::RequestEncodingFailed("the request is too large".to_owned()),
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::MalformedMessage("the reply is not CBOR".to_owned()),
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::UnexpectedReplyIdentifier {
                expected: "1".to_owned(),
                received: "7".to_owned(),
            },
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::ReplyWithoutResult,
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::Rejected(ErrorReply {
                code: -32601,
                description: "unknown method".to_owned(),
            }),
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::RequestNotConfirmed {
                method_name: "get_version_info".to_owned(),
            },
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::PinServerFailed(PinServerError::UnexpectedHttpStatus(500)),
            DeviceConnectionError::DeviceNotRecognized,
        ),
        (
            DeviceError::UnsupportedPinServerRequest("an unknown request".to_owned()),
            DeviceConnectionError::DeviceNotRecognized,
        ),
    ];
    for (request_failure, expected_code) in codes_by_request_failure {
        assert_eq!(
            connection_error_from_version_request(&request_failure),
            expected_code,
            "{request_failure}"
        );
    }
}

#[test]
fn connection_errors_serialize_as_their_codes() {
    let codes_by_connection_error = [
        (
            DeviceConnectionError::PortEnumerationFailed,
            "port-enumeration-failed",
        ),
        (DeviceConnectionError::PortNotListed, "port-not-listed"),
        (DeviceConnectionError::PortUnavailable, "port-unavailable"),
        (
            DeviceConnectionError::PortAccessDenied,
            "port-access-denied",
        ),
        (DeviceConnectionError::PortOpenFailed, "port-open-failed"),
        (
            DeviceConnectionError::DeviceNotAnswering,
            "device-not-answering",
        ),
        (
            DeviceConnectionError::DeviceNotRecognized,
            "device-not-recognized",
        ),
        (DeviceConnectionError::NotXbtFirmware, "not-xbt-firmware"),
        (DeviceConnectionError::ConnectionLost, "connection-lost"),
        (
            DeviceConnectionError::CommandInterrupted,
            "command-interrupted",
        ),
    ];
    for (connection_error, expected_code) in codes_by_connection_error {
        assert_eq!(
            serde_json::to_value(connection_error).expect("a connection error should serialize"),
            json!(expected_code)
        );
    }
}
