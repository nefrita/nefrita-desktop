use serde_json::json;

use super::*;
use crate::connection::scripted::{SCRIPTED_REPLY_TIMEOUT, encode_cbor_message};
use crate::device::scripted::{
    DEVELOPMENT_PIN_SERVER_URL, ENCRYPTED_DEVICE_MESSAGE, confirmation_reply, device_replying_with,
    pin_server_request_reply, pin_server_request_reply_message, rejection_reply,
    version_information_reply,
};
use crate::pin_server::PinServerError;
use crate::pin_server::scripted::ScriptedPinServer;
use crate::protocol::ErrorReplyKind;

const ENCRYPTED_SERVER_REPLY: &str = "server reply";
const INVALID_PARAMETERS_CODE: i64 = -32602;
const NETWORK_MISMATCH_CODE: i64 = -32003;
const ALLOWED_CLOCK_DIFFERENCE_SECONDS: u64 = 60;

fn server_reply_payload() -> PinServerPayload {
    PinServerPayload {
        encrypted_message: ENCRYPTED_SERVER_REPLY.to_owned(),
    }
}

fn unreachable_pin_server_error() -> PinServerError {
    PinServerError::RequestFailed("connection refused".to_owned())
}

fn test_clock_unix_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the test clock should be after 1970")
        .as_secs()
}

#[test]
fn unlock_forwards_the_pin_server_request_and_reports_unlocked() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        pin_server_request_reply("2"),
        confirmation_reply("3", true),
    ]);
    let mut pin_server = ScriptedPinServer::answering(vec![Ok(server_reply_payload())]);
    let unlock_outcome = device
        .unlock(Network::XbtRegtest, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    let written_messages = device.connection().written_messages();
    assert_eq!(
        written_messages[2],
        json!({"id": "3", "method": "pin", "params": {"data": ENCRYPTED_SERVER_REPLY}})
    );
    assert_eq!(
        pin_server.received_requests(),
        [PinServerRequest {
            urls: vec![DEVELOPMENT_PIN_SERVER_URL.to_owned()],
            root_certificates: Vec::new(),
            device_payload: PinServerPayload {
                encrypted_message: ENCRYPTED_DEVICE_MESSAGE.to_owned(),
            },
        }]
    );
}

#[test]
fn authentication_carries_the_network_and_the_current_time() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        confirmation_reply("2", true),
    ]);
    device
        .unlock(
            Network::XbtRegtest,
            &mut ScriptedPinServer::answering(Vec::new()),
        )
        .expect("the unlock should finish");
    let written_messages = device.connection().written_messages();
    assert_eq!(written_messages[0]["method"], json!("get_version_info"));
    assert_eq!(written_messages[1]["method"], json!("auth_user"));
    assert_eq!(
        written_messages[1]["params"]["network"],
        json!("xbt-regtest")
    );
    let sent_unix_time_seconds = written_messages[1]["params"]["epoch"]
        .as_u64()
        .expect("auth_user should carry the current time");
    assert!(
        test_clock_unix_time_seconds().abs_diff(sent_unix_time_seconds)
            <= ALLOWED_CLOCK_DIFFERENCE_SECONDS
    );
}

#[test]
fn refused_unlock_is_reported() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        pin_server_request_reply("2"),
        confirmation_reply("3", false),
    ]);
    let mut pin_server = ScriptedPinServer::answering(vec![Ok(server_reply_payload())]);
    let unlock_outcome = device
        .unlock(Network::XbtRegtest, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Refused);
}

#[test]
fn unlocked_device_does_not_use_the_pin_server() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        confirmation_reply("2", true),
    ]);
    let mut pin_server = ScriptedPinServer::answering(Vec::new());
    let unlock_outcome = device
        .unlock(Network::XbtRegtest, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    assert!(pin_server.received_requests().is_empty());
}

#[test]
fn firmware_that_is_not_xbt_is_rejected_before_authentication() {
    let mut device = device_replying_with(vec![version_information_reply("1", None)]);
    let unlock_attempt = device.unlock(
        Network::XbtRegtest,
        &mut ScriptedPinServer::answering(Vec::new()),
    );
    assert!(matches!(
        unlock_attempt,
        Err(DeviceError::NotXbtFirmware { chain: None })
    ));
    assert_eq!(device.connection().written_messages().len(), 1);
    assert!(device.connection().reply_timeout_changes().is_empty());
}

