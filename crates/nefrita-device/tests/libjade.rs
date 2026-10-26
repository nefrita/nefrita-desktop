use std::env;
use std::fs;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use nefrita_device::{
    CustomPinServer, DebugWalletKind, Device, DeviceError, HttpPinServer, Network, PinServer,
    PinServerError, PinServerPayload, PinServerRequest, UnlockOutcome,
};

const LIBJADE_ADDRESS_VARIABLE: &str = "NEFRITA_LIBJADE_ADDRESS";
const CUSTOM_PIN_SERVER_URL_VARIABLE: &str = "NEFRITA_CUSTOM_PIN_SERVER_URL";
const CUSTOM_PIN_SERVER_PUBLIC_KEY_FILE_VARIABLE: &str =
    "NEFRITA_CUSTOM_PIN_SERVER_PUBLIC_KEY_FILE";
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const RETRY_INTERVAL: Duration = Duration::from_secs(1);
const UNLOCK_NETWORK: Network = Network::XbtRegtest;
const PUBLIC_TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const DEVELOPMENT_PIN_SERVER_URL: &str = "http://localhost:8096";
const UNREACHABLE_PIN_SERVER_URL: &str = "http://127.0.0.1:1";
const DEFAULT_PIN_TO_ENTER: u32 = 12345;
const WRONG_PIN_TO_ENTER: u32 = 111111;

#[derive(Default)]
struct RecordingHttpPinServer {
    requested_urls: Vec<String>,
}

impl PinServer for RecordingHttpPinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        self.requested_urls
            .extend(pin_server_request.urls.iter().cloned());
        HttpPinServer.exchange(pin_server_request)
    }
}

fn required_environment_variable(variable_name: &str, target_description: &str) -> String {
    env::var(variable_name)
        .unwrap_or_else(|_| panic!("{variable_name} should point to {target_description}"))
}

fn connect_to_libjade() -> Device<TcpStream> {
    let libjade_address =
        required_environment_variable(LIBJADE_ADDRESS_VARIABLE, "a running libjade_daemon");
    let startup_deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        let connection_attempt = Device::connect_tcp(libjade_address.as_str(), REPLY_TIMEOUT)
            .and_then(|mut device| device.request_version_information().map(|_| device));
        match connection_attempt {
            Ok(device) => return device,
            Err(connection_error) if Instant::now() >= startup_deadline => {
                panic!("libjade_daemon did not answer at {libjade_address}: {connection_error}")
            }
            Err(_) => thread::sleep(RETRY_INTERVAL),
        }
    }
}

fn connect_to_clean_libjade() -> Device<TcpStream> {
    let mut device = connect_to_libjade();
    device
        .debug_clean_reset()
        .expect("libjade should accept a clean reset");
    device
}

fn custom_pin_server_at(url: &str) -> CustomPinServer {
    let public_key_file = required_environment_variable(
        CUSTOM_PIN_SERVER_PUBLIC_KEY_FILE_VARIABLE,
        "the public key file of the custom PIN server",
    );
    let public_key_bytes = fs::read(&public_key_file)
        .unwrap_or_else(|read_error| panic!("{public_key_file} should be readable: {read_error}"));
    CustomPinServer {
        url: url.to_owned(),
        onion_url: None,
        public_key: public_key_bytes
            .try_into()
            .expect("the public key file should hold a compressed public key"),
        certificate: None,
    }
}

fn create_pin_wallet(device: &mut Device<TcpStream>, pin_server: &mut RecordingHttpPinServer) {
    device
        .debug_set_mnemonic(PUBLIC_TEST_MNEMONIC, DebugWalletKind::SavedWithPin)
        .expect("libjade should accept the test mnemonic");
    let unlock_outcome = device
        .unlock(UNLOCK_NETWORK, pin_server)
        .expect("the new PIN should be registered");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
}

#[test]
#[ignore = "needs libjade_daemon built with XBT, listening at NEFRITA_LIBJADE_ADDRESS"]
fn xbt_libjade_reports_the_xbt_chain() {
    let mut device = connect_to_libjade();
    let version_information = device
        .require_xbt_firmware()
        .expect("libjade_daemon should be an XBT build");
    assert_eq!(version_information.chain.as_deref(), Some("XBT"));
}

