use serde_json::json;

use super::*;

const PIN_SERVER_URL: &str = "https://pin.example.test";
const PIN_SERVER_ONION_URL: &str = "http://pinserverexample.onion";
const PUBLIC_KEY_LENGTH: usize = 33;
const COMPRESSED_PUBLIC_KEY_PREFIX: u8 = 0x02;
const UNCOMPRESSED_PUBLIC_KEY_PREFIX: u8 = 0x04;
const PUBLIC_KEY_BODY_BYTE: u8 = 0xab;
const CERTIFICATE_LINES: [&str; 3] = [
    "-----BEGIN CERTIFICATE-----",
    "TUlJQmNlcnRpZmljYXRl",
    "-----END CERTIFICATE-----",
];

fn hex_public_key(public_key_prefix: u8, public_key_length: usize) -> String {
    let public_key_body_hex = format!("{PUBLIC_KEY_BODY_BYTE:02x}").repeat(public_key_length - 1);
    format!("{public_key_prefix:02x}{public_key_body_hex}")
}

fn normalized_test_certificate() -> String {
    format!("{}\n", CERTIFICATE_LINES.join("\n"))
}

fn valid_custom_pin_server_details() -> CustomPinServerDetails {
    CustomPinServerDetails {
        url: PIN_SERVER_URL.to_owned(),
        onion_url: Some(PIN_SERVER_ONION_URL.to_owned()),
        public_key_hex: hex_public_key(COMPRESSED_PUBLIC_KEY_PREFIX, PUBLIC_KEY_LENGTH),
        certificate: None,
    }
}

#[test]
fn hex_text_becomes_its_bytes_in_either_letter_case() {
    assert_eq!(
        public_key_bytes_from_hex("00ff10Ab"),
        Some(vec![0x00, 0xff, 0x10, 0xab])
    );
}

#[test]
fn hex_text_is_read_without_the_blank_space_around_it() {
    assert_eq!(public_key_bytes_from_hex(" 00ff\n"), Some(vec![0x00, 0xff]));
}

#[test]
fn text_that_is_not_whole_hex_bytes_is_refused() {
    for text_that_is_not_whole_hex_bytes in ["00f", "00fg", "+f", "00 ff"] {
        assert_eq!(
            public_key_bytes_from_hex(text_that_is_not_whole_hex_bytes),
            None,
            "{text_that_is_not_whole_hex_bytes}"
        );
    }
}

#[test]
fn certificate_gets_lf_line_endings_and_a_single_final_line_break() {
    for pasted_certificate in [
        CERTIFICATE_LINES.join("\r\n"),
        CERTIFICATE_LINES.join("\r"),
        format!("{}\n\n\n", CERTIFICATE_LINES.join("\n")),
        format!("\n  {}  \n", CERTIFICATE_LINES.join("\n")),
    ] {
        assert_eq!(
            normalized_certificate(&pasted_certificate),
            normalized_test_certificate(),
            "{pasted_certificate:?}"
        );
    }
}

#[test]
fn custom_pin_server_details_become_the_custom_pin_server_to_send() {
    let details_with_a_pasted_certificate = CustomPinServerDetails {
        certificate: Some(CERTIFICATE_LINES.join("\r\n")),
        ..valid_custom_pin_server_details()
    };
    let mut expected_public_key = [PUBLIC_KEY_BODY_BYTE; PUBLIC_KEY_LENGTH];
    expected_public_key[0] = COMPRESSED_PUBLIC_KEY_PREFIX;
    assert_eq!(
        custom_pin_server_from_details(details_with_a_pasted_certificate),
        Ok(CustomPinServer {
            url: PIN_SERVER_URL.to_owned(),
            onion_url: Some(PIN_SERVER_ONION_URL.to_owned()),
            public_key: expected_public_key,
            certificate: Some(normalized_test_certificate()),
        })
    );
}

#[test]
fn details_with_every_field_right_pass_the_check() {
    let converted_custom_pin_server =
        custom_pin_server_from_details(valid_custom_pin_server_details())
            .expect("valid details should convert");
    assert_eq!(
        checked_custom_pin_server(valid_custom_pin_server_details()),
        Ok(converted_custom_pin_server)
    );
}

#[test]
fn details_with_a_wrong_field_become_the_error_of_that_field() {
    let one_byte_short_public_key_hex =
        hex_public_key(COMPRESSED_PUBLIC_KEY_PREFIX, PUBLIC_KEY_LENGTH - 1);
    let choice_errors_by_wrong_details = [
        (
            CustomPinServerDetails {
                url: format!("{PIN_SERVER_URL}/"),
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidUrl,
        ),
        (
            CustomPinServerDetails {
                onion_url: Some(PIN_SERVER_URL.to_owned()),
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidOnionUrl,
        ),
        (
            CustomPinServerDetails {
                public_key_hex: "not hex".to_owned(),
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidPublicKey,
        ),
        (
            CustomPinServerDetails {
                public_key_hex: one_byte_short_public_key_hex,
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidPublicKey,
        ),
        (
            CustomPinServerDetails {
                public_key_hex: hex_public_key(UNCOMPRESSED_PUBLIC_KEY_PREFIX, PUBLIC_KEY_LENGTH),
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidPublicKey,
        ),
        (
            CustomPinServerDetails {
                certificate: Some("not a certificate".to_owned()),
                ..valid_custom_pin_server_details()
            },
            PinServerChoiceError::InvalidCertificate,
        ),
    ];
    for (wrong_details, expected_choice_error) in choice_errors_by_wrong_details {
        let wrong_details_description = format!("{wrong_details:?}");
        assert_eq!(
            checked_custom_pin_server(wrong_details),
            Err(expected_choice_error),
            "{wrong_details_description}"
        );
    }
}

#[test]
fn custom_pin_server_details_deserialize_from_the_field_names_that_typescript_sends() {
    let public_key_hex = hex_public_key(COMPRESSED_PUBLIC_KEY_PREFIX, PUBLIC_KEY_LENGTH);
    let sent_details = json!({
        "url": PIN_SERVER_URL,
        "onionUrl": null,
        "publicKeyHex": public_key_hex,
        "certificate": null,
    });
    assert_eq!(
        serde_json::from_value::<CustomPinServerDetails>(sent_details)
            .expect("the sent details should deserialize"),
        CustomPinServerDetails {
            url: PIN_SERVER_URL.to_owned(),
            onion_url: None,
            public_key_hex,
            certificate: None,
        }
    );
}
