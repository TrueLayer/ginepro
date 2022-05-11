use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::{X509Name, X509};

/// A SSL certificate for test usage.
///
/// It helps generating all the bits and pieces required to spin up an API using HTTPS in a test
/// suite.
pub struct TestSslCertificate {
    private_key: Rsa<Private>,
    certificate: X509,
}

impl TestSslCertificate {
    /// Generate a new private key and SSL certificate.
    pub fn generate() -> Self {
        let private_key = Rsa::generate(4096).unwrap();
        let certificate = generate_ssl_certificate(&private_key);
        Self {
            private_key,
            certificate,
        }
    }

    /// Convert the X509 certificate to pem.
    pub fn pem_certificate(&self) -> Vec<u8> {
        self.certificate
            .clone()
            .to_pem()
            .expect("failed to convert to pem")
    }

    /// Convert private key to pem.
    pub fn pem_private_key(&self) -> Vec<u8> {
        self.private_key
            .clone()
            .private_key_to_pem()
            .expect("failed to convert to pem")
    }
}

/// Generate an SSL certificate for testing purposes.
fn generate_ssl_certificate(private_key: &Rsa<Private>) -> X509 {
    let private_key = PKey::from_rsa(private_key.to_owned()).unwrap();

    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_nid(Nid::COMMONNAME, "localhost")
        .unwrap();
    let name = name.build();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(365).unwrap())
        .unwrap();
    builder.set_pubkey(&private_key).unwrap();

    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).unwrap();
    builder
        .set_serial_number(&serial.to_asn1_integer().unwrap())
        .unwrap();

    let basic_constraints = BasicConstraints::new().critical().ca().build().unwrap();
    builder.append_extension(basic_constraints).unwrap();
    let key_usage = KeyUsage::new()
        .digital_signature()
        .key_encipherment()
        .build()
        .unwrap();
    builder.append_extension(key_usage).unwrap();
    let ext_key_usage = ExtendedKeyUsage::new()
        .client_auth()
        .server_auth()
        .other("2.999.1")
        .build()
        .unwrap();
    builder.append_extension(ext_key_usage).unwrap();
    let subject_key_identifier = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(None, None))
        .unwrap();
    builder.append_extension(subject_key_identifier).unwrap();
    let authority_key_identifier = AuthorityKeyIdentifier::new()
        .keyid(true)
        .build(&builder.x509v3_context(None, None))
        .unwrap();
    builder.append_extension(authority_key_identifier).unwrap();
    let subject_alternative_name = SubjectAlternativeName::new()
        .dns("localhost")
        .build(&builder.x509v3_context(None, None))
        .unwrap();
    builder.append_extension(subject_alternative_name).unwrap();

    builder.sign(&private_key, MessageDigest::sha256()).unwrap();

    builder.build()
}
