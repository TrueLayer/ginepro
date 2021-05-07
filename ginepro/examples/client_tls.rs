use ginepro::LoadBalancedChannelBuilder;
use tonic::transport::Certificate;
use tonic::transport::ClientTlsConfig;

use shared_proto::pb::echo_client::EchoClient;
use shared_proto::pb::EchoRequest;
use tests::tls::TestSslCertificate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pem = TestSslCertificate::generate().pem_certificate();
    let ca = Certificate::from_pem(pem);

    let tls = ClientTlsConfig::new().ca_certificate(ca);

    let channel = LoadBalancedChannelBuilder::new_with_service(("localhost", 5000 as u16))
        .await?
        .with_tls(tls)
        .dns_probe_interval(std::time::Duration::from_secs(5))
        .channel();

    let mut client = EchoClient::new(channel);

    let request = tonic::Request::new(EchoRequest {
        message: "hello".into(),
    });

    let response = client.unary_echo(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
