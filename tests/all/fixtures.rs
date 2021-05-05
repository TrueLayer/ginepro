use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{HasPrivate, PKey, PKeyRef, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::{X509Builder, X509Name, X509Ref, X509};

/// A test utility to get HTTPS working properly in tests.
///
/// It fiddles with OpenSSL so that you do not have to.
pub struct TestTlsFixture {
    ca_private_key: PKey<Private>,
    ca_certificate: X509,
    server_private_key: PKey<Private>,
    server_certificate: X509,
}

impl TestTlsFixture {
    /// Generate a certificate authority, a certificate signing request and a
    /// server certificate signed from the generated CA.
    pub fn generate() -> Self {
        let ca_private_key = PKey::from_rsa(Rsa::generate(4096).unwrap()).unwrap();
        let ca_certificate = generate_ca(&ca_private_key);
        let server_private_key = PKey::from_rsa(Rsa::generate(4096).unwrap()).unwrap();
        let server_certificate = generate_server_certificate(
            &server_private_key,
            &ca_certificate,
            &ca_private_key,
            MessageDigest::sha256(),
        );
        Self {
            ca_private_key,
            ca_certificate,
            server_private_key,
            server_certificate,
        }
    }

    /// The CA private key used to sign the server certificate.
    pub fn ca_private_key(&self) -> &PKey<Private> {
        &self.ca_private_key
    }

    /// The X509 CA certificate.
    pub fn ca_certificate(&self) -> &X509 {
        &self.ca_certificate
    }

    /// The server private key.
    pub fn server_private_key(&self) -> &PKey<Private> {
        &self.server_private_key
    }

    /// The X509 server certificate.
    pub fn server_certificate(&self) -> &X509 {
        &self.server_certificate
    }
}

/// Generate a Certificate Authority (CA) for testing purposes.
fn generate_ca<T: HasPrivate>(private_key: &PKeyRef<T>) -> X509 {
    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_text("O", "TrueLayerRoot").unwrap();
    name.append_entry_by_text("L", "London").unwrap();
    name.append_entry_by_text("ST", "England").unwrap();
    name.append_entry_by_text("C", "GB").unwrap();
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
        .critical()
        .crl_sign()
        .key_cert_sign()
        .key_encipherment()
        .build()
        .unwrap();
    builder.append_extension(key_usage).unwrap();
    let ext_key_usage = ExtendedKeyUsage::new()
        .client_auth()
        .server_auth()
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

    builder.sign(&private_key, MessageDigest::sha256()).unwrap();

    builder.build()
}

fn generate_server_certificate<Q: HasPrivate, T: HasPrivate>(
    server_private_key: &PKeyRef<Q>,
    ca_certificate: &X509Ref,
    ca_private_key: &PKeyRef<T>,
    hash: MessageDigest,
) -> X509 {
    let mut builder = X509Builder::new().unwrap();

    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_text("O", "TrueLayer").unwrap();
    name.append_entry_by_text("L", "London").unwrap();
    name.append_entry_by_text("ST", "England").unwrap();
    name.append_entry_by_text("C", "GB").unwrap();
    name.append_entry_by_nid(Nid::COMMONNAME, "localhost")
        .unwrap();
    let name = name.build();

    builder.set_version(2).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(365).unwrap())
        .unwrap();

    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).unwrap();
    builder
        .set_serial_number(&serial.to_asn1_integer().unwrap())
        .unwrap();

    builder.set_pubkey(&server_private_key).unwrap();

    // All the stuff that requires using the CA certificate information
    let key_usage = KeyUsage::new()
        .digital_signature()
        .critical()
        .crl_sign()
        .key_cert_sign()
        .key_encipherment()
        .build()
        .unwrap();
    builder.append_extension(key_usage).unwrap();
    let ext_key_usage = ExtendedKeyUsage::new().server_auth().build().unwrap();
    builder.append_extension(ext_key_usage).unwrap();
    let subject_key_identifier = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(Some(ca_certificate), None))
        .unwrap();
    builder.append_extension(subject_key_identifier).unwrap();
    let authority_key_identifier = AuthorityKeyIdentifier::new()
        .keyid(true)
        .build(&builder.x509v3_context(Some(ca_certificate), None))
        .unwrap();
    builder.append_extension(authority_key_identifier).unwrap();
    let subject_alternative_name = SubjectAlternativeName::new()
        .dns("invalid")
        .build(&builder.x509v3_context(Some(ca_certificate), None))
        .unwrap();
    builder.append_extension(subject_alternative_name).unwrap();

    builder
        .set_issuer_name(ca_certificate.subject_name())
        .unwrap();

    // Signing MUST be the last step, otherwise signature validation will fail
    builder.sign(ca_private_key, hash).unwrap();

    builder.build()
}
