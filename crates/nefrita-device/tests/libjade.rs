use std::env;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use nefrita_device::Device;

const LIBJADE_ADDRESS_VARIABLE: &str = "NEFRITA_LIBJADE_ADDRESS";
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

fn connect_to_libjade() -> Device<TcpStream> {
    let libjade_address = env::var(LIBJADE_ADDRESS_VARIABLE).unwrap_or_else(|_| {
        panic!("{LIBJADE_ADDRESS_VARIABLE} should point to a running libjade_daemon")
    });
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

#[test]
#[ignore = "needs libjade_daemon built with XBT, listening at NEFRITA_LIBJADE_ADDRESS"]
fn xbt_libjade_reports_the_xbt_chain() {
    let mut device = connect_to_libjade();
    let version_information = device
        .require_xbt_firmware()
        .expect("libjade_daemon should be an XBT build");
    assert_eq!(version_information.chain.as_deref(), Some("XBT"));
}
