use serde_json::{Value, json};

use crate::device::scripted::{device_replying_with, version_information_reply};

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
