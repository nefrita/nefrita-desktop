use nefrita_device::CustomPinServer;
use serde::Deserialize;

use crate::pin_server_choice::{PinServerChoiceError, choice_error_from_check_error};

const HEX_RADIX: u32 = 16;
const HEX_DIGITS_PER_BYTE: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CustomPinServerDetails {
    url: String,
    onion_url: Option<String>,
    public_key_hex: String,
    certificate: Option<String>,
}

pub(super) fn checked_custom_pin_server(
    custom_pin_server_details: CustomPinServerDetails,
) -> Result<CustomPinServer, PinServerChoiceError> {
    let custom_pin_server = custom_pin_server_from_details(custom_pin_server_details)?;
    custom_pin_server
        .check()
        .map_err(|check_error| choice_error_from_check_error(&check_error))?;
    Ok(custom_pin_server)
}

fn custom_pin_server_from_details(
    custom_pin_server_details: CustomPinServerDetails,
) -> Result<CustomPinServer, PinServerChoiceError> {
    let public_key = public_key_bytes_from_hex(&custom_pin_server_details.public_key_hex)
        .and_then(|public_key_bytes| public_key_bytes.try_into().ok())
        .ok_or(PinServerChoiceError::InvalidPublicKey)?;
    Ok(CustomPinServer {
        url: custom_pin_server_details.url,
        onion_url: custom_pin_server_details.onion_url,
        public_key,
        certificate: custom_pin_server_details
            .certificate
            .as_deref()
            .map(normalized_certificate),
    })
}

fn public_key_bytes_from_hex(public_key_hex: &str) -> Option<Vec<u8>> {
    let digit_values = public_key_hex
        .trim()
        .chars()
        .map(|hex_digit| hex_digit.to_digit(HEX_RADIX))
        .collect::<Option<Vec<u32>>>()?;
    let (digit_value_pairs, unpaired_digit_values) =
        digit_values.as_chunks::<HEX_DIGITS_PER_BYTE>();
    if !unpaired_digit_values.is_empty() {
        return None;
    }
    digit_value_pairs
        .iter()
        .map(|&[high_digit_value, low_digit_value]| {
            u8::try_from(high_digit_value * HEX_RADIX + low_digit_value).ok()
        })
        .collect()
}

fn normalized_certificate(certificate: &str) -> String {
    let certificate_with_lf_endings = certificate.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed_certificate = certificate_with_lf_endings.trim();
    format!("{trimmed_certificate}\n")
}

#[cfg(test)]
mod tests;
