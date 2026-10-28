use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinServerError {
    NoUsableUrl(Vec<String>),
    NoOnionUrl(Vec<String>),
    InvalidUrl(String),
    InvalidCertificate(String),
    TorProxyUnreachable(SocketAddr),
    RequestFailed(String),
    UnexpectedHttpStatus(u16),
    MalformedReply(String),
}

impl fmt::Display for PinServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUsableUrl(offered_urls) => write!(
                formatter,
                "the device offers no PIN server URL that works without Tor: {}",
                offered_urls.join(", ")
            ),
            Self::NoOnionUrl(offered_urls) => write!(
                formatter,
                "the device offers no onion PIN server URL to use through Tor: {}",
                offered_urls.join(", ")
            ),
            Self::InvalidUrl(invalid_url) => {
                write!(formatter, "the PIN server URL is not valid: {invalid_url}")
            }
            Self::InvalidCertificate(certificate_error_description) => write!(
                formatter,
                "the PIN server certificate is not valid: {certificate_error_description}"
            ),
            Self::TorProxyUnreachable(socks_proxy_address) => write!(
                formatter,
                "the Tor SOCKS proxy does not answer at {socks_proxy_address}"
            ),
            Self::RequestFailed(request_error_description) => write!(
                formatter,
                "the request to the PIN server failed: {request_error_description}"
            ),
            Self::UnexpectedHttpStatus(status_code) => write!(
                formatter,
                "the PIN server answered with HTTP status {status_code}"
            ),
            Self::MalformedReply(decoding_error_description) => write!(
                formatter,
                "the PIN server sent a malformed reply: {decoding_error_description}"
            ),
        }
    }
}

impl Error for PinServerError {}
