use std::error::Error;
use std::fmt;

use serde::{Serialize, Serializer};
use ureq::http::Uri;

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;
use crate::pin_server::{is_onion_host, parse_root_certificates};

const PIN_SERVER_PUBLIC_KEY_LENGTH: usize = 33;
const COMPRESSED_PUBLIC_KEY_PREFIXES: [u8; 2] = [0x02, 0x03];
const SUPPORTED_SCHEME_PREFIXES: [&str; 2] = ["http://", "https://"];
const FIRMWARE_URL_BUFFER_LENGTH: usize = 120;
const FIRMWARE_CERTIFICATE_BUFFER_LENGTH: usize = 2048;

const RESET_DETAILS_AND_CERTIFICATE: PinServerReset = PinServerReset {
    reset_details: true,
    reset_certificate: true,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomPinServer {
    #[serde(rename = "urlA")]
    pub url: String,
    #[serde(rename = "urlB", skip_serializing_if = "Option::is_none")]
    pub onion_url: Option<String>,
    #[serde(rename = "pubkey", serialize_with = "serialize_public_key_as_bytes")]
    pub public_key: [u8; PIN_SERVER_PUBLIC_KEY_LENGTH],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomPinServerCheckError {
    InvalidUrl(String),
    InvalidOnionUrl(String),
    InvalidPublicKey,
    InvalidCertificate(String),
}

#[derive(Serialize)]
struct PinServerReset {
    reset_details: bool,
    reset_certificate: bool,
}

impl CustomPinServer {
    pub fn check(&self) -> Result<(), CustomPinServerCheckError> {
        checked_url_host_name(&self.url).map_err(CustomPinServerCheckError::InvalidUrl)?;
        if let Some(onion_url) = &self.onion_url {
            let onion_host_name = checked_url_host_name(onion_url)
                .map_err(CustomPinServerCheckError::InvalidOnionUrl)?;
            if !is_onion_host(&onion_host_name) {
                return Err(CustomPinServerCheckError::InvalidOnionUrl(
                    "its host is not an onion address".to_owned(),
                ));
            }
        }
        let [public_key_prefix, ..] = self.public_key;
        if !COMPRESSED_PUBLIC_KEY_PREFIXES.contains(&public_key_prefix) {
            return Err(CustomPinServerCheckError::InvalidPublicKey);
        }
        if let Some(certificate) = &self.certificate {
            check_certificate(certificate)
                .map_err(CustomPinServerCheckError::InvalidCertificate)?;
        }
        Ok(())
    }
}

impl<Connection: DeviceConnection> Device<Connection> {
    pub fn set_custom_pin_server(
        &mut self,
        custom_pin_server: &CustomPinServer,
    ) -> Result<(), DeviceError> {
        custom_pin_server
            .check()
            .map_err(DeviceError::InvalidCustomPinServer)?;
        self.run_without_reply_timeout(|device| {
            device.send_request_requiring_confirmation("update_pinserver", Some(custom_pin_server))
        })
    }

    pub fn reset_pin_server(&mut self) -> Result<(), DeviceError> {
        self.run_without_reply_timeout(|device| {
            device.send_request_requiring_confirmation(
                "update_pinserver",
                Some(&RESET_DETAILS_AND_CERTIFICATE),
            )
        })
    }
}

impl fmt::Display for CustomPinServerCheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(url_error_description) => write!(
                formatter,
                "the PIN server URL is not valid: {url_error_description}"
            ),
            Self::InvalidOnionUrl(url_error_description) => write!(
                formatter,
                "the onion URL of the PIN server is not valid: {url_error_description}"
            ),
            Self::InvalidPublicKey => write!(
                formatter,
                "the PIN server public key is not a compressed public key"
            ),
            Self::InvalidCertificate(certificate_error_description) => write!(
                formatter,
                "the PIN server certificate is not valid: {certificate_error_description}"
            ),
        }
    }
}

impl Error for CustomPinServerCheckError {}

fn checked_url_host_name(url: &str) -> Result<String, String> {
    if url.len() >= FIRMWARE_URL_BUFFER_LENGTH {
        return Err(format!("it has {FIRMWARE_URL_BUFFER_LENGTH} bytes or more"));
    }
    let starts_with_supported_scheme = SUPPORTED_SCHEME_PREFIXES
        .iter()
        .any(|scheme_prefix| url.starts_with(scheme_prefix));
    if !starts_with_supported_scheme {
        return Err("it does not start with http:// or https://".to_owned());
    }
    if url.ends_with('/') {
        return Err("it ends with /".to_owned());
    }
    let Ok(parsed_url) = url.parse::<Uri>() else {
        return Err("it is not a valid URL".to_owned());
    };
    parsed_url
        .host()
        .map(str::to_owned)
        .ok_or_else(|| "it has no host".to_owned())
}

fn check_certificate(certificate: &str) -> Result<(), String> {
    if certificate.len() >= FIRMWARE_CERTIFICATE_BUFFER_LENGTH {
        return Err(format!(
            "it has {FIRMWARE_CERTIFICATE_BUFFER_LENGTH} bytes or more"
        ));
    }
    parse_root_certificates(&[certificate.to_owned()])
        .map(drop)
        .map_err(|_| "it is not PEM text holding only certificates".to_owned())
}

fn serialize_public_key_as_bytes<FieldSerializer: Serializer>(
    public_key: &[u8; PIN_SERVER_PUBLIC_KEY_LENGTH],
    serializer: FieldSerializer,
) -> Result<FieldSerializer::Ok, FieldSerializer::Error> {
    serializer.serialize_bytes(public_key)
}

#[cfg(test)]
mod tests;
