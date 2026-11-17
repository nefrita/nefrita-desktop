use std::collections::BTreeMap;
use std::fmt;

use rcgen::{KeyPair, generate_simple_self_signed};
use serde::de::{self, IgnoredAny, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::json;

use super::*;
use crate::connection::scripted::SCRIPTED_REPLY_TIMEOUT;
use crate::device::scripted::{confirmation_reply, device_replying_with};

const CUSTOM_PIN_SERVER_HOST_NAME: &str = "pin.example.test";
const CUSTOM_PIN_SERVER_URL: &str = "https://pin.example.test";
const CUSTOM_PIN_SERVER_ONION_URL: &str = "http://pinserverexample.onion";
const CUSTOM_PIN_SERVER_PUBLIC_KEY: [u8; PIN_SERVER_PUBLIC_KEY_LENGTH] =
    [2; PIN_SERVER_PUBLIC_KEY_LENGTH];
const UNCOMPRESSED_PUBLIC_KEY_PREFIX: u8 = 0x04;

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

fn self_signed_certificate_pem() -> String {
    generate_simple_self_signed(vec![CUSTOM_PIN_SERVER_HOST_NAME.to_owned()])
        .expect("a test certificate should generate")
        .cert
        .pem()
}

fn custom_pin_server_without_onion_or_certificate() -> CustomPinServer {
    CustomPinServer {
        url: CUSTOM_PIN_SERVER_URL.to_owned(),
        onion_url: None,
        public_key: CUSTOM_PIN_SERVER_PUBLIC_KEY,
        certificate: None,
    }
}

fn custom_pin_server_with_onion_and_certificate(certificate_pem: &str) -> CustomPinServer {
    CustomPinServer {
        onion_url: Some(CUSTOM_PIN_SERVER_ONION_URL.to_owned()),
        certificate: Some(certificate_pem.to_owned()),
        ..custom_pin_server_without_onion_or_certificate()
    }
}

fn url_with_byte_length(url_byte_length: usize) -> String {
    let path_byte_length = url_byte_length - CUSTOM_PIN_SERVER_URL.len() - 1;
    format!("{CUSTOM_PIN_SERVER_URL}/{}", "p".repeat(path_byte_length))
}

#[test]
fn custom_pin_server_is_sent_with_the_public_key_as_bytes() {
    let certificate_pem = self_signed_certificate_pem();
    let custom_pin_server = custom_pin_server_with_onion_and_certificate(&certificate_pem);
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .set_custom_pin_server(&custom_pin_server)
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
            certificate: Some(certificate_pem),
        }
    );
}

#[test]
fn custom_pin_server_without_onion_or_certificate_leaves_them_out() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .set_custom_pin_server(&custom_pin_server_without_onion_or_certificate())
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
        device.set_custom_pin_server(&custom_pin_server_without_onion_or_certificate());
    let Err(DeviceError::RequestNotConfirmed { method_name }) = update_attempt else {
        panic!("expected an unconfirmed request, got {update_attempt:?}");
    };
    assert_eq!(method_name, "update_pinserver");
}

#[test]
fn reply_timeout_is_removed_during_the_pin_server_update_and_restored() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .set_custom_pin_server(&custom_pin_server_without_onion_or_certificate())
        .expect("the update should be confirmed");
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}

#[test]
fn reply_timeout_is_removed_during_the_pin_server_reset_and_restored() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .reset_pin_server()
        .expect("the reset should be confirmed");
    assert_eq!(
        device.connection().reply_timeout_changes(),
        [None, Some(SCRIPTED_REPLY_TIMEOUT)]
    );
}

#[test]
fn custom_pin_server_that_fails_the_check_is_not_sent() {
    let mut device = device_replying_with(Vec::new());
    let custom_pin_server = CustomPinServer {
        url: format!("{CUSTOM_PIN_SERVER_URL}/"),
        ..custom_pin_server_without_onion_or_certificate()
    };
    let update_attempt = device.set_custom_pin_server(&custom_pin_server);
    let Err(DeviceError::InvalidCustomPinServer(check_error)) = update_attempt else {
        panic!("expected a failed check, got {update_attempt:?}");
    };
    assert_eq!(
        check_error,
        CustomPinServerCheckError::InvalidUrl("it ends with /".to_owned())
    );
    assert!(device.connection().written_messages().is_empty());
    assert!(device.connection().reply_timeout_changes().is_empty());
}

