use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;
use crate::pin_server::{PinServer, PinServerPayload, PinServerRequest};
use crate::protocol::{Network, WITHOUT_PARAMETERS};

const PIN_REPLY_METHOD_NAME: &str = "pin";
const HTTP_POST_METHOD: &str = "POST";
const JSON_CONTENT_TYPE: &str = "json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockOutcome {
    Unlocked,
    Refused,
}

#[derive(Serialize)]
struct AuthenticationParameters {
    network: Network,
    #[serde(rename = "epoch", skip_serializing_if = "Option::is_none")]
    unix_time_seconds: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AuthenticationStep {
    Finished(bool),
    PinServerRequested { http_request: DeviceHttpRequest },
}

#[derive(Deserialize)]
struct DeviceHttpRequest {
    #[serde(rename = "params")]
    http_parameters: DeviceHttpParameters,
    #[serde(rename = "on-reply")]
    reply_method_name: String,
}

#[derive(Deserialize)]
struct DeviceHttpParameters {
    #[serde(default)]
    urls: Vec<String>,
    #[serde(default)]
    root_certificates: Vec<String>,
    #[serde(rename = "method")]
    http_method: String,
    #[serde(rename = "accept")]
    accepted_content_type: String,
    #[serde(rename = "data")]
    device_payload: PinServerPayload,
}

impl<Connection: DeviceConnection> Device<Connection> {
    pub fn unlock(
        &mut self,
        network: Network,
        pin_server: &mut impl PinServer,
    ) -> Result<UnlockOutcome, DeviceError> {
        self.require_xbt_firmware()?;
        let normal_reply_timeout = self
            .connection
            .reply_timeout()
            .map_err(DeviceError::ConnectionFailed)?;
        self.connection
            .set_reply_timeout(None)
            .map_err(DeviceError::ConnectionFailed)?;
        let unlock_attempt = self.authenticate(network, pin_server);
        let reply_timeout_restoration = self.connection.set_reply_timeout(normal_reply_timeout);
        let unlock_outcome = unlock_attempt?;
        reply_timeout_restoration.map_err(DeviceError::ConnectionFailed)?;
        Ok(unlock_outcome)
    }

    pub fn logout(&mut self) -> Result<(), DeviceError> {
        self.send_request_requiring_confirmation("logout", WITHOUT_PARAMETERS)
    }

    fn authenticate(
        &mut self,
        network: Network,
        pin_server: &mut impl PinServer,
    ) -> Result<UnlockOutcome, DeviceError> {
        let authentication_parameters = AuthenticationParameters {
            network,
            unix_time_seconds: current_unix_time_seconds(),
        };
        let mut authentication_step =
            self.send_request("auth_user", Some(&authentication_parameters))?;
        loop {
            authentication_step = match authentication_step {
                AuthenticationStep::Finished(true) => return Ok(UnlockOutcome::Unlocked),
                AuthenticationStep::Finished(false) => return Ok(UnlockOutcome::Refused),
                AuthenticationStep::PinServerRequested { http_request } => {
                    self.forward_to_pin_server(http_request, pin_server)?
                }
            };
        }
    }

    fn forward_to_pin_server(
        &mut self,
        http_request: DeviceHttpRequest,
        pin_server: &mut impl PinServer,
    ) -> Result<AuthenticationStep, DeviceError> {
        if let Some(request_description) = unsupported_request_description(&http_request) {
            self.write_request("cancel", WITHOUT_PARAMETERS)?;
            return Err(DeviceError::UnsupportedPinServerRequest(
                request_description,
            ));
        }
        let http_parameters = http_request.http_parameters;
        let pin_server_request = PinServerRequest {
            urls: http_parameters.urls,
            root_certificates: http_parameters.root_certificates,
            device_payload: http_parameters.device_payload,
        };
        match pin_server.exchange(&pin_server_request) {
            Ok(server_reply_payload) => {
                self.send_request(PIN_REPLY_METHOD_NAME, Some(&server_reply_payload))
            }
            Err(pin_server_error) => {
                match self.send_request(PIN_REPLY_METHOD_NAME, WITHOUT_PARAMETERS) {
                    Err(DeviceError::Rejected(_)) => {
                        Err(DeviceError::PinServerFailed(pin_server_error))
                    }
                    next_authentication_step => next_authentication_step,
                }
            }
        }
    }
}

fn unsupported_request_description(http_request: &DeviceHttpRequest) -> Option<String> {
    let http_parameters = &http_request.http_parameters;
    if http_request.reply_method_name != PIN_REPLY_METHOD_NAME {
        Some(format!(
            "it expects the reply in {}",
            http_request.reply_method_name
        ))
    } else if http_parameters.http_method != HTTP_POST_METHOD {
        Some(format!(
            "it uses the HTTP method {}",
            http_parameters.http_method
        ))
    } else if http_parameters.accepted_content_type != JSON_CONTENT_TYPE {
        Some(format!(
            "it accepts {}",
            http_parameters.accepted_content_type
        ))
    } else {
        None
    }
}

fn current_unix_time_seconds() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time_since_unix_epoch| time_since_unix_epoch.as_secs())
}

#[cfg(test)]
mod tests;
