use std::net::TcpListener;

use rcgen::{BasicConstraints, IsCa, KeyPair};

use super::*;
use crate::pin_server::test_certificates::{
    certificate_authority, certificate_signed_by, self_signed_certificate,
};
use crate::pin_server::test_server::{
    CapturedHttpRequest, ScriptedHttpResponse, TEST_SERVER_ADDRESS, closed_local_address,
    listen_without_blocking, start_http_server, start_https_server,
};

const ENCRYPTED_DEVICE_MESSAGE: &str = "device message";
const ENCRYPTED_SERVER_REPLY: &str = "server reply";
const OWN_AUTHORITY_NAME: &str = "Nefrita test authority";
const OTHER_AUTHORITY_NAME: &str = "Other test authority";
const ONION_PIN_SERVER_URL: &str = "http://pinserver.onion/get_pin";

const SERVER_REPLY_RESPONSE: ScriptedHttpResponse = ScriptedHttpResponse {
    status_line: "HTTP/1.1 200 OK",
    extra_headers: "",
    response_body: r#"{"data":"server reply"}"#,
};

fn pin_server_request(
    pin_server_urls: Vec<String>,
    root_certificates: Vec<String>,
) -> PinServerRequest {
    PinServerRequest {
        urls: pin_server_urls,
        root_certificates,
        device_payload: PinServerPayload {
            encrypted_message: ENCRYPTED_DEVICE_MESSAGE.to_owned(),
        },
    }
}

fn local_pin_server_url(url_scheme: &str, server_port: u16) -> String {
    format!("{url_scheme}://{TEST_SERVER_ADDRESS}:{server_port}/get_pin")
}

fn server_reply_payload() -> PinServerPayload {
    PinServerPayload {
        encrypted_message: ENCRYPTED_SERVER_REPLY.to_owned(),
    }
}

fn exchange_over_http(
    scripted_response: ScriptedHttpResponse,
) -> (
    Result<PinServerPayload, PinServerError>,
    CapturedHttpRequest,
) {
    let (server_port, server_thread) = start_http_server(scripted_response);
    let exchange_attempt = HttpPinServer::direct().exchange(&pin_server_request(
        vec![local_pin_server_url("http", server_port)],
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
    let exchange_attempt = HttpPinServer::direct().exchange(&pin_server_request(
        vec![local_pin_server_url("https", server_port)],
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

fn assert_never_contacted(normal_url_listener: &TcpListener) {
    let accept_attempt = normal_url_listener.accept();
    assert!(
        matches!(&accept_attempt, Err(accept_error) if accept_error.kind() == ErrorKind::WouldBlock),
        "expected no connection to the normal URL, got {accept_attempt:?}"
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
    assert_eq!(
        posted_payload,
        serde_json::json!({"data": ENCRYPTED_DEVICE_MESSAGE})
    );
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

#[test]
fn tor_route_without_an_onion_url_fails_before_contacting_the_normal_url() {
    let (normal_url_listener, normal_url_port) = listen_without_blocking();
    let normal_url = local_pin_server_url("http", normal_url_port);
    let exchange_attempt = HttpPinServer::through_tor(closed_local_address())
        .exchange(&pin_server_request(vec![normal_url.clone()], Vec::new()));
    assert_eq!(
        exchange_attempt,
        Err(PinServerError::NoOnionUrl(vec![normal_url]))
    );
    assert_never_contacted(&normal_url_listener);
}

#[test]
fn tor_route_with_the_proxy_down_fails_without_contacting_the_normal_url() {
    let (normal_url_listener, normal_url_port) = listen_without_blocking();
    let tor_socks_proxy_address = closed_local_address();
    let exchange_attempt =
        HttpPinServer::through_tor(tor_socks_proxy_address).exchange(&pin_server_request(
            vec![
                local_pin_server_url("http", normal_url_port),
                ONION_PIN_SERVER_URL.to_owned(),
            ],
            Vec::new(),
        ));
    assert_eq!(
        exchange_attempt,
        Err(PinServerError::TorProxyUnreachable(tor_socks_proxy_address))
    );
    assert_never_contacted(&normal_url_listener);
}
