use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rcgen::KeyPair;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

pub(super) const TEST_SERVER_ADDRESS: &str = "127.0.0.1";

const HEAD_TERMINATOR: &[u8] = b"\r\n\r\n";
const TEST_SERVER_TIMEOUT: Duration = Duration::from_secs(10);
const READ_CHUNK_LENGTH: usize = 1024;

pub(super) struct ScriptedHttpResponse {
    pub(super) status_line: &'static str,
    pub(super) extra_headers: &'static str,
    pub(super) response_body: &'static str,
}

pub(super) struct CapturedHttpRequest {
    pub(super) request_head: String,
    pub(super) request_body: Vec<u8>,
}

pub(super) fn start_http_server(
    scripted_response: ScriptedHttpResponse,
) -> (u16, JoinHandle<io::Result<CapturedHttpRequest>>) {
    let (server_listener, server_port) = listen_on_local_port();
    let server_thread = thread::spawn(move || {
        let (client_stream, _) = server_listener.accept()?;
        client_stream.set_read_timeout(Some(TEST_SERVER_TIMEOUT))?;
        serve_one_request(client_stream, &scripted_response)
    });
    (server_port, server_thread)
}

pub(super) fn start_https_server(
    server_certificate: &rcgen::Certificate,
    server_key: &KeyPair,
    scripted_response: ScriptedHttpResponse,
) -> (u16, JoinHandle<()>) {
    let tls_server_config = ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the test server should support the default TLS versions")
    .with_no_client_auth()
    .with_single_cert(
        vec![server_certificate.der().clone()],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
    )
    .expect("the test server should load its certificate");
    let (server_listener, server_port) = listen_on_local_port();
    let server_thread = thread::spawn(move || {
        serve_one_https_request(server_listener, tls_server_config, &scripted_response).ok();
    });
    (server_port, server_thread)
}

fn serve_one_https_request(
    server_listener: TcpListener,
    tls_server_config: ServerConfig,
    scripted_response: &ScriptedHttpResponse,
) -> io::Result<CapturedHttpRequest> {
    let (tcp_stream, _) = server_listener.accept()?;
    tcp_stream.set_read_timeout(Some(TEST_SERVER_TIMEOUT))?;
    let server_connection =
        ServerConnection::new(Arc::new(tls_server_config)).map_err(io::Error::other)?;
    serve_one_request(
        StreamOwned::new(server_connection, tcp_stream),
        scripted_response,
    )
}

fn serve_one_request(
    mut client_stream: impl Read + Write,
    scripted_response: &ScriptedHttpResponse,
) -> io::Result<CapturedHttpRequest> {
    let mut received_bytes = Vec::new();
    let head_length = loop {
        if let Some(terminator_position) = received_bytes
            .windows(HEAD_TERMINATOR.len())
            .position(|received_window| received_window == HEAD_TERMINATOR)
        {
            break terminator_position + HEAD_TERMINATOR.len();
        }
        receive_more_bytes(&mut client_stream, &mut received_bytes)?;
    };
    let request_head = String::from_utf8_lossy(&received_bytes[..head_length]).into_owned();
    let body_length = declared_content_length(&request_head);
    while received_bytes.len() < head_length + body_length {
        receive_more_bytes(&mut client_stream, &mut received_bytes)?;
    }
    write!(
        client_stream,
        "{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
        scripted_response.status_line,
        scripted_response.response_body.len(),
        scripted_response.extra_headers,
        scripted_response.response_body
    )?;
    client_stream.flush()?;
    Ok(CapturedHttpRequest {
        request_head,
        request_body: received_bytes[head_length..head_length + body_length].to_vec(),
    })
}

fn receive_more_bytes(
    client_stream: &mut impl Read,
    received_bytes: &mut Vec<u8>,
) -> io::Result<()> {
    let mut read_chunk = [0; READ_CHUNK_LENGTH];
    let read_length = client_stream.read(&mut read_chunk)?;
    if read_length == 0 {
        return Err(io::Error::from(ErrorKind::UnexpectedEof));
    }
    received_bytes.extend_from_slice(&read_chunk[..read_length]);
    Ok(())
}

fn declared_content_length(request_head: &str) -> usize {
    request_head
        .lines()
        .find_map(|header_line| {
            let (header_name, header_value) = header_line.split_once(':')?;
            if header_name.trim().eq_ignore_ascii_case("content-length") {
                header_value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn listen_on_local_port() -> (TcpListener, u16) {
    let server_listener = TcpListener::bind((TEST_SERVER_ADDRESS, 0))
        .expect("the test server should listen on a local port");
    let server_port = server_listener
        .local_addr()
        .expect("the test server should have a local address")
        .port();
    (server_listener, server_port)
}
