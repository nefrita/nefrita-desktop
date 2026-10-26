use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinServerError {
    NoUsableUrl(Vec<String>),
    InvalidUrl(String),
    InvalidCertificate(String),
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
            Self::InvalidUrl(invalid_url) => {
                write!(formatter, "the PIN server URL is not valid: {invalid_url}")
            }
            Self::InvalidCertificate(certificate_error_description) => write!(
                formatter,
                "the PIN server certificate is not valid: {certificate_error_description}"
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
