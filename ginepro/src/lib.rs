//! `ginepro` offers an enriched tonic [`Channel`](tonic::transport::Channel) using a with pluggable service discovery
//! to periodcally update the active set of gRPC servers.
//!
//! # Example
//! Simple example.
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::{LoadBalancedChannelBuilder,LoadBalancedChannel};
//!     use shared_proto::pb::tester_client::TesterClient;
//!
//!     // Create a load balanced channel with the default lookup implementation.
//!     let load_balanced_channel =
//!     LoadBalancedChannelBuilder::new_with_service(("my_hostname", 5000)).await
//!                                     .expect("failed to read system conf")
//!                                     .channel();
//!
//!     let tester_client: TesterClient<LoadBalancedChannel> = TesterClient::new(load_balanced_channel);
//! }
//! ```
//! [`LoadBalancedChannel`] also allows pluggin in a different implementation of [`LookupService`].
//!
//! ```rust
//! use std::collections::HashSet;
//! use std::net::SocketAddr;
//! use ginepro::{LookupService, ServiceDefinition};
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
//!
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::{LoadBalancedChannelBuilder,LoadBalancedChannel};
//!     use shared_proto::pb::tester_client::TesterClient;
//!
//!     let load_balanced_channel =
//!     LoadBalancedChannelBuilder::new_with_service(("my_hostname", 5000)).await
//!                                     .expect("failed to read system conf")
//!                                     .lookup_service(DummyLookupService).channel();
//!
//!     let tester_client: TesterClient<LoadBalancedChannel> = TesterClient::new(load_balanced_channel);
//! }
//! ```
//! For systems with lower churn, the probe interval can be lowered.
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     use ginepro::{LoadBalancedChannelBuilder,LoadBalancedChannel};
//!     use shared_proto::pb::tester_client::TesterClient;
//!
//!     let load_balanced_channel =
//!     LoadBalancedChannelBuilder::new_with_service(("my_hostname", 5000)).await
//!                                     .expect("failed to read system conf")
//!                                     .dns_probe_interval(std::time::Duration::from_secs(3))
//!                                     .channel();
//!
//!     let tester_client: TesterClient<LoadBalancedChannel> = TesterClient::new(load_balanced_channel);
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
//!     use ginepro::{LoadBalancedChannelBuilder,LoadBalancedChannel};
//!     use shared_proto::pb::tester_client::TesterClient;
//!
//!     let load_balanced_channel =
//!     LoadBalancedChannelBuilder::new_with_service(("my_hostname", 5000)).await
//!                                     .expect("failed to read system conf")
//!                                     .timeout(std::time::Duration::from_secs(10))
//!                                     .channel();
//!
//!     let tester_client: TesterClient<LoadBalancedChannel> = TesterClient::new(load_balanced_channel);
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
