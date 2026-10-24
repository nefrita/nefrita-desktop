use rcgen::{BasicConstraints, IsCa, KeyPair};

use super::*;
use crate::pin_server::test_certificates::{
    certificate_authority, certificate_signed_by, self_signed_certificate,
};
use crate::pin_server::test_server::{
    CapturedHttpRequest, ScriptedHttpResponse, TEST_SERVER_ADDRESS, start_http_server,
    start_https_server,
};

const DEVICE_MESSAGE: &str = "device message";
const SERVER_REPLY: &str = "server reply";
const OWN_AUTHORITY_NAME: &str = "Nefrita test authority";
const OTHER_AUTHORITY_NAME: &str = "Other test authority";

const SERVER_REPLY_RESPONSE: ScriptedHttpResponse = ScriptedHttpResponse {
    status_line: "HTTP/1.1 200 OK",
    extra_headers: "",
    response_body: r#"{"data":"server reply"}"#,
};

fn pin_server_request(pin_server_url: String, root_certificates: Vec<String>) -> PinServerRequest {
    PinServerRequest {
        urls: vec![pin_server_url],
        root_certificates,
        device_payload: PinServerPayload {
            encrypted_message: DEVICE_MESSAGE.to_owned(),
        },
    }
}

fn server_reply_payload() -> PinServerPayload {
    PinServerPayload {
        encrypted_message: SERVER_REPLY.to_owned(),
    }
}

fn exchange_over_http(
    scripted_response: ScriptedHttpResponse,
) -> (
    Result<PinServerPayload, PinServerError>,
    CapturedHttpRequest,
) {
    let (server_port, server_thread) = start_http_server(scripted_response);
    let exchange_attempt = HttpPinServer.exchange(&pin_server_request(
        format!("http://{TEST_SERVER_ADDRESS}:{server_port}/get_pin"),
        Vec::new(),
    ));
    let captured_request = server_thread
        .join()
        .expect("the test server thread should not panic")
        .expect("the test server should capture the request");
    (exchange_attempt, captured_request)
}

fn exchange_over_https(
    server_certificate: &rcgen::Certificate,
    server_key: &KeyPair,
    root_certificates_pem: Vec<String>,
) -> Result<PinServerPayload, PinServerError> {
    let (server_port, server_thread) =
        start_https_server(server_certificate, server_key, SERVER_REPLY_RESPONSE);
    let exchange_attempt = HttpPinServer.exchange(&pin_server_request(
        format!("https://{TEST_SERVER_ADDRESS}:{server_port}/get_pin"),
        root_certificates_pem,
    ));
    server_thread
        .join()
        .expect("the test server thread should not panic");
    exchange_attempt
}

fn assert_request_failed(exchange_attempt: &Result<PinServerPayload, PinServerError>) {
    assert!(
        matches!(exchange_attempt, Err(PinServerError::RequestFailed(_))),
        "expected a failed request, got {exchange_attempt:?}"
    );
}

#[test]
fn payload_is_posted_as_json_and_the_reply_is_returned() {
    let (exchange_attempt, captured_request) = exchange_over_http(SERVER_REPLY_RESPONSE);
    assert_eq!(exchange_attempt, Ok(server_reply_payload()));
    let lowercase_request_head = captured_request.request_head.to_ascii_lowercase();
    assert!(lowercase_request_head.starts_with("post /get_pin http/1.1\r\n"));
    assert!(lowercase_request_head.contains("content-type: application/json"));
    assert!(!lowercase_request_head.contains("user-agent"));
    let posted_payload: serde_json::Value = serde_json::from_slice(&captured_request.request_body)
        .expect("the posted body should be JSON");
    assert_eq!(posted_payload, serde_json::json!({"data": DEVICE_MESSAGE}));
}

#[test]
fn http_status_other_than_ok_is_reported() {
    let (exchange_attempt, _) = exchange_over_http(ScriptedHttpResponse {
        status_line: "HTTP/1.1 500 Internal Server Error",
        extra_headers: "",
        response_body: "{}",
    });
    assert_eq!(
        exchange_attempt,
        Err(PinServerError::UnexpectedHttpStatus(500))
    );
}

