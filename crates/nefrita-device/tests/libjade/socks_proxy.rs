use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use ureq::http::Uri;

const LOOPBACK_ADDRESS: &str = "127.0.0.1";
const SOCKS_VERSION: u8 = 5;
const NO_AUTHENTICATION_METHOD: u8 = 0;
const NO_ACCEPTABLE_METHOD: u8 = 0xff;
const CONNECT_COMMAND: u8 = 1;
const DOMAIN_NAME_ADDRESS_TYPE: u8 = 3;
const IPV4_ADDRESS_TYPE: u8 = 1;
const RESERVED_BYTE: u8 = 0;
const SUCCEEDED_REPLY: u8 = 0;
const GENERAL_FAILURE_REPLY: u8 = 1;
const COMMAND_NOT_SUPPORTED_REPLY: u8 = 7;
const ADDRESS_TYPE_NOT_SUPPORTED_REPLY: u8 = 8;
const UNSPECIFIED_BOUND_ADDRESS_AND_PORT: [u8; 6] = [0; 6];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostAndPort {
    pub(super) host_name: String,
    pub(super) port: u16,
}

pub(super) struct TestSocksProxy {
    pub(super) address: SocketAddr,
    connect_targets: Arc<Mutex<Vec<HostAndPort>>>,
}

impl TestSocksProxy {
    pub(super) fn forwarding_to(pin_server_url: &str) -> Self {
        let forward_target = host_and_port_of(pin_server_url);
        let proxy_listener = TcpListener::bind((LOOPBACK_ADDRESS, 0))
            .expect("the test SOCKS proxy should listen on a local port");
        let address = proxy_listener
            .local_addr()
            .expect("the test SOCKS proxy should have a local address");
        let test_socks_proxy = Self {
            address,
            connect_targets: Arc::new(Mutex::new(Vec::new())),
        };
        let connect_targets = Arc::clone(&test_socks_proxy.connect_targets);
        thread::spawn(move || {
            for client_stream in proxy_listener.incoming().flatten() {
                let forward_target = forward_target.clone();
                let connect_targets = Arc::clone(&connect_targets);
                thread::spawn(move || {
                    serve_client(client_stream, &forward_target, &connect_targets).ok();
                });
            }
        });
        test_socks_proxy
    }

    pub(super) fn connect_targets(&self) -> Vec<HostAndPort> {
        self.connect_targets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

pub(super) fn closed_proxy_address() -> SocketAddr {
    let free_port_listener = TcpListener::bind((LOOPBACK_ADDRESS, 0))
        .expect("the free port listener should listen on a local port");
    free_port_listener
        .local_addr()
        .expect("the free port listener should have a local address")
}

fn host_and_port_of(pin_server_url: &str) -> HostAndPort {
    let parsed_url: Uri = pin_server_url
        .parse()
        .unwrap_or_else(|_| panic!("{pin_server_url} should be a valid URL"));
    HostAndPort {
        host_name: parsed_url
            .host()
            .unwrap_or_else(|| panic!("{pin_server_url} should have a host"))
            .to_owned(),
        port: parsed_url
            .port_u16()
            .unwrap_or_else(|| panic!("{pin_server_url} should have a port")),
    }
}

fn serve_client(
    mut client_stream: TcpStream,
    forward_target: &HostAndPort,
    connect_targets: &Mutex<Vec<HostAndPort>>,
) -> io::Result<()> {
    accept_without_authentication(&mut client_stream)?;
    let connect_target = read_connect_target(&mut client_stream)?;
    connect_targets
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(connect_target);
    let server_stream =
        match TcpStream::connect((forward_target.host_name.as_str(), forward_target.port)) {
            Ok(server_stream) => server_stream,
            Err(forwarding_error) => {
                send_reply(&mut client_stream, GENERAL_FAILURE_REPLY)?;
                return Err(forwarding_error);
            }
        };
    send_reply(&mut client_stream, SUCCEEDED_REPLY)?;
    relay_both_ways(client_stream, server_stream)
}

fn accept_without_authentication(client_stream: &mut TcpStream) -> io::Result<()> {
    let mut greeting_header = [0; 2];
    client_stream.read_exact(&mut greeting_header)?;
    let [socks_version, method_count] = greeting_header;
    let mut offered_methods = vec![0; usize::from(method_count)];
    client_stream.read_exact(&mut offered_methods)?;
    if socks_version != SOCKS_VERSION || !offered_methods.contains(&NO_AUTHENTICATION_METHOD) {
        client_stream.write_all(&[SOCKS_VERSION, NO_ACCEPTABLE_METHOD])?;
        return Err(io::Error::other(
            "the client offered no SOCKS5 method without authentication",
        ));
    }
    client_stream.write_all(&[SOCKS_VERSION, NO_AUTHENTICATION_METHOD])
}

fn read_connect_target(client_stream: &mut TcpStream) -> io::Result<HostAndPort> {
    let mut request_header = [0; 4];
    client_stream.read_exact(&mut request_header)?;
    let [_, command, _, address_type] = request_header;
    if command != CONNECT_COMMAND {
        send_reply(client_stream, COMMAND_NOT_SUPPORTED_REPLY)?;
        return Err(io::Error::other(
            "the client asked for a command other than CONNECT",
        ));
    }
    if address_type != DOMAIN_NAME_ADDRESS_TYPE {
        send_reply(client_stream, ADDRESS_TYPE_NOT_SUPPORTED_REPLY)?;
        return Err(io::Error::other(
            "the client sent an IP address instead of a host name",
        ));
    }
    let mut host_name_length = [0; 1];
    client_stream.read_exact(&mut host_name_length)?;
    let mut host_name_bytes = vec![0; usize::from(host_name_length[0])];
    client_stream.read_exact(&mut host_name_bytes)?;
    let mut port_bytes = [0; 2];
    client_stream.read_exact(&mut port_bytes)?;
    Ok(HostAndPort {
        host_name: String::from_utf8_lossy(&host_name_bytes).into_owned(),
        port: u16::from_be_bytes(port_bytes),
    })
}

fn send_reply(client_stream: &mut TcpStream, reply_code: u8) -> io::Result<()> {
    let mut reply_bytes = vec![SOCKS_VERSION, reply_code, RESERVED_BYTE, IPV4_ADDRESS_TYPE];
    reply_bytes.extend_from_slice(&UNSPECIFIED_BOUND_ADDRESS_AND_PORT);
    client_stream.write_all(&reply_bytes)
}

fn relay_both_ways(client_stream: TcpStream, server_stream: TcpStream) -> io::Result<()> {
    let mut client_reader = client_stream.try_clone()?;
    let mut server_writer = server_stream.try_clone()?;
    thread::spawn(move || {
        io::copy(&mut client_reader, &mut server_writer).ok();
        server_writer.shutdown(Shutdown::Write).ok();
    });
    let mut server_reader = server_stream;
    let mut client_writer = client_stream;
    io::copy(&mut server_reader, &mut client_writer)?;
    client_writer.shutdown(Shutdown::Write)
}
