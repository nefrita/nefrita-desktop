mod certificates;
mod error;
mod http;
#[cfg(test)]
mod test_certificates;
#[cfg(test)]
mod test_server;
mod url_choice;

use serde::{Deserialize, Serialize};

pub use error::PinServerError;
pub use http::HttpPinServer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinServerPayload {
    #[serde(rename = "data")]
    pub encrypted_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinServerRequest {
    pub urls: Vec<String>,
    pub root_certificates: Vec<String>,
    pub device_payload: PinServerPayload,
}

pub trait PinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError>;
}
