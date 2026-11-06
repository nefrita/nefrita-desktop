use serde_json::{Value, json};

use crate::connection::scripted::{SCRIPTED_REPLY_TIMEOUT, ScriptedConnection, ScriptedRead};
use crate::device::Device;
use crate::device::scripted::{device_replying_with, version_information_reply};
use crate::error::DeviceError;

#[test]
fn each_request_uses_the_next_identifier() {
    let mut device = device_replying_with(vec![
        version_information_reply("1", Some("XBT")),
        version_information_reply("2", Some("XBT")),
    ]);
    device
        .request_version_information()
        .expect("the first request should be answered");
    device
        .request_version_information()
        .expect("the second request should be answered");
    let request_identifiers: Vec<Value> = device
        .connection()
        .written_messages()
        .into_iter()
        .map(|written_message| written_message["id"].clone())
        .collect();
    assert_eq!(request_identifiers, vec![json!("1"), json!("2")]);
}

#[test]
fn request_after_a_timed_out_request_reads_its_own_reply() {
    let first_request_reply = version_information_reply("1", None);
    let second_request_reply = version_information_reply("2", Some("XBT"));
    let scripted_connection = ScriptedConnection::new(vec![
        ScriptedRead::TimedOut,
        ScriptedRead::Bytes(first_request_reply),
        ScriptedRead::Bytes(second_request_reply),
    ]);
    let mut device = Device::new(scripted_connection);
    let first_request_attempt = device.request_version_information();
    assert!(matches!(
        first_request_attempt,
        Err(DeviceError::ReplyTimedOut)
    ));
    let version_information = device
        .request_version_information()
        .expect("the second request should be answered");
    assert_eq!(version_information.chain.as_deref(), Some("XBT"));
}

#[test]
fn reply_timeout_is_removed_while_the_device_is_used_and_restored() {
    let mut device = device_replying_with(vec![version_information_reply("1", Some("XBT"))]);
    device
        .run_without_reply_timeout(|device| device.request_version_information())
        .expect("the request should be answered");
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}

#[test]
fn reply_timeout_is_restored_after_a_failed_device_use() {
    let mut device = device_replying_with(Vec::new());
    let device_use_attempt =
        device.run_without_reply_timeout(|device| device.request_version_information());
    assert!(device_use_attempt.is_err());
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}
