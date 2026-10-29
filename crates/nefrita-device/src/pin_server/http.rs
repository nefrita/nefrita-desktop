use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ureq::config::AutoHeaderValue;
use ureq::tls::{Certificate, RootCerts, TlsConfig, TlsProvider};
use ureq::{Agent, Proxy};

use crate::pin_server::certificates::parse_root_certificates;
use crate::pin_server::url_choice::choose_url;
use crate::pin_server::{
    PinServer, PinServerError, PinServerPayload, PinServerRequest, PinServerRoute,
};

const DIRECT_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);
const TOR_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(120);
const TOR_SOCKS_PROXY_SCHEME: &str = "socks5h";
const HTTP_OK_STATUS: u16 = 200;

#[derive(Debug)]
pub struct HttpPinServer {
    route: PinServerRoute,
}

impl HttpPinServer {
    pub fn direct() -> Self {
        Self {
            route: PinServerRoute::Direct,
        }
    }

    pub fn through_tor(tor_socks_proxy_address: SocketAddr) -> Self {
        Self {
            route: PinServerRoute::Tor {
                socks_proxy_address: tor_socks_proxy_address,
            },
        }
    }
}

impl PinServer for HttpPinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        let pin_server_url = choose_url(&pin_server_request.urls, self.route)?;
        let parsed_root_certificates =
            parse_root_certificates(&pin_server_request.root_certificates)?;
        let mut pin_server_response = pin_server_agent(self.route, parsed_root_certificates)?
            .post(pin_server_url)
            .send_json(&pin_server_request.device_payload)
            .map_err(|request_error| failed_request_error(self.route, request_error))?;
        let status_code = pin_server_response.status().as_u16();
        if status_code != HTTP_OK_STATUS {
            return Err(PinServerError::UnexpectedHttpStatus(status_code));
        }
        pin_server_response
            .body_mut()
            .read_json()
            .map_err(|decoding_error| PinServerError::MalformedReply(decoding_error.to_string()))
    }
}

fn pin_server_agent(
    route: PinServerRoute,
    root_certificates: Vec<Certificate<'static>>,
) -> Result<Agent, PinServerError> {
    let trusted_roots = if root_certificates.is_empty() {
        RootCerts::PlatformVerifier
    } else {
        RootCerts::from(root_certificates)
    };
    let tls_config = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(trusted_roots)
        .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .build();
    let (socks_proxy, exchange_timeout) = match route {
        PinServerRoute::Direct => (None, DIRECT_EXCHANGE_TIMEOUT),
        PinServerRoute::Tor {
            socks_proxy_address,
        } => (
            Some(tor_socks_proxy(socks_proxy_address)?),
            TOR_EXCHANGE_TIMEOUT,
        ),
    };
    let agent_config = Agent::config_builder()
        .tls_config(tls_config)
        .proxy(socks_proxy)
        .max_redirects(0)
        .http_status_as_error(false)
        .user_agent(AutoHeaderValue::None)
        .timeout_global(Some(exchange_timeout))
        .build();
    Ok(Agent::new_with_config(agent_config))
}

fn tor_socks_proxy(socks_proxy_address: SocketAddr) -> Result<Proxy, PinServerError> {
    Proxy::new(&format!("{TOR_SOCKS_PROXY_SCHEME}://{socks_proxy_address}")).map_err(
        |proxy_configuration_error| {
            PinServerError::RequestFailed(proxy_configuration_error.to_string())
        },
    )
}

fn failed_request_error(route: PinServerRoute, request_error: ureq::Error) -> PinServerError {
    match (route, request_error) {
        (
            PinServerRoute::Tor {
                socks_proxy_address,
            },
            ureq::Error::Io(connection_error),
        ) if connection_error.kind() == ErrorKind::ConnectionRefused => {
            PinServerError::TorProxyUnreachable(socks_proxy_address)
        }
        (_, request_error) => PinServerError::RequestFailed(request_error.to_string()),
    }
}

#[cfg(test)]
mod tests;
