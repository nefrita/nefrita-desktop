use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, IgnoredAny, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::json;

use super::*;
use crate::device::scripted::{confirmation_reply, device_replying_with};

const CUSTOM_PIN_SERVER_URL: &str = "https://pin.example.test";
const CUSTOM_PIN_SERVER_ONION_URL: &str = "http://pinserverexample.onion";
const CUSTOM_PIN_SERVER_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----";
const CUSTOM_PIN_SERVER_PUBLIC_KEY: [u8; PIN_SERVER_PUBLIC_KEY_LENGTH] =
    [2; PIN_SERVER_PUBLIC_KEY_LENGTH];

#[derive(Deserialize)]
struct WrittenPinServerUpdate {
    #[serde(rename = "params")]
    custom_pin_server: WrittenCustomPinServer,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct WrittenCustomPinServer {
    #[serde(rename = "urlA")]
    url: String,
    #[serde(rename = "urlB")]
    onion_url: Option<String>,
    #[serde(rename = "pubkey", deserialize_with = "deserialize_byte_string")]
    public_key: Vec<u8>,
    certificate: Option<String>,
}

#[derive(Deserialize)]
struct WrittenParameterNames {
    #[serde(rename = "params")]
    parameters: BTreeMap<String, IgnoredAny>,
}

fn deserialize_byte_string<'de, FieldDeserializer: Deserializer<'de>>(
    deserializer: FieldDeserializer,
) -> Result<Vec<u8>, FieldDeserializer::Error> {
    struct ByteStringVisitor;

    impl Visitor<'_> for ByteStringVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a CBOR byte string")
        }

        fn visit_bytes<VisitorError: de::Error>(
            self,
            byte_string: &[u8],
        ) -> Result<Vec<u8>, VisitorError> {
            Ok(byte_string.to_vec())
        }
    }

    deserializer.deserialize_bytes(ByteStringVisitor)
}

fn custom_pin_server_with_onion_and_certificate() -> CustomPinServer {
    CustomPinServer {
        url: CUSTOM_PIN_SERVER_URL.to_owned(),
        onion_url: Some(CUSTOM_PIN_SERVER_ONION_URL.to_owned()),
        public_key: CUSTOM_PIN_SERVER_PUBLIC_KEY,
        certificate: Some(CUSTOM_PIN_SERVER_CERTIFICATE.to_owned()),
    }
}

#[test]
fn custom_pin_server_is_sent_with_the_public_key_as_bytes() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .set_custom_pin_server(&custom_pin_server_with_onion_and_certificate())
        .expect("the update should be confirmed");
    let written_update: WrittenPinServerUpdate =
        minicbor_serde::from_slice(&device.connection().encoded_written_messages()[0])
            .expect("the update should decode with the public key as bytes");
    assert_eq!(
        written_update.custom_pin_server,
        WrittenCustomPinServer {
            url: CUSTOM_PIN_SERVER_URL.to_owned(),
            onion_url: Some(CUSTOM_PIN_SERVER_ONION_URL.to_owned()),
            public_key: CUSTOM_PIN_SERVER_PUBLIC_KEY.to_vec(),
            certificate: Some(CUSTOM_PIN_SERVER_CERTIFICATE.to_owned()),
        }
    );
}

#[test]
fn custom_pin_server_without_onion_or_certificate_leaves_them_out() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    let custom_pin_server = CustomPinServer {
        onion_url: None,
        certificate: None,
        ..custom_pin_server_with_onion_and_certificate()
    };
    device
        .set_custom_pin_server(&custom_pin_server)
        .expect("the update should be confirmed");
    let written_parameter_names: WrittenParameterNames =
        minicbor_serde::from_slice(&device.connection().encoded_written_messages()[0])
            .expect("the update should decode");
    let parameter_names: Vec<&str> = written_parameter_names
        .parameters
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(parameter_names, ["pubkey", "urlA"]);
}

#[test]
fn pin_server_reset_asks_to_reset_details_and_certificate() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .reset_pin_server()
        .expect("the reset should be confirmed");
    assert_eq!(
        device.connection().written_messages(),
        vec![json!({
            "id": "1",
            "method": "update_pinserver",
            "params": {"reset_details": true, "reset_certificate": true},
        })]
    );
}

#[test]
fn unconfirmed_pin_server_update_is_reported() {
    let mut device = device_replying_with(vec![confirmation_reply("1", false)]);
    let update_attempt =
        device.set_custom_pin_server(&custom_pin_server_with_onion_and_certificate());
    let Err(DeviceError::RequestNotConfirmed { method_name }) = update_attempt else {
        panic!("expected an unconfirmed request, got {update_attempt:?}");
    };
    assert_eq!(method_name, "update_pinserver");
}
