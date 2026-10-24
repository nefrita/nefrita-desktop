use serde_json::json;

use crate::connection::scripted::{ScriptedConnection, ScriptedRead, encode_cbor_message};
use crate::device::Device;

pub(super) fn version_information_reply(request_identifier: &str, chain: Option<&str>) -> Vec<u8> {
    let mut method_result = json!({
        "JADE_VERSION": "1.0.41",
        "BOARD_TYPE": "M5_STICKC_PLUS_2",
        "JADE_STATE": "LOCKED",
        "JADE_NETWORKS": "ALL",
        "JADE_HAS_PIN": true,
    });
    if let Some(chain) = chain {
        method_result["JADE_CHAIN"] = json!(chain);
    }
    encode_cbor_message(&json!({"id": request_identifier, "result": method_result}))
}

pub(super) fn device_replying_with(encoded_replies: Vec<Vec<u8>>) -> Device<ScriptedConnection> {
    let scripted_reads = encoded_replies
        .into_iter()
        .map(ScriptedRead::Bytes)
        .collect();
    Device::new(ScriptedConnection::new(scripted_reads))
}
