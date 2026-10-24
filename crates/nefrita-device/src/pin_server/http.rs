use std::sync::Arc;
use std::time::Duration;

use ureq::Agent;
use ureq::config::AutoHeaderValue;
use ureq::tls::{Certificate, RootCerts, TlsConfig, TlsProvider};

use crate::pin_server::certificates::parse_root_certificates;
use crate::pin_server::url_choice::choose_url;
use crate::pin_server::{PinServer, PinServerError, PinServerPayload, PinServerRequest};

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_OK_STATUS: u16 = 200;

#[derive(Debug)]
pub struct HttpPinServer;

impl PinServer for HttpPinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        let pin_server_url = choose_url(&pin_server_request.urls)?;
        let parsed_root_certificates =
            parse_root_certificates(&pin_server_request.root_certificates)?;
        let mut pin_server_response = pin_server_agent(parsed_root_certificates)
            .post(pin_server_url)
            .send_json(&pin_server_request.device_payload)
            .map_err(|request_error| PinServerError::RequestFailed(request_error.to_string()))?;
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

fn pin_server_agent(root_certificates: Vec<Certificate<'static>>) -> Agent {
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
    let agent_config = Agent::config_builder()
        .tls_config(tls_config)
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .user_agent(AutoHeaderValue::None)
        .timeout_global(Some(EXCHANGE_TIMEOUT))
        .build();
    Agent::new_with_config(agent_config)
}

#[cfg(test)]
mod tests;