#[test]
fn custom_pin_server_with_onion_and_certificate_passes_the_check() {
    let custom_pin_server =
        custom_pin_server_with_onion_and_certificate(&self_signed_certificate_pem());
    assert_eq!(custom_pin_server.check(), Ok(()));
}

#[test]
fn url_without_http_or_https_fails_the_check() {
    for url_without_supported_scheme in [
        "pin.example.test",
        "ftp://pin.example.test",
        "HTTPS://pin.example.test",
    ] {
        let custom_pin_server = CustomPinServer {
            url: url_without_supported_scheme.to_owned(),
            ..custom_pin_server_without_onion_or_certificate()
        };
        assert_eq!(
            custom_pin_server.check(),
            Err(CustomPinServerCheckError::InvalidUrl(
                "it does not start with http:// or https://".to_owned()
            )),
            "{url_without_supported_scheme}"
        );
    }
}

#[test]
fn url_ending_with_a_slash_fails_the_check() {
    let custom_pin_server = CustomPinServer {
        url: format!("{CUSTOM_PIN_SERVER_URL}/"),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidUrl(
            "it ends with /".to_owned()
        ))
    );
}

#[test]
fn url_that_does_not_parse_fails_the_check() {
    let custom_pin_server = CustomPinServer {
        url: "https://pin example.test".to_owned(),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidUrl(
            "it is not a valid URL".to_owned()
        ))
    );
}

#[test]
fn url_of_120_bytes_fails_the_check_and_one_of_119_passes() {
    let custom_pin_server_with_the_longest_accepted_url = CustomPinServer {
        url: url_with_byte_length(FIRMWARE_URL_BUFFER_LENGTH - 1),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server_with_the_longest_accepted_url.check(),
        Ok(())
    );
    let custom_pin_server_with_the_shortest_refused_url = CustomPinServer {
        url: url_with_byte_length(FIRMWARE_URL_BUFFER_LENGTH),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server_with_the_shortest_refused_url.check(),
        Err(CustomPinServerCheckError::InvalidUrl(
            "it has 120 bytes or more".to_owned()
        ))
    );
}

#[test]
fn onion_url_with_a_host_that_is_not_onion_fails_the_check() {
    let custom_pin_server = CustomPinServer {
        onion_url: Some(CUSTOM_PIN_SERVER_URL.to_owned()),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidOnionUrl(
            "its host is not an onion address".to_owned()
        ))
    );
}

#[test]
fn onion_url_ending_with_a_slash_fails_the_check() {
    let custom_pin_server = CustomPinServer {
        onion_url: Some(format!("{CUSTOM_PIN_SERVER_ONION_URL}/")),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidOnionUrl(
            "it ends with /".to_owned()
        ))
    );
}

#[test]
fn public_key_that_is_not_compressed_fails_the_check() {
    let mut public_key_without_compressed_prefix = CUSTOM_PIN_SERVER_PUBLIC_KEY;
    public_key_without_compressed_prefix[0] = UNCOMPRESSED_PUBLIC_KEY_PREFIX;
    let custom_pin_server = CustomPinServer {
        public_key: public_key_without_compressed_prefix,
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidPublicKey)
    );
}

#[test]
fn certificate_that_is_not_pem_fails_the_check() {
    let custom_pin_server = CustomPinServer {
        certificate: Some("not a certificate".to_owned()),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidCertificate(
            "it is not PEM text holding only certificates".to_owned()
        ))
    );
}

#[test]
fn pem_text_holding_a_private_key_fails_the_check() {
    let private_key_pem = KeyPair::generate()
        .expect("a test key should generate")
        .serialize_pem();
    let custom_pin_server = CustomPinServer {
        certificate: Some(private_key_pem),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidCertificate(
            "it is not PEM text holding only certificates".to_owned()
        ))
    );
}

#[test]
fn certificate_of_2048_bytes_or_more_fails_the_check() {
    let certificate_pem = self_signed_certificate_pem();
    let certificate_copy_count = FIRMWARE_CERTIFICATE_BUFFER_LENGTH.div_ceil(certificate_pem.len());
    let custom_pin_server = CustomPinServer {
        certificate: Some(certificate_pem.repeat(certificate_copy_count)),
        ..custom_pin_server_without_onion_or_certificate()
    };
    assert_eq!(
        custom_pin_server.check(),
        Err(CustomPinServerCheckError::InvalidCertificate(
            "it has 2048 bytes or more".to_owned()
        ))
    );
}
