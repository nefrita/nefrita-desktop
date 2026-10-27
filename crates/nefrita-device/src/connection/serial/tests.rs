use std::sync::{Mutex, PoisonError};
use std::thread;

use serde_json::Value;
use serialport::TTYPort;

use super::*;
use crate::device::scripted::version_information_reply;

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const SHORT_REPLY_TIMEOUT: Duration = Duration::from_millis(200);
const SIMULATED_DEVICE_READ_TIMEOUT: Duration = Duration::from_secs(5);
const MISSING_PORT_NAME: &str = "/dev/nefrita-missing-serial-port";

static PSEUDO_TERMINAL_PAIR_CREATION: Mutex<()> = Mutex::new(());

fn connection_over_pseudo_terminal(reply_timeout: Duration) -> (SerialConnection, TTYPort) {
    let (mut simulated_device_port, client_port) = {
        let _pair_creation_turn = PSEUDO_TERMINAL_PAIR_CREATION
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        TTYPort::pair().expect("the system should open a pseudo terminal pair")
    };
    simulated_device_port
        .set_timeout(SIMULATED_DEVICE_READ_TIMEOUT)
        .expect("the simulated device port should accept a timeout");
    let connection = SerialConnection::prepare_open_port(Box::new(client_port), reply_timeout)
        .expect("a pseudo terminal should become a serial connection");
    (connection, simulated_device_port)
}

fn read_client_request(simulated_device_port: &mut TTYPort) -> Value {
    let mut received_bytes = Vec::new();
    let mut read_chunk = [0; 256];
    loop {
        let mut decoder = minicbor::Decoder::new(&received_bytes);
        if decoder.skip().is_ok() {
            return minicbor_serde::from_slice(&received_bytes[..decoder.position()])
                .expect("the client request should decode");
        }
        let read_length = simulated_device_port
            .read(&mut read_chunk)
            .expect("the client request should arrive");
        received_bytes.extend_from_slice(&read_chunk[..read_length]);
    }
}

fn assert_timeouts(
    connection: &SerialConnection,
    expected_reply_timeout: Option<Duration>,
    expected_port_timeout: Duration,
) {
    assert_eq!(
        connection
            .reply_timeout()
            .expect("the reply timeout should be readable"),
        expected_reply_timeout
    );
    assert_eq!(connection.serial_port.timeout(), expected_port_timeout);
}

#[test]
fn pseudo_terminal_without_control_lines_becomes_a_connection_with_the_reply_timeout() {
    let (connection, _simulated_device_port) = connection_over_pseudo_terminal(REPLY_TIMEOUT);
    assert_timeouts(&connection, Some(REPLY_TIMEOUT), REPLY_TIMEOUT);
}

#[test]
fn request_and_reply_cross_the_serial_connection() {
    let (connection, mut simulated_device_port) = connection_over_pseudo_terminal(REPLY_TIMEOUT);
    let simulated_device = thread::spawn(move || {
        let client_request = read_client_request(&mut simulated_device_port);
        let request_identifier = client_request["id"]
            .as_str()
            .expect("the client request should have an identifier");
        simulated_device_port
            .write_all(&version_information_reply(request_identifier, Some("XBT")))
            .expect("the simulated device should send its reply");
        (client_request, simulated_device_port)
    });
    let version_information = Device::new(connection)
        .request_version_information()
        .expect("the reply should cross the serial connection");
    let (client_request, _simulated_device_port) = simulated_device
        .join()
        .expect("the simulated device should answer without panicking");
    assert_eq!(client_request["method"], "get_version_info");
    assert_eq!(version_information.chain.as_deref(), Some("XBT"));
}

#[test]
fn request_the_device_never_reads_still_times_out() {
    let (connection, _simulated_device_port) = connection_over_pseudo_terminal(SHORT_REPLY_TIMEOUT);
    let request_attempt = Device::new(connection).request_version_information();
    assert!(
        matches!(request_attempt, Err(DeviceError::ReplyTimedOut)),
        "expected the request to time out, got {request_attempt:?}"
    );
}

#[test]
fn removed_reply_timeout_uses_the_long_port_timeout_until_it_is_restored() {
    let (mut connection, _simulated_device_port) = connection_over_pseudo_terminal(REPLY_TIMEOUT);
    connection
        .set_reply_timeout(None)
        .expect("the reply timeout should be removable");
    assert_timeouts(&connection, None, PORT_TIMEOUT_WITHOUT_REPLY_TIMEOUT);
    connection
        .set_reply_timeout(Some(REPLY_TIMEOUT))
        .expect("the reply timeout should be restorable");
    assert_timeouts(&connection, Some(REPLY_TIMEOUT), REPLY_TIMEOUT);
}

#[test]
fn missing_serial_port_fails_the_connection() {
    let connection_attempt = Device::connect_serial(MISSING_PORT_NAME, REPLY_TIMEOUT);
    let Err(DeviceError::ConnectionFailed(_)) = connection_attempt else {
        panic!("expected the connection to {MISSING_PORT_NAME} to fail");
    };
}