#[test]
#[ignore = "needs libjade_daemon built with XBT and the development PIN server"]
fn pin_wallet_is_created_and_unlocked_with_the_development_pin_server() {
    let mut device = connect_to_clean_libjade();
    let mut pin_server = RecordingHttpPinServer::default();
    create_pin_wallet(&mut device, &mut pin_server);
    device.logout().expect("libjade should log out");
    let unlock_outcome = device
        .unlock(UNLOCK_NETWORK, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    assert_eq!(
        pin_server.requested_urls,
        [
            format!("{DEVELOPMENT_PIN_SERVER_URL}/set_pin"),
            format!("{DEVELOPMENT_PIN_SERVER_URL}/get_pin"),
        ]
    );
}

#[test]
#[ignore = "needs libjade_daemon built with XBT and the development PIN server"]
fn wrong_pin_is_refused_and_the_right_pin_unlocks() {
    let mut device = connect_to_clean_libjade();
    let mut pin_server = RecordingHttpPinServer::default();
    create_pin_wallet(&mut device, &mut pin_server);
    device.logout().expect("libjade should log out");
    device
        .debug_set_pin_to_enter(WRONG_PIN_TO_ENTER)
        .expect("libjade should accept the wrong PIN to enter");
    let wrong_pin_outcome = device
        .unlock(UNLOCK_NETWORK, &mut pin_server)
        .expect("the unlock with the wrong PIN should finish");
    assert_eq!(wrong_pin_outcome, UnlockOutcome::Refused);
    device
        .debug_set_pin_to_enter(DEFAULT_PIN_TO_ENTER)
        .expect("libjade should accept the right PIN to enter");
    let right_pin_outcome = device
        .unlock(UNLOCK_NETWORK, &mut pin_server)
        .expect("the unlock with the right PIN should finish");
    assert_eq!(right_pin_outcome, UnlockOutcome::Unlocked);
}

#[test]
#[ignore = "needs libjade_daemon built with XBT and the custom PIN server"]
fn pin_wallet_is_created_and_unlocked_with_the_custom_pin_server() {
    let custom_pin_server_url =
        required_environment_variable(CUSTOM_PIN_SERVER_URL_VARIABLE, "the custom PIN server");
    let mut device = connect_to_clean_libjade();
    device
        .set_custom_pin_server(&custom_pin_server_at(&custom_pin_server_url))
        .expect("libjade should accept the custom PIN server");
    let mut pin_server = RecordingHttpPinServer::default();
    create_pin_wallet(&mut device, &mut pin_server);
    device.logout().expect("libjade should log out");
    let unlock_outcome = device
        .unlock(UNLOCK_NETWORK, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    assert_eq!(
        pin_server.requested_urls,
        [
            format!("{custom_pin_server_url}/set_pin"),
            format!("{custom_pin_server_url}/get_pin"),
        ]
    );
}

#[test]
#[ignore = "needs libjade_daemon built with XBT, listening at NEFRITA_LIBJADE_ADDRESS"]
fn temporary_wallet_unlocks_without_the_pin_server() {
    let mut device = connect_to_clean_libjade();
    device
        .debug_set_mnemonic(PUBLIC_TEST_MNEMONIC, DebugWalletKind::Temporary)
        .expect("libjade should accept the test mnemonic");
    let mut pin_server = RecordingHttpPinServer::default();
    let unlock_outcome = device
        .unlock(UNLOCK_NETWORK, &mut pin_server)
        .expect("the unlock should finish");
    assert_eq!(unlock_outcome, UnlockOutcome::Unlocked);
    assert!(pin_server.requested_urls.is_empty());
}

#[test]
#[ignore = "needs libjade_daemon built with XBT and the public key file of the custom PIN server"]
fn unreachable_custom_pin_server_fails_the_unlock() {
    let mut device = connect_to_clean_libjade();
    device
        .set_custom_pin_server(&custom_pin_server_at(UNREACHABLE_PIN_SERVER_URL))
        .expect("libjade should accept the custom PIN server");
    device
        .debug_set_mnemonic(PUBLIC_TEST_MNEMONIC, DebugWalletKind::SavedWithPin)
        .expect("libjade should accept the test mnemonic");
    let unlock_attempt = device.unlock(UNLOCK_NETWORK, &mut HttpPinServer);
    let Err(DeviceError::PinServerFailed(pin_server_error)) = unlock_attempt else {
        panic!("expected a failed PIN server exchange, got {unlock_attempt:?}");
    };
    assert!(
        matches!(pin_server_error, PinServerError::RequestFailed(_)),
        "expected a failed PIN server request, got {pin_server_error:?}"
    );
}
