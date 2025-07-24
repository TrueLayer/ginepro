use anyhow::Context;
use ginepro::LoadBalancedChannel;

use shared_proto::pb::{echo_client::EchoClient, EchoRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a load balanced channel that connects to localhost:5000.
    // Here the service discovery will update the set of servers every 5 seconds.
    let channel = LoadBalancedChannel::builder(("localhost", 5000_u16))
        .dns_probe_interval(std::time::Duration::from_secs(5))
        .channel()
        .await
        .context("failed to construct LoadBalancedChannel")?;

    // Use the channel created above to drive the communication in EchoClient.
    let mut client = EchoClient::new(channel);

    let request = tonic::Request::new(EchoRequest {
        message: "hello".into(),
    });

    let response = client.unary_echo(request).await?;

    println!("RESPONSE={response:?}");

    Ok(())
}
