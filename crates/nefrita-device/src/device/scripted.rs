use serde_json::{Value, json};

use crate::connection::scripted::{ScriptedConnection, ScriptedRead, encode_cbor_message};
use crate::device::Device;

pub(super) const DEVELOPMENT_SET_PIN_URL: &str = "http://localhost:8096/set_pin";
pub(super) const ENCRYPTED_DEVICE_MESSAGE: &str = "device message";

pub(crate) fn version_information_reply(request_identifier: &str, chain: Option<&str>) -> Vec<u8> {
    let mut version_information = json!({
        "JADE_VERSION": "1.0.41",
        "BOARD_TYPE": "M5_STICKC_PLUS_2",
        "JADE_STATE": "LOCKED",
        "JADE_NETWORKS": "ALL",
        "JADE_HAS_PIN": true,
    });
    if let Some(chain) = chain {
        version_information["JADE_CHAIN"] = json!(chain);
    }
    encode_cbor_message(&json!({"id": request_identifier, "result": version_information}))
}

pub(super) fn confirmation_reply(request_identifier: &str, device_confirmed: bool) -> Vec<u8> {
    encode_cbor_message(&json!({"id": request_identifier, "result": device_confirmed}))
}

pub(super) fn rejection_reply(request_identifier: &str, error_code: i64) -> Vec<u8> {
    encode_cbor_message(&json!({
        "id": request_identifier,
        "error": {"code": error_code, "message": "rejected by the scripted device"},
    }))
}

pub(super) fn pin_server_request_reply_message(request_identifier: &str) -> Value {
    json!({
        "id": request_identifier,
        "result": {"http_request": {
            "params": {
                "urls": [DEVELOPMENT_SET_PIN_URL],
                "method": "POST",
                "accept": "json",
                "data": {"data": ENCRYPTED_DEVICE_MESSAGE},
            },
            "on-reply": "pin",
        }},
    })
}

pub(super) fn pin_server_request_reply(request_identifier: &str) -> Vec<u8> {
    encode_cbor_message(&pin_server_request_reply_message(request_identifier))
}

pub(super) fn device_replying_with(encoded_replies: Vec<Vec<u8>>) -> Device<ScriptedConnection> {
    let scripted_reads = encoded_replies
        .into_iter()
        .map(ScriptedRead::Bytes)
        .collect();
    Device::new(ScriptedConnection::new(scripted_reads))
}
