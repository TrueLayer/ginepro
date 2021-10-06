use ginepro::LoadBalancedChannel;

use anyhow::Context;

use shared_proto::pb::{echo_client::EchoClient, EchoRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // By using the constructor `build_and_resolve` the hostname is resolved once and ensures
    // that LoadBalancedChannel will have a non-empty set of IPs to contact before the program
    // starts.
    let channel = LoadBalancedChannel::builder(("localhost", 5000_u16))
        .resolve_eagerly(None)
        .channel()
        .await
        .context("failed to build LoadBalancedChannel")?;

    // Use the channel created above to drive the communication in EchoClient.
    let mut client = EchoClient::new(channel);

    let request = tonic::Request::new(EchoRequest {
        message: "hello".into(),
    });

    let response = client.unary_echo(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
