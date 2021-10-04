use crate::lookup::TestDnsResolver;
use crate::lookup::TesterImpl;
use ginepro::{LoadBalancedChannelBuilder, ServiceDefinition};
use shared_proto::pb::pong::Payload;
use shared_proto::pb::tester_client::TesterClient;
use shared_proto::pb::Ping;
use std::collections::HashSet;
use std::sync::Arc;
use tests::tls::{NoVerifier, TestSslCertificate};
use tokio::sync::Mutex;
use tonic::transport::ClientTlsConfig;
use tonic::transport::ServerTlsConfig;

fn get_payload_raw(payload: Payload) -> String {
    match payload {
        Payload::Raw(s) => s,
    }
}

#[tokio::test]
async fn load_balance_succeeds_with_churn() {
    // Steps:
    //  1. Create a server that is added to the list of endpoints.
    //  2. Do a gRPC call.
    //  3. Remove the server from the list of endpoints and shut it down.
    //  4. Repeat 1-3.
    // What we want to test:
    //  Clients function normally when servers are removed and added.

    // Arrange
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let sender = Arc::new(Mutex::new(sender));
    let mut resolver = TestDnsResolver::default();
    let probe_interval = tokio::time::Duration::from_millis(3);

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(
        ServiceDefinition::from_parts("test", 5000).unwrap(),
    )
    .await
    .expect("failed to init")
    .lookup_service(resolver.clone())
    .dns_probe_interval(probe_interval)
    .channel();
    let mut client = TesterClient::new(load_balanced_channel);

    let servers: Vec<String> = (0..10).into_iter().map(|s| s.to_string()).collect();
    let mut servers_called = Vec::new();

    // Act
    for server in &servers {
        resolver
            .add_server_with_provided_impl(
                server.to_string(),
                TesterImpl {
                    sender: Arc::clone(&sender),
                    name: server.to_string(),
                },
            )
            .await;
        // Give time to the DNS probe to run
        tokio::time::sleep(probe_interval * 3).await;

        let res = client
            .test(tonic::Request::new(Ping {}))
            .await
            .expect("failed to call server");
        let server = receiver.recv().await.expect("");
        assert_eq!(
            server,
            get_payload_raw(res.into_inner().payload.expect("no payload"))
        );
        servers_called.push(server.clone());
        resolver.remove_server(server).await;
        // Give time to the DNS probe to run
        tokio::time::sleep(probe_interval * 3).await;
    }

    // Assert
    assert_eq!(servers, servers_called);
}

#[tokio::test]
async fn load_balance_succeeds_with_churn_with_tls_enabled() {
    // Arrange
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let sender = Arc::new(Mutex::new(sender));

    let test_certificate = TestSslCertificate::generate();

    let ca: Vec<u8> = test_certificate.pem_certificate();

    let pkey = test_certificate.pem_private_key();

    let identity = tonic::transport::Identity::from_pem(&ca, &pkey);

    let server_config = ServerTlsConfig::new().identity(identity);

    let mut resolver = TestDnsResolver::new_with_tls(server_config);

    let mut rustls_client_config = rustls::ClientConfig::new();

    rustls_client_config.set_protocols(&["h2".to_string().as_bytes().to_vec()]);

    rustls_client_config
        .dangerous()
        .set_certificate_verifier(std::sync::Arc::new(NoVerifier {}));

    let config = ClientTlsConfig::new()
        .domain_name("test".to_string())
        .rustls_client_config(rustls_client_config);

    let probe_interval = tokio::time::Duration::from_millis(3);

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(
        ServiceDefinition::from_parts("test", 5000).unwrap(),
    )
    .await
    .expect("failed to init")
    .lookup_service(resolver.clone())
    .with_tls(config)
    .dns_probe_interval(probe_interval)
    .channel();
    let mut client = TesterClient::new(load_balanced_channel);

    let servers: Vec<String> = (0..10).into_iter().map(|s| s.to_string()).collect();
    let mut servers_called = Vec::new();

    // Act
    for server in &servers {
        resolver
            .add_server_with_provided_impl(
                server.to_string(),
                TesterImpl {
                    sender: Arc::clone(&sender),
                    name: server.to_string(),
                },
            )
            .await;

        // Give time to the DNS probe to run
        tokio::time::sleep(probe_interval * 3).await;

        let res = client
            .test(tonic::Request::new(Ping {}))
            .await
            .expect("failed to call server");
        let server = receiver.recv().await.expect("");
        assert_eq!(
            server,
            get_payload_raw(res.into_inner().payload.expect("no payload"))
        );
        servers_called.push(server.clone());
        resolver.remove_server(server).await;
    }

    // Assert
    assert_eq!(servers, servers_called);
}

