use ginepro::LoadBalancedChannelBuilder;

use shared_proto::pb::echo_client::EchoClient;
use shared_proto::pb::EchoRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = LoadBalancedChannelBuilder::new_with_service(("localhost", 5000 as u16))
        .await?
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