#[test]
fn pin_server_failure_is_reported_when_the_device_gives_up() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        pin_server_request_reply("2"),
        rejection_reply("3", INVALID_PARAMETERS_CODE),
    ]);
    let mut pin_server = ScriptedPinServer::answering(vec![Err(unreachable_pin_server_error())]);
    let unlock_attempt = device.unlock(Network::XbtRegtest, &mut pin_server);
    let Err(DeviceError::PinServerFailed(pin_server_error)) = unlock_attempt else {
        panic!("expected a failed PIN server exchange, got {unlock_attempt:?}");
    };
    assert_eq!(pin_server_error, unreachable_pin_server_error());
    assert_eq!(
        device.connection().written_messages()[2],
        json!({"id": "3", "method": "pin"})
    );
}

#[test]
fn pin_server_is_asked_again_when_the_device_retries() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        pin_server_request_reply("2"),
        pin_server_request_reply("3"),
        confirmation_reply("4", true),
    ]);
    let mut pin_server = ScriptedPinServer::answering(vec![
        Err(unreachable_pin_server_error()),
        Ok(server_reply_payload()),
    ]);
    let unlock_outcome = device
        .unlock(Network::XbtRegtest, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    let written_messages = device.connection().written_messages();
    assert_eq!(written_messages[2], json!({"id": "3", "method": "pin"}));
    assert_eq!(
        written_messages[3],
        json!({"id": "4", "method": "pin", "params": {"data": ENCRYPTED_SERVER_REPLY}})
    );
}

#[test]
fn unsupported_pin_server_requests_are_cancelled() {
    let unsupported_settings_and_descriptions = [
        (
            "/result/http_request/on-reply",
            "get_pin",
            "it expects the reply in get_pin",
        ),
        (
            "/result/http_request/params/method",
            "GET",
            "it uses the HTTP method GET",
        ),
        (
            "/result/http_request/params/accept",
            "text",
            "it accepts text",
        ),
    ];
    for (field_pointer, unsupported_setting, expected_description) in
        unsupported_settings_and_descriptions
    {
        let mut unsupported_reply_message = pin_server_request_reply_message("2");
        *unsupported_reply_message
            .pointer_mut(field_pointer)
            .expect("the scripted reply should have the field") = json!(unsupported_setting);
        let mut device = device_replying_with(vec![
            version_information_reply("1", Some("XBT")),
            encode_cbor_message(&unsupported_reply_message),
        ]);
        let mut pin_server = ScriptedPinServer::answering(Vec::new());
        let unlock_attempt = device.unlock(Network::XbtRegtest, &mut pin_server);
        let Err(DeviceError::UnsupportedPinServerRequest(request_description)) = unlock_attempt
        else {
            panic!("expected an unsupported request for {field_pointer}, got {unlock_attempt:?}");
        };
        assert_eq!(request_description, expected_description);
        assert_eq!(
            device.connection().written_messages()[2],
            json!({"id": "3", "method": "cancel"}),
            "field {field_pointer}"
        );
        assert!(
            pin_server.received_requests().is_empty(),
            "field {field_pointer}"
        );
    }
}

#[test]
fn rejected_authentication_is_reported_with_its_kind() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        rejection_reply("2", NETWORK_MISMATCH_CODE),
    ]);
    let unlock_attempt = device.unlock(Network::Xbt, &mut ScriptedPinServer::answering(Vec::new()));
    let Err(DeviceError::Rejected(rejection)) = unlock_attempt else {
        panic!("expected a rejection, got {unlock_attempt:?}");
    };
    assert_eq!(rejection.kind(), ErrorReplyKind::NetworkMismatch);
}

#[test]
fn reply_timeout_is_removed_during_the_unlock_and_restored() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        confirmation_reply("2", true),
    ]);
    device
        .unlock(
            Network::XbtRegtest,
            &mut ScriptedPinServer::answering(Vec::new()),
        )
        .expect("the unlock should finish");
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}

#[test]
fn reply_timeout_is_restored_after_a_failed_unlock() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        rejection_reply("2", NETWORK_MISMATCH_CODE),
    ]);
    let unlock_attempt = device.unlock(Network::Xbt, &mut ScriptedPinServer::answering(Vec::new()));
    assert!(unlock_attempt.is_err());
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}

#[test]
fn logout_is_confirmed_by_the_device() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device.logout().expect("the logout should be confirmed");
    assert_eq!(
        device.connection().written_messages(),
        vec![json!({"id": "1", "method": "logout"})]
    );
}

#[test]
fn unconfirmed_logout_is_reported() {
    let mut device = device_replying_with(vec![confirmation_reply("1", false)]);
    let logout_attempt = device.logout();
    let Err(DeviceError::RequestNotConfirmed { method_name }) = logout_attempt else {
        panic!("expected an unconfirmed request, got {logout_attempt:?}");
    };
    assert_eq!(method_name, "logout");
}