#[tokio::test]
async fn load_balance_happy_path_scenario_calls_all_endpoints() {
    // Steps:
    //  1. Create 3 server that is added to the list of endpoints.
    //  2. Do 20 gRPC calls.
    //  3. Assert that all 3 servers have been called.
    // What we want to test:
    //  A common load balaning scenario in which you have more calls
    //  than servers, and you want all servers to be called.

    let num_calls = 20;
    let (sender, mut receiver) = tokio::sync::mpsc::channel(num_calls);
    let sender = Arc::new(Mutex::new(sender));
    let mut resolver = TestDnsResolver::default();

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(
        ServiceDefinition::from_parts("test", 5000).unwrap(),
    )
    .await
    .expect("failed to init")
    .lookup_service(resolver.clone())
    .dns_probe_interval(tokio::time::Duration::from_millis(3))
    .channel();
    let mut client = TesterClient::new(load_balanced_channel);

    resolver
        .add_server_with_provided_impl(
            "server_a".to_string(),
            TesterImpl {
                sender: Arc::clone(&sender),
                name: "server_a".to_string(),
            },
        )
        .await;
    resolver
        .add_server_with_provided_impl(
            "server_b".to_string(),
            TesterImpl {
                sender: Arc::clone(&sender),
                name: "server_b".to_string(),
            },
        )
        .await;
    resolver
        .add_server_with_provided_impl(
            "server_c".to_string(),
            TesterImpl {
                sender: Arc::clone(&sender),
                name: "server_c".to_string(),
            },
        )
        .await;

    let mut servers_called = HashSet::new();

    for _ in 0..num_calls {
        let res = client
            .test(tonic::Request::new(Ping {}))
            .await
            .expect("failed to call server");

        let server = receiver.recv().await.expect("");
        assert_eq!(
            server,
            get_payload_raw(res.into_inner().payload.expect("no payload"))
        );

        servers_called.insert(server);
    }

    assert_eq!(3, servers_called.len());
}

#[tokio::test]
async fn connection_timeout_is_not_fatal() {
    // Scenario:
    // The DNS probe returns an IP that we fail to connect to.
    // We want to ensure that our client keeps working as expected
    // as long as another good server comes up.
    // Steps:
    //   * Discover an IP without a backing server (`ghost_server`)
    //   * See the client call fail
    //   * Discover an IP with a backing server (`good_server`)
    //   * Wait for discovery update to happen in the probe task
    //   * See the client call succeed
    let (sender, mut receiver) = tokio::sync::mpsc::channel(10);
    let sender = Arc::new(Mutex::new(sender));
    let mut resolver = TestDnsResolver::default();
    let probe_interval = tokio::time::Duration::from_millis(3);

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(
        ServiceDefinition::from_parts("test", 5000).unwrap(),
    )
    .await
    .expect("failed to init")
    .lookup_service(resolver.clone())
    .timeout(tokio::time::Duration::from_millis(500))
    .dns_probe_interval(probe_interval)
    .channel();
    let mut client = TesterClient::new(load_balanced_channel);

    resolver
        .add_ip_without_server("ghost_server".into(), "127.0.0.124:5000".into())
        .await;
    client
        .test(tonic::Request::new(Ping {}))
        .await
        .expect_err("The call without a backing server should fail");
    resolver
        .remove_ip_and_not_server("ghost_server".into())
        .await;

    resolver
        .add_server_with_provided_impl(
            "good_server".to_string(),
            TesterImpl {
                sender: Arc::clone(&sender),
                name: "good_server".to_string(),
            },
        )
        .await;

    // Give time to the DNS probe to add the new good server
    tokio::time::sleep(probe_interval * 5).await;

    let res = client
        .test(tonic::Request::new(Ping {}))
        .await
        .expect("failed to call server");

    let server = receiver.recv().await.expect("");
    assert_eq!(
        server,
        get_payload_raw(res.into_inner().payload.expect("no payload"))
    );
}
