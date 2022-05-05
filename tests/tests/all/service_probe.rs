use crate::lookup::TestDnsResolver;
use crate::lookup::TesterImpl;
use ginepro::{LoadBalancedChannel, LoadBalancedChannelBuilder, LookupService, ServiceDefinition};
use shared_proto::pb::pong::Payload;
use shared_proto::pb::tester_client::TesterClient;
use shared_proto::pb::Ping;
use std::sync::Arc;
use std::{collections::HashSet, net::SocketAddr};
use std::{net::AddrParseError, time::Duration};
use tokio::sync::Mutex;

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

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(("test.com", 5000))
        .lookup_service(resolver.clone())
        .dns_probe_interval(probe_interval)
        .channel()
        .await
        .expect("failed to init");
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

// #[tokio::test]
// async fn load_balance_succeeds_with_churn_with_tls_enabled() {
//     // Arrange
//     let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
//     let sender = Arc::new(Mutex::new(sender));

//     let test_certificate = TestSslCertificate::generate();

//     let ca: Vec<u8> = test_certificate.pem_certificate();

//     let pkey = test_certificate.pem_private_key();

//     let identity = tonic::transport::Identity::from_pem(&ca, &pkey);

//     let server_config = ServerTlsConfig::new().identity(identity);

//     let mut resolver = TestDnsResolver::new_with_tls(server_config);

//     let mut roots = RootCertStore::empty();
//     let mut buf = std::io::BufReader::new(pkey.as_slice());
//     let certs = rustls_pemfile::certs(&mut buf).unwrap();
//     roots.add_parsable_certificates(&certs);

//     let tls = rustls::ClientConfig::builder()
//         .with_safe_defaults()
//         .with_root_certificates(roots)
//         .with_no_client_auth();

//     let mut http = HttpConnector::new();
//     http.enforce_http(false);

//     // We have to do some wrapping here to map the request type from
//     // `https://example.com` -> `https://[::1]:50051` because `rustls`
//     // doesn't accept ip's as `ServerName`.
//     let connector = tower::ServiceBuilder::new()
//         .layer_fn(move |s| {
//             let tls = tls.clone();

//             hyper_rustls::HttpsConnectorBuilder::new()
//                 .with_tls_config(tls)
//                 .https_or_http()
//                 .enable_http2()
//                 .wrap_connector(s)
//         })
//         // Since our cert is signed with `example.com` but we actually want to connect
//         // to a local server we will override the Uri passed from the `HttpsConnector`
//         // and map it to the correct `Uri` that will connect us directly to the local server.
//         .map_request(|_| Uri::from_static("https://[::1]:50051"))
//         .service(http);

//     let client = hyper::Client::builder().build(connector);

//     // Hyper expects an absolute `Uri` to allow it to know which server to connect too.
//     // Currently, tonic's generated code only sets the `path_and_query` section so we
//     // are going to write a custom tower layer in front of the hyper client to add the
//     // scheme and authority.
//     //
//     // Again, this Uri is `example.com` because our tls certs is signed with this SNI but above
//     // we actually map this back to `[::1]:50051` before the `Uri` is passed to hyper's `HttpConnector`
//     // to allow it to correctly establish the tcp connection to the local `tls-server`.
//     let uri = Uri::from_static("test.com");
//     let svc = tower::ServiceBuilder::new()
//         .map_request(move |mut req: http::Request<tonic::body::BoxBody>| {
//             let uri = Uri::builder()
//                 .scheme(uri.scheme().unwrap().clone())
//                 .authority(uri.authority().unwrap().clone())
//                 .path_and_query(req.uri().path_and_query().unwrap().clone())
//                 .build()
//                 .unwrap();

//             *req.uri_mut() = uri;
//             req
//         })
//         .service(client);

//     let probe_interval = tokio::time::Duration::from_millis(3);

//     let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(svc)
//         .lookup_service(resolver.clone())
//         .with_tls(config)
//         .dns_probe_interval(probe_interval)
//         .channel()
//         .await
//         .expect("failed to init");
//     let mut client = TesterClient::new(load_balanced_channel);

//     let servers: Vec<String> = (0..10i32).into_iter().map(|s| s.to_string()).collect();
//     let mut servers_called = Vec::new();

//     // Act
//     for server in &servers {
//         resolver
//             .add_server_with_provided_impl(
//                 server.to_string(),
//                 TesterImpl {
//                     sender: Arc::clone(&sender),
//                     name: server.to_string(),
//                 },
//             )
//             .await;

//         // Give time to the DNS probe to run
//         tokio::time::sleep(probe_interval * 3).await;

//         let res = client
//             .test(tonic::Request::new(Ping {}))
//             .await
//             .expect("failed to call server");
//         let server = receiver.recv().await.expect("");
//         assert_eq!(
//             server,
//             get_payload_raw(res.into_inner().payload.expect("no payload"))
//         );
//         servers_called.push(server.clone());
//         resolver.remove_server(server).await;
//     }

//     // Assert
//     assert_eq!(servers, servers_called);
// }

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

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(("test", 5000))
        .lookup_service(resolver.clone())
        .dns_probe_interval(tokio::time::Duration::from_millis(3))
        .channel()
        .await
        .expect("failed to init");
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

    let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(("test", 5000))
        .lookup_service(resolver.clone())
        .timeout(tokio::time::Duration::from_millis(500))
        .dns_probe_interval(probe_interval)
        .channel()
        .await
        .expect("failed to init");
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

#[tokio::test]
async fn builder_and_resolve_shall_fail_on_error() {
    struct FailResolve;
    #[async_trait::async_trait]
    impl LookupService for FailResolve {
        async fn resolve_service_endpoints(
            &self,
            _definition: &ServiceDefinition,
        ) -> Result<HashSet<SocketAddr>, anyhow::Error> {
            anyhow::bail!("could not reach dns")
        }
    }

    LoadBalancedChannel::builder(("www.test.com", 5000))
        .lookup_service(FailResolve)
        .timeout(tokio::time::Duration::from_millis(500))
        .resolution_strategy(ginepro::ResolutionStrategy::Eager {
            timeout: Duration::from_secs(20),
        })
        .channel()
        .await
        .unwrap_err();
}

#[tokio::test]
async fn builder_and_resolve_shall_succeed_when_ips_are_returned() {
    struct SucceedResolve;
    #[async_trait::async_trait]
    impl LookupService for SucceedResolve {
        async fn resolve_service_endpoints(
            &self,
            _definition: &ServiceDefinition,
        ) -> Result<HashSet<SocketAddr>, anyhow::Error> {
            Ok(vec!["127.0.0.1:8000".to_string()]
                .into_iter()
                .map(|s| s.parse::<SocketAddr>())
                .collect::<Result<HashSet<SocketAddr>, AddrParseError>>()?)
        }
    }

    assert!(
        LoadBalancedChannel::builder(ServiceDefinition::from_parts("test.com", 5000).unwrap(),)
            .lookup_service(SucceedResolve)
            .timeout(tokio::time::Duration::from_millis(500))
            .resolution_strategy(ginepro::ResolutionStrategy::Eager {
                timeout: Duration::from_secs(20),
            })
            .channel()
            .await
            .is_ok()
    );
}
