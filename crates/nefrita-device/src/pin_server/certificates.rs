use ureq::tls::{Certificate, PemItem};

use crate::pin_server::PinServerError;

pub(super) fn parse_root_certificates(
    root_certificates_pem: &[String],
) -> Result<Vec<Certificate<'static>>, PinServerError> {
    let mut root_certificates = Vec::new();
    for pem_text in root_certificates_pem {
        let certificate_count_before = root_certificates.len();
        for pem_item in ureq::tls::parse_pem(pem_text.as_bytes()) {
            match pem_item.map_err(|parsing_error| {
                PinServerError::InvalidCertificate(parsing_error.to_string())
            })? {
                PemItem::Certificate(root_certificate) => root_certificates.push(root_certificate),
                _ => {
                    return Err(PinServerError::InvalidCertificate(
                        "the PEM text holds something other than certificates".to_owned(),
                    ));
                }
            }
        }
        if root_certificates.len() == certificate_count_before {
            return Err(PinServerError::InvalidCertificate(
                "the PEM text holds no certificate".to_owned(),
            ));
        }
    }
    Ok(root_certificates)
}

#[cfg(test)]
mod tests;
