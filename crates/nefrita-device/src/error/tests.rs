use std::io::ErrorKind;

use super::*;

#[test]
fn connection_failure_shows_its_cause_in_the_message_not_as_its_source() {
    let connection_failure =
        DeviceError::ConnectionFailed(io::Error::from(ErrorKind::ConnectionRefused));
    assert_eq!(
        connection_failure.to_string(),
        "the connection to the device failed: connection refused"
    );
    assert!(connection_failure.source().is_none());
}

#[test]
fn pin_server_failure_shows_its_cause_in_the_message_not_as_its_source() {
    let pin_server_failure = DeviceError::PinServerFailed(PinServerError::RequestFailed(
        "connection refused".to_owned(),
    ));
    assert_eq!(
        pin_server_failure.to_string(),
        "the PIN server exchange failed: the request to the PIN server failed: connection refused"
    );
    assert!(pin_server_failure.source().is_none());
}

#[test]
fn custom_pin_server_check_failure_shows_its_cause_in_the_message_not_as_its_source() {
    let custom_pin_server_check_failure =
        DeviceError::InvalidCustomPinServer(CustomPinServerCheckError::InvalidPublicKey);
    assert_eq!(
        custom_pin_server_check_failure.to_string(),
        "the custom PIN server was not sent to the device: the PIN server public key is not a compressed public key"
    );
    assert!(custom_pin_server_check_failure.source().is_none());
}
