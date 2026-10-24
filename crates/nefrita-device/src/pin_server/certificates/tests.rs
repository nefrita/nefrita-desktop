use rcgen::IsCa;

use super::*;
use crate::pin_server::test_certificates::{generate_test_key, self_signed_certificate};

#[test]
fn certificate_in_pem_is_parsed() {
    let (certificate, _) = self_signed_certificate("127.0.0.1", IsCa::NoCa);
    let root_certificates =
        parse_root_certificates(&[certificate.pem()]).expect("the certificate should parse");
    assert_eq!(root_certificates.len(), 1);
    assert_eq!(root_certificates[0].der(), certificate.der().as_ref());
}

#[test]
fn text_without_certificates_is_rejected() {
    let parsing_attempt = parse_root_certificates(&["not a certificate".to_owned()]);
    assert!(matches!(
        parsing_attempt,
        Err(PinServerError::InvalidCertificate(_))
    ));
}

#[test]
fn private_key_instead_of_a_certificate_is_rejected() {
    let parsing_attempt = parse_root_certificates(&[generate_test_key().serialize_pem()]);
    assert!(matches!(
        parsing_attempt,
        Err(PinServerError::InvalidCertificate(_))
    ));
}