#[test]
fn redirect_is_not_followed() {
    let (exchange_attempt, _) = exchange_over_http(ScriptedHttpResponse {
        status_line: "HTTP/1.1 307 Temporary Redirect",
        extra_headers: "Location: http://127.0.0.1:9/get_pin\r\n",
        response_body: "{}",
    });
    assert_eq!(
        exchange_attempt,
        Err(PinServerError::UnexpectedHttpStatus(307))
    );
}

#[test]
fn malformed_reply_is_reported() {
    let (exchange_attempt, _) = exchange_over_http(ScriptedHttpResponse {
        status_line: "HTTP/1.1 200 OK",
        extra_headers: "",
        response_body: "not json",
    });
    assert!(matches!(
        exchange_attempt,
        Err(PinServerError::MalformedReply(_))
    ));
}

#[test]
fn own_certificate_authority_is_trusted_when_the_device_sends_it() {
    let own_authority = certificate_authority(OWN_AUTHORITY_NAME);
    let (server_certificate, server_key) =
        certificate_signed_by(&own_authority, TEST_SERVER_ADDRESS);
    let exchange_attempt = exchange_over_https(
        &server_certificate,
        &server_key,
        vec![own_authority.certificate.pem()],
    );
    assert_eq!(exchange_attempt, Ok(server_reply_payload()));
}

#[test]
fn system_roots_do_not_trust_an_own_certificate_authority() {
    let own_authority = certificate_authority(OWN_AUTHORITY_NAME);
    let (server_certificate, server_key) =
        certificate_signed_by(&own_authority, TEST_SERVER_ADDRESS);
    let exchange_attempt = exchange_over_https(&server_certificate, &server_key, Vec::new());
    assert_request_failed(&exchange_attempt);
}

#[test]
fn server_signed_by_another_authority_is_refused() {
    let own_authority = certificate_authority(OWN_AUTHORITY_NAME);
    let other_authority = certificate_authority(OTHER_AUTHORITY_NAME);
    let (server_certificate, server_key) =
        certificate_signed_by(&own_authority, TEST_SERVER_ADDRESS);
    let exchange_attempt = exchange_over_https(
        &server_certificate,
        &server_key,
        vec![other_authority.certificate.pem()],
    );
    assert_request_failed(&exchange_attempt);
}

#[test]
fn certificate_for_another_name_is_refused() {
    let own_authority = certificate_authority(OWN_AUTHORITY_NAME);
    let (server_certificate, server_key) = certificate_signed_by(&own_authority, "pin-server.test");
    let exchange_attempt = exchange_over_https(
        &server_certificate,
        &server_key,
        vec![own_authority.certificate.pem()],
    );
    assert_request_failed(&exchange_attempt);
}

#[test]
fn self_signed_certificate_without_the_authority_flag_is_trusted() {
    let (server_certificate, server_key) =
        self_signed_certificate(TEST_SERVER_ADDRESS, IsCa::ExplicitNoCa);
    let exchange_attempt = exchange_over_https(
        &server_certificate,
        &server_key,
        vec![server_certificate.pem()],
    );
    assert_eq!(exchange_attempt, Ok(server_reply_payload()));
}

#[test]
fn self_signed_certificate_marked_as_authority_is_refused() {
    let (server_certificate, server_key) = self_signed_certificate(
        TEST_SERVER_ADDRESS,
        IsCa::Ca(BasicConstraints::Unconstrained),
    );
    let exchange_attempt = exchange_over_https(
        &server_certificate,
        &server_key,
        vec![server_certificate.pem()],
    );
    let Err(PinServerError::RequestFailed(request_error_description)) = exchange_attempt else {
        panic!("expected a failed request, got {exchange_attempt:?}");
    };
    assert!(
        request_error_description.contains("CaUsedAsEndEntity"),
        "unexpected error: {request_error_description}"
    );
}
