use serde_json::json;

use super::*;
use crate::connection::scripted::encode_cbor_message;
use crate::device::scripted::{device_replying_with, version_information_reply};

#[test]
fn version_information_is_requested_and_decoded() {
    let mut device = device_replying_with(vec![version_information_reply("1", Some("XBT"))]);
    let version_information = device
        .request_version_information()
        .expect("the version information should be decoded");
    assert_eq!(
        version_information,
        VersionInformation {
            firmware_version: "1.0.41".to_owned(),
            board_type: "M5_STICKC_PLUS_2".to_owned(),
            device_state: DeviceState::Locked,
            chain: Some("XBT".to_owned()),
        }
    );
    assert_eq!(
        device.connection().written_messages(),
        vec![json!({"id": "1", "method": "get_version_info"})]
    );
}

#[test]
fn device_states_decode_from_their_protocol_names() {
    let protocol_names_and_states = [
        ("UNINIT", DeviceState::Uninitialized),
        ("UNSAVED", DeviceState::Unsaved),
        ("LOCKED", DeviceState::Locked),
        ("READY", DeviceState::Ready),
        ("TEMP", DeviceState::Temporary),
    ];
    for (protocol_name, device_state) in protocol_names_and_states {
        let encoded_state = encode_cbor_message(&json!(protocol_name));
        let decoded_state: DeviceState =
            minicbor_serde::from_slice(&encoded_state).expect("the state should decode");
        assert_eq!(decoded_state, device_state, "state {protocol_name}");
    }
}

#[test]
fn xbt_firmware_is_accepted() {
    let mut device = device_replying_with(vec![version_information_reply("1", Some("XBT"))]);
    let version_information = device
        .require_xbt_firmware()
        .expect("an XBT firmware should be accepted");
    assert_eq!(version_information.chain.as_deref(), Some("XBT"));
}

#[test]
fn firmware_without_a_chain_is_rejected() {
    let mut device = device_replying_with(vec![version_information_reply("1", None)]);
    let firmware_check = device.require_xbt_firmware();
    assert!(matches!(
        firmware_check,
        Err(DeviceError::NotXbtFirmware { chain: None })
    ));
}

#[test]
fn firmware_of_another_chain_is_rejected() {
    let mut device = device_replying_with(vec![version_information_reply("1", Some("BTC"))]);
    let firmware_check = device.require_xbt_firmware();
    let Err(DeviceError::NotXbtFirmware { chain }) = firmware_check else {
        panic!("expected a firmware that is not XBT, got {firmware_check:?}");
    };
    assert_eq!(chain.as_deref(), Some("BTC"));
}
