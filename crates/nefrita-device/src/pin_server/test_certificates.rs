use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};

pub(super) struct CertificateAuthority {
    pub(super) certificate: rcgen::Certificate,
    issuer: Issuer<'static, KeyPair>,
}

pub(super) fn generate_test_key() -> KeyPair {
    KeyPair::generate().expect("a test key should generate")
}

pub(super) fn self_signed_certificate(
    subject_alternative_name: &str,
    authority_flag: IsCa,
) -> (rcgen::Certificate, KeyPair) {
    let certificate_key = generate_test_key();
    let mut certificate_parameters =
        CertificateParams::new(vec![subject_alternative_name.to_owned()])
            .expect("the test certificate parameters should be valid");
    certificate_parameters.is_ca = authority_flag;
    let certificate = certificate_parameters
        .self_signed(&certificate_key)
        .expect("the test certificate should be signed");
    (certificate, certificate_key)
}

pub(super) fn certificate_authority(common_name: &str) -> CertificateAuthority {
    let authority_key = generate_test_key();
    let mut authority_parameters = CertificateParams::new(Vec::<String>::new())
        .expect("the test authority parameters should be valid");
    authority_parameters.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    authority_parameters
        .distinguished_name
        .push(DnType::CommonName, common_name);
    let certificate = authority_parameters
        .self_signed(&authority_key)
        .expect("the test authority should be signed");
    CertificateAuthority {
        certificate,
        issuer: Issuer::new(authority_parameters, authority_key),
    }
}

pub(super) fn certificate_signed_by(
    authority: &CertificateAuthority,
    subject_alternative_name: &str,
) -> (rcgen::Certificate, KeyPair) {
    let certificate_key = generate_test_key();
    let certificate = CertificateParams::new(vec![subject_alternative_name.to_owned()])
        .expect("the test certificate parameters should be valid")
        .signed_by(&certificate_key, &authority.issuer)
        .expect("the test certificate should be signed");
    (certificate, certificate_key)
}
