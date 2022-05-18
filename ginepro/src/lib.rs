//! `ginepro` offers an enriched tonic [`Channel`](tonic::transport::Channel) using a pluggable service discovery
//! to periodcally update the active set of `gRPC` servers.
//!
//! # Simple example
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::LoadBalancedChannel;
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use std::convert::TryInto;
//!
//!     // Create a load balanced channel with the default lookup implementation.
//!     let load_balanced_channel = LoadBalancedChannel::builder(("my.hostname", 5000))
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//!
//! [`LoadBalancedChannel`] also allows plugging in a different implementation of [`LookupService`].
//!
//! ```rust
//! use ginepro::{LookupService, ServiceDefinition};
//! use std::collections::HashSet;
//! use std::net::SocketAddr;
//!
//! // This does nothing
//! struct DummyLookupService;
//!
//! #[async_trait::async_trait]
//! impl LookupService for DummyLookupService {
//!     async fn resolve_service_endpoints(
//!         &self,
//!         _definition: &ServiceDefinition,
//!     ) -> Result<HashSet<SocketAddr>, anyhow::Error> {
//!         Ok(HashSet::new())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::LoadBalancedChannel;
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use std::convert::TryInto;
//!
//!     // Create a load balanced channel with the default lookup implementation.
//!     let load_balanced_channel = LoadBalancedChannel::builder(("my.hostname", 5000))
//!         .lookup_service(DummyLookupService)
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//! For systems with lower churn, the probe interval can be lowered.
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::{LoadBalancedChannel, LoadBalancedChannelBuilder};
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use std::convert::TryInto;
//!
//!     let load_balanced_channel = LoadBalancedChannelBuilder::new_with_service(("my.hostname", 5000))
//!         .dns_probe_interval(std::time::Duration::from_secs(3))
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//!
//! It's also possible to associate a timeout for every new endpoint that the
//! [`LoadBalancedChannel`] tries to connect to.
//! .
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::LoadBalancedChannel;
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use std::convert::TryInto;
//!
//!     let load_balanced_channel = LoadBalancedChannel::builder(("my.hostname", 5000))
//!         .timeout(std::time::Duration::from_secs(10))
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//!
//! It's also possible to eagerly resolve the service endpoints once before
//! [`LoadBalancedChannel`] is constructed.
//! .
//!
//! ```rust,no_run
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::{LoadBalancedChannel, ResolutionStrategy};
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use std::time::Duration;
//!     use std::convert::TryInto;
//!
//!     let load_balanced_channel = LoadBalancedChannel::builder(("my.hostname", 5000))
//!         .timeout(std::time::Duration::from_secs(10))
//!          .resolution_strategy(ginepro::ResolutionStrategy::Eager {
//!              timeout: Duration::from_secs(20),
//!          })
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//!
//! If needed, you can use the [`with_endpoint_layer`](LoadBalancedChannelBuilder::with_endpoint_layer)
//! method to add more configuration to the channel endpoints
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::LoadBalancedChannel;
//!     use shared_proto::pb::tester_client::TesterClient;
//!     use tonic::transport::Endpoint;
//!
//!     // Create a load balanced channel with the default lookup implementation and a custom User-Agent.
//!     let load_balanced_channel = LoadBalancedChannel::builder(("my.hostname", 5000))
//!         .with_endpoint_layer(|endpoint: Endpoint| endpoint.user_agent("my ginepro client").ok())
//!         .channel()
//!         .await
//!         .expect("failed to construct LoadBalancedChannel");
//!
//!     let tester_client = TesterClient::new(load_balanced_channel);
//! }
//! ```
//!
//! # Internals
//! The tonic [`Channel`](tonic::transport::Channel) exposes the function
//! [`balance_channel`](tonic::transport::Channel::balance_channel) which returnes a bounded channel through which
//! endpoint changes can be sent.
//! `ginepro` uses this message passing mechanism to report when servers are added and removed.

mod balanced_channel;
mod dns_resolver;
mod lookup_service;
mod service_definition;
mod service_probe;

pub use balanced_channel::*;
pub use dns_resolver::*;
pub use lookup_service::*;
pub use service_definition::*;
pub use service_probe::{EndpointMiddleware, EndpointMiddlewareIdentity, EndpointMiddlewareLayer};
